// Rust
use alloc::{borrow::Cow, boxed::Box, format, string::String, vec::Vec};
use core::fmt::{Debug, Formatter, Result as FormatResult};

// 3rd-party
use anyhow::{anyhow, bail, ensure, Result};
use async_recursion::async_recursion;
use async_trait::async_trait;
use futures::{future, TryStreamExt};
use hashbrown::HashMap;
use rand::{rngs::StdRng, Rng, SeedableRng};

// IOTA
use crypto::keys::x25519;

// Streams
use lets::{
    address::{Address, AppAddr, MsgId},
    id::{Identifier, Identity, PermissionDuration, Permissioned, Psk, PskId},
    message::{
        ContentSizeof, ContentUnwrap, ContentWrap, Message as LetsMessage, PreparsedMessage, Topic, TransportMessage,
        HDF, PCF,
    },
    transport::Transport,
};
use spongos::{
    ddml::{
        commands::{sizeof, unwrap, wrap, Absorb, Commit, Mask, Squeeze},
        modifiers::External,
        types::{Mac, Maybe, NBytes, Size},
    },
    KeccakF1600, Spongos, SpongosRng,
};

// Local
use crate::{
    api::{
        cursor_store::{CursorStore, InnerCursorStore},
        message::Message,
        messages::Messages,
        send_response::SendResponse,
        user_builder::UserBuilder,
    },
    error::{Error, Result2},
    message::{
        announcement, branch_announcement, keyload, message_types, signed_packet, subscription, tagged_packet,
        unsubscription,
    },
};

const ANN_MESSAGE_NUM: usize = 0; // Announcement is always the first message of authors
const SUB_MESSAGE_NUM: usize = 0; // Subscription is always the first message of subscribers
const INIT_MESSAGE_NUM: usize = 1; // First non-reserved message number

#[derive(PartialEq, Eq, Default)]
struct State {
    /// Users' Identity information, contains keys and logic for signing and verification
    user_id: Option<Identity>,

    /// Address of the stream announcement message
    ///
    /// None if channel is not created or user is not subscribed.
    stream_address: Option<Address>,

    author_identifier: Option<Identifier>,

    /// Users' trusted public keys together with additional sequencing info: (msgid, seq_no) mapped
    /// by branch topic Vec.
    cursor_store: CursorStore,

    /// Mapping of trusted pre shared keys and identifiers
    psk_store: HashMap<PskId, Psk>,

    /// Mapping of exchange keys and identifiers
    exchange_keys: HashMap<Identifier, x25519::PublicKey>,

    spongos_store: HashMap<MsgId, Spongos>,

    base_branch: Topic,
}

pub struct User<T> {
    transport: T,

    state: State,
}

impl User<()> {
    pub fn builder() -> UserBuilder<()> {
        UserBuilder::new()
    }
}

impl<T> User<T> {
    pub(crate) fn new<Psks>(user_id: Option<Identity>, psks: Psks, transport: T) -> Self
    where
        Psks: IntoIterator<Item = (PskId, Psk)>,
    {
        let mut psk_store = HashMap::new();
        let mut exchange_keys = HashMap::new();

        // Store any pre shared keys
        psks.into_iter().for_each(|(pskid, psk)| {
            psk_store.insert(pskid, psk);
        });

        if let Some(id) = user_id.as_ref() {
            exchange_keys.insert(id.to_identifier(), id._ke_sk().public_key());
        }

        Self {
            transport,
            state: State {
                user_id,
                cursor_store: CursorStore::new(),
                psk_store,
                exchange_keys,
                spongos_store: Default::default(),
                stream_address: None,
                author_identifier: None,
                base_branch: Default::default(),
            },
        }
    }

    /// User's identifier
    pub fn identifier(&self) -> Option<Identifier> {
        self.identity().map(|id| id.to_identifier())
    }

    /// User Identity
    fn identity(&self) -> &Option<Identity> {
        &self.state.user_id
    }

    /// User's cursor
    fn cursor(&self, topic: &Topic) -> Option<usize> {
        self.identifier()
            .and_then(|id| self.state.cursor_store.get_cursor(topic, &id))
    }

    fn branch_mut(&mut self, topic: &Topic) -> Option<&mut InnerCursorStore> {
        self.state.cursor_store.branch_mut(topic)
    }

    fn branch(&self, topic: &Topic) -> Option<&InnerCursorStore> {
        self.state.cursor_store.branch(topic)
    }

    pub(crate) fn base_branch(&self) -> &Topic {
        &self.state.base_branch
    }

    pub(crate) fn stream_address(&self) -> Option<Address> {
        self.state.stream_address
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    pub fn topics(&self) -> impl Iterator<Item = &Topic> + ExactSizeIterator {
        self.state.cursor_store.topics()
    }

    pub(crate) fn cursors(&self) -> impl Iterator<Item = (&Topic, &Identifier, usize)> + '_ {
        self.state.cursor_store.cursors()
    }

    pub fn subscribers(&self) -> impl Iterator<Item = &Identifier> + Clone + '_ {
        self.state.exchange_keys.keys()
    }

    fn should_store_new_cursor(branch: &InnerCursorStore, subscriber: Permissioned<&Identifier>) -> bool {
        !subscriber.is_readonly() && !branch.contains_cursor(subscriber.identifier())
    }

    pub fn add_subscriber(&mut self, subscriber: Identifier) -> bool {
        let ke_pk = subscriber
            ._ke_pk()
            .expect("subscriber must have an identifier from which an x25519 public key can be derived");
        self.state.exchange_keys.insert(subscriber, ke_pk).is_none()
    }

    pub fn remove_subscriber(&mut self, id: &Identifier) -> bool {
        self.state.cursor_store.remove(id) | self.state.exchange_keys.remove(id).is_some()
    }

    pub fn add_psk(&mut self, psk: Psk) -> bool {
        self.state.psk_store.insert(psk.to_pskid(), psk).is_none()
    }

    pub fn remove_psk(&mut self, pskid: PskId) -> bool {
        self.state.psk_store.remove(&pskid).is_some()
    }

    fn get_latest_link(&self, topic: &Topic) -> Option<MsgId> {
        self.state.cursor_store.get_latest_link(topic)
    }

    pub(crate) async fn handle_message(&mut self, address: Address, msg: TransportMessage) -> Result2<Message> {
        let preparsed = msg
            .parse_header()
            .await
            .map_err(|e| Error::unwrapping("header of", address, e))?;
        match preparsed.header().message_type() {
            message_types::ANNOUNCEMENT => self.handle_announcement(address, preparsed).await,
            message_types::BRANCH_ANNOUNCEMENT => self.handle_branch_announcement(address, preparsed).await,
            message_types::SUBSCRIPTION => self.handle_subscription(address, preparsed).await,
            message_types::UNSUBSCRIPTION => self.handle_unsubscription(address, preparsed).await,
            message_types::KEYLOAD => self.handle_keyload(address, preparsed).await,
            message_types::SIGNED_PACKET => self.handle_signed_packet(address, preparsed).await,
            message_types::TAGGED_PACKET => self.handle_tagged_packet(address, preparsed).await,
            unknown => Err(Error::unexpected_message_type(address, unknown)),
        }
    }

    /// Bind Subscriber to the channel announced
    /// in the message.
    async fn handle_announcement(&mut self, address: Address, preparsed: PreparsedMessage) -> Result2<Message> {
        // Check Topic
        let topic = preparsed.header().topic().clone();
        let publisher = preparsed.header().publisher().clone();

        // Insert new branch into store
        let branch = self.state.cursor_store.new_branch(topic.clone());
        // From the point of view of cursor tracking, the message exists, regardless of the validity or
        // accessibility to its content. Therefore we must update the cursor of the publisher before
        // handling the message
        branch.set_cursor(publisher, INIT_MESSAGE_NUM);

        // Unwrap message
        let announcement = announcement::Unwrap::default();
        let (message, spongos) = preparsed
            .unwrap(announcement)
            .await
            .map_err(|e| Error::unwrapping("announcement", address, e))?;

        // Store spongos
        self.state.spongos_store.insert(address.relative(), spongos);

        // Store message content into stores
        let author_id = message.payload().content().author_id().clone();
        let author_ke_pk = *message.payload().content().author_ke_pk();

        // Update branch links
        branch.set_latest_link(address.relative());
        self.state.author_identifier = Some(author_id.clone());
        self.state.exchange_keys.insert(author_id, author_ke_pk);
        self.state.base_branch = topic;
        self.state.stream_address = Some(address);

        Ok(Message::from_lets_message(address, message))
    }

    async fn handle_branch_announcement(&mut self, address: Address, preparsed: PreparsedMessage) -> Result2<Message> {
        // Retrieve header values
        let prev_topic = preparsed.header().topic().clone();
        let publisher = preparsed.header().publisher().clone();
        let cursor = preparsed.header().sequence();

        let parent_branch = self.state.cursor_store.branch(&prev_topic).ok_or_else(
            || Error::no_parent_branch(address, )
        );
        // From the point of view of cursor tracking, the message exists, regardless of the validity or
        // accessibility to its content. Therefore we must update the cursor of the publisher before
        // handling the message
        self.state
            .cursor_store
            .insert_cursor(&prev_topic, publisher.clone(), cursor);

        // Unwrap message
        let linked_msg_address = preparsed
            .header()
            .linked_msg_address()
            .ok_or_else(|| Error::not_linked("branch-announcement", address))?;
        let mut linked_msg_spongos = {
            if let Some(spongos) = self.state.spongos_store.get(&linked_msg_address).copied() {
                // Spongos must be copied because wrapping mutates it
                spongos
            } else {
                return Ok(Message::orphan(address, preparsed));
            }
        };
        let branch_announcement = branch_announcement::Unwrap::new(&mut linked_msg_spongos);
        let (message, spongos) = preparsed
            .unwrap(branch_announcement)
            .await
            .map_err(|e| Error::unwrapping("branch-announcement", address, e))?;

        let new_topic = message.payload().content().new_topic();
        // Store spongos
        self.state.spongos_store.insert(address.relative(), spongos);
        // Insert new branch into store
        self.state.cursor_store.new_branch(new_topic.clone());
        // Collect permissions from previous branch and clone them into new branch
        let prev_permissions = self
            .cursors()
            .filter(|cursor| cursor.0 == &prev_topic)
            .map(|(_, id, _)| id.clone())
            .collect::<Vec<Identifier>>();
        for id in prev_permissions {
            self.state.cursor_store.insert_cursor(new_topic, id, INIT_MESSAGE_NUM);
        }

        // Update branch links
        self.set_latest_link(new_topic, address.relative());

        Ok(Message::from_lets_message(address, message))
    }

    async fn handle_subscription(&mut self, address: Address, preparsed: PreparsedMessage) -> Result2<Message> {
        // Cursor is not stored, as cursor is only tracked for subscribers with write permissions

        // Unwrap message
        let linked_msg_address = preparsed
            .header()
            .linked_msg_address()
            .ok_or_else(|| Error::not_linked("subscription", address))?;
        let mut linked_msg_spongos = {
            if let Some(spongos) = self.state.spongos_store.get(&linked_msg_address).copied() {
                // Spongos must be copied because wrapping mutates it
                spongos
            } else {
                return Ok(Message::orphan(address, preparsed));
            }
        };
        let user_ke_sk = &self
            .identity()
            .as_ref()
            .ok_or_else(|| Error::no_identity("read a subscription message"))?
            ._ke_sk();
        let subscription = subscription::Unwrap::new(&mut linked_msg_spongos, user_ke_sk);
        let (message, _spongos) = preparsed
            .unwrap(subscription)
            .await
            .map_err(|e| Error::unwrapping("subscription", address, e))?;

        // Store spongos
        // Subscription messages are never stored in spongos to maintain consistency about the view of the
        // set of messages of the stream between all the subscribers and across stateless recovers

        // Store message content into stores
        let subscriber_identifier = message.payload().content().subscriber_identifier();
        let subscriber_ke_pk = message.payload().content().subscriber_ke_pk();
        self.state
            .exchange_keys
            .insert(subscriber_identifier.clone(), subscriber_ke_pk);

        Ok(Message::from_lets_message(address, message))
    }

    async fn handle_unsubscription(&mut self, address: Address, preparsed: PreparsedMessage) -> Result2<Message> {
        // Cursor is not stored, as user is unsubscribing

        // Unwrap message
        let linked_msg_address = preparsed
            .header()
            .linked_msg_address()
            .ok_or_else(|| Error::not_linked("unsubscription", address))?;
        let mut linked_msg_spongos = {
            if let Some(spongos) = self.state.spongos_store.get(&linked_msg_address) {
                // Spongos must be cloned because wrapping mutates it
                *spongos
            } else {
                return Ok(Message::orphan(address, preparsed));
            }
        };
        let unsubscription = unsubscription::Unwrap::new(&mut linked_msg_spongos);
        let (message, spongos) = preparsed
            .unwrap(unsubscription)
            .await
            .map_err(|e| Error::unwrapping("unsubscription", address, e))?;

        // Store spongos
        self.state.spongos_store.insert(address.relative(), spongos);

        // Store message content into stores
        self.remove_subscriber(message.payload().content().subscriber_identifier());

        Ok(Message::from_lets_message(address, message))
    }

    async fn handle_keyload(&mut self, address: Address, preparsed: PreparsedMessage) -> Result2<Message> {
        let stream_address = self
            .stream_address()
            .ok_or_else(|| Error::no_stream("read a keyload"))?;

        // From the point of view of cursor tracking, the message exists, regardless of the validity or
        // accessibility to its content. Therefore we must update the cursor of the publisher before
        // handling the message
        self.state.cursor_store.insert_cursor(
            preparsed.header().topic(),
            preparsed.header().publisher().clone(),
            preparsed.header().sequence(),
        );

        // Unwrap message
        // Ok to unwrap since an author identifier is set at the same time as the stream address
        let author_identifier = self.state.author_identifier.as_ref().unwrap();
        let mut announcement_spongos = self
            .state
            .spongos_store
            .get(&stream_address.relative())
            .copied()
            .expect("a subscriber that has received an stream announcement must keep its spongos in store");

        // TODO: Remove Psk from Identity and Identifier, and manage it as a complementary permission
        let keyload = keyload::Unwrap::new(
            &mut announcement_spongos,
            self.state.user_id.as_ref(),
            author_identifier,
            &self.state.psk_store,
        );
        let (message, spongos) = preparsed
            .unwrap(keyload)
            .await
            .map_err(|e| Error::unwrapping("keyload", address, e))?;

        // Store spongos
        self.state.spongos_store.insert(address.relative(), spongos);

        // Store message content into stores
        for subscriber in message.payload().content().subscribers() {
            if Self::should_store_new_cursor(&branch, subscriber.as_ref()) {
                self.state.cursor_store.insert_cursor(
                    message.header().topic(),
                    subscriber.identifier().clone(),
                    INIT_MESSAGE_NUM,
                );
            }
        }

        // Have to make message before setting branch links due to immutable borrow in keyload::unwrap
        let final_message = Message::from_lets_message(address, message);
        // Update branch links
        self.set_latest_link(&final_message.header().topic, address.relative());
        Ok(final_message)
    }

    async fn handle_signed_packet(&mut self, address: Address, preparsed: PreparsedMessage) -> Result2<Message> {
        // From the point of view of cursor tracking, the message exists, regardless of the validity or
        // accessibility to its content. Therefore we must update the cursor of the publisher before
        // handling the message
        self.state.cursor_store.insert_cursor(
            preparsed.header().topic(),
            preparsed.header().publisher().clone(),
            preparsed.header().sequence(),
        );

        // Unwrap message
        let linked_msg_address = preparsed
            .header()
            .linked_msg_address()
            .ok_or_else(|| Error::not_linked("signed-packet", address))?;
        let mut linked_msg_spongos = {
            if let Some(spongos) = self.state.spongos_store.get(&linked_msg_address).copied() {
                // Spongos must be copied because wrapping mutates it
                spongos
            } else {
                // TODO: CONSIDER USING Error::
                return Ok(Message::orphan(address, preparsed));
            }
        };
        let signed_packet = signed_packet::Unwrap::new(&mut linked_msg_spongos);
        let (message, spongos) = preparsed
            .unwrap(signed_packet)
            .await
            .map_err(|e| Error::unwrapping("signed-packet", address, e))?;

        // Store spongos
        self.state.spongos_store.insert(address.relative(), spongos);

        // Store message content into stores
        self.set_latest_link(message.header().topic(), address.relative());
        Ok(Message::from_lets_message(address, message))
    }

    async fn handle_tagged_packet(&mut self, address: Address, preparsed: PreparsedMessage) -> Result2<Message> {
        // From the point of view of cursor tracking, the message exists, regardless of the validity or
        // accessibility to its content. Therefore we must update the cursor of the publisher before
        // handling the message
        self.state.cursor_store.insert_cursor(
            preparsed.header().topic(),
            preparsed.header().publisher().clone(),
            preparsed.header().sequence(),
        );

        // Unwrap message
        let linked_msg_address = preparsed
            .header()
            .linked_msg_address()
            .ok_or_else(|| Error::not_linked("tagged-packet", address))?;
        let mut linked_msg_spongos = {
            if let Some(spongos) = self.state.spongos_store.get(&linked_msg_address).copied() {
                // Spongos must be copied because wrapping mutates it
                spongos
            } else {
                return Ok(Message::orphan(address, preparsed));
            }
        };
        let tagged_packet = tagged_packet::Unwrap::new(&mut linked_msg_spongos);
        let (message, spongos) = preparsed
            .unwrap(tagged_packet)
            .await
            .map_err(|e| Error::unwrapping("tagged-packet", address, e))?;

        // Store spongos
        self.state.spongos_store.insert(address.relative(), spongos);

        // Store message content into stores
        self.set_latest_link(message.header().topic(), address.relative());

        Ok(Message::from_lets_message(address, message))
    }

    pub async fn backup<P>(&mut self, pwd: P) -> Result<Vec<u8>>
    where
        P: AsRef<[u8]>,
    {
        let mut ctx = sizeof::Context::new();
        ctx.sizeof(&self.state).await?;
        let buf_size = ctx.finalize() + 32; // State + Mac Size

        let mut buf = vec![0; buf_size];

        let mut ctx = wrap::Context::new(&mut buf[..]);
        let key: [u8; 32] = SpongosRng::<KeccakF1600>::new(pwd).gen();
        ctx.absorb(External::new(&NBytes::new(key)))?
            .commit()?
            .squeeze(&Mac::new(32))?;
        ctx.wrap(&mut self.state).await?;
        assert!(
            ctx.stream().is_empty(),
            "Missmatch between buffer size expected by SizeOf ({buf_size}) and actual size of Wrap ({})",
            ctx.stream().len()
        );

        Ok(buf)
    }

    pub async fn restore<B, P>(backup: B, pwd: P, transport: T) -> Result<Self>
    where
        P: AsRef<[u8]>,
        B: AsRef<[u8]>,
    {
        let mut ctx = unwrap::Context::new(backup.as_ref());
        let key: [u8; 32] = SpongosRng::<KeccakF1600>::new(pwd).gen();
        ctx.absorb(External::new(&NBytes::new(key)))?
            .commit()?
            .squeeze(&Mac::new(32))?;
        let mut state = State::default();
        ctx.unwrap(&mut state).await?;
        Ok(User { transport, state })
    }
}

impl<T> User<T>
where
    T: for<'a> Transport<'a, Msg = TransportMessage>,
{
    pub async fn receive_message(&mut self, address: Address) -> Result2<Message>
    where
        T: for<'a> Transport<'a, Msg = TransportMessage>,
    {
        let msg = self
            .transport
            .recv_message(address)
            .await
            .map_err(|e| Error::transport("receive_message", address, e))?;
        self.handle_message(address, msg).await
    }

    /// Start a [`Messages`] stream to traverse the channel messages
    ///
    /// See the documentation in [`Messages`] for more details and examples.
    pub fn messages(&mut self) -> Messages<T> {
        Messages::new(self)
    }

    /// Iteratively fetches all the next messages until internal state has caught up
    ///
    /// If succeeded, returns the number of messages advanced.
    pub async fn sync(&mut self) -> Result<usize> {
        // ignoring the result is sound as Drain::Error is Infallible
        self.messages().try_fold(0, |n, _| future::ok(n + 1)).await
    }

    /// Iteratively fetches all the pending messages from the transport
    ///
    /// Return a vector with all the messages collected. This is a convenience
    /// method around the [`Messages`] stream. Check out its docs for more
    /// advanced usages.
    pub async fn fetch_next_messages(&mut self) -> Result<Vec<Message>> {
        self.messages().try_collect().await
    }
}

impl<T, TSR> User<T>
where
    T: for<'a> Transport<'a, Msg = TransportMessage, SendResponse = TSR>,
{
    /// Prepare channel Announcement message.
    pub async fn create_stream<Top: Into<Topic>>(&mut self, topic: Top) -> Result2<SendResponse<TSR>> {
        // Confirm user has identity
        let identity = self
            .identity()
            .as_ref()
            .ok_or_else(|| Error::no_identity("create a stream"))?;
        let identifier = identity.to_identifier();
        // Convert topic
        let topic = topic.into();
        // Generate stream address
        let stream_base_address = AppAddr::gen(&identifier, &topic);
        let stream_rel_address = MsgId::gen(stream_base_address, &identifier, &topic, INIT_MESSAGE_NUM);
        let stream_address = Address::new(stream_base_address, stream_rel_address);

        // Prepare HDF and PCF
        let header = HDF::new(
            message_types::ANNOUNCEMENT,
            ANN_MESSAGE_NUM,
            identity.to_identifier(),
            topic.clone(),
        );

        let content = PCF::new_final_frame().with_content(announcement::Wrap::new(identity));

        // Wrap message
        let (transport_msg, spongos) = LetsMessage::new(header, content)
            .wrap()
            .await
            .map_err(|e| Error::wrapping("announcement", &topic, stream_address, e))?;

        // Attempt to send message
        if let Ok(message) = self.transport.recv_message(stream_address).await {
            if message == transport_msg {
                return Err(Error::topic_already_used(topic, stream_address));
            } else {
                return Err(Error::address_taken("announcement", stream_address));
            }
        }
        let send_response = self
            .transport
            .send_message(stream_address, transport_msg)
            .await
            .map_err(|e| Error::transport("announcement", stream_address, e))?;

        // If a message has been sent successfully, insert the base branch into store
        let branch = self.state.cursor_store.new_branch(topic.clone());
        // Commit message to stores
        branch.set_cursor(identifier.clone(), INIT_MESSAGE_NUM);
        self.state.spongos_store.insert(stream_address.relative(), spongos);

        // Update branch links
        branch.set_latest_link(stream_address.relative());

        // Commit Author Identifier and Stream Address to store
        self.state.stream_address = Some(stream_address);
        self.state.author_identifier = Some(identifier);
        self.state.base_branch = topic;

        Ok(SendResponse::new(stream_address, send_response))
    }

    #[async_recursion(?Send)]
    pub async fn new_branch(
        &mut self,
        from_topic: impl Into<Topic> + 'async_recursion,
        to_topic: impl Into<Topic> + 'async_recursion,
    ) -> Result2<SendResponse<TSR>> {
        let to_topic: Topic = to_topic.into();
        let from_topic: Topic = from_topic.into();

        // Check conditions
        let stream_address = self
            .stream_address()
            .ok_or_else(|| Error::no_stream("create a branch"))?;

        // let branch = match self.branch(&from_topic) {
        //     Some(branch) => branch,
        //     None => {
        //         self.new_branch(self.base_branch().clone(), from_topic.clone()).await?;
        //         self.branch(&from_topic)
        //             .expect("<from_topic> branch should exist, it was just created")
        //     }
        // };
        let branch = match self.branch_mut(&from_topic) {
            Some(branch) => branch,
            None => {
                self.new_branch(self.base_branch().clone(), from_topic.clone()).await?;
                self.branch_mut(&from_topic)
                    .expect("<from_topic> branch should exist, it was just created")
            }
        };

        let user_id = self
            .identity()
            .as_ref()
            .ok_or_else(|| Error::no_identity("create a branch"))?;
        let identifier = user_id.to_identifier();

        // Update own's cursor
        let current_cursor = branch.cursor(&identifier).ok_or_else(|| Error::no_cursor(&to_topic))?;
        let new_cursor = current_cursor.next();
        let msgid = MsgId::gen(stream_address.base(), &identifier, &from_topic, new_cursor);
        let address = Address::new(stream_address.base(), msgid);

        // Prepare HDF and PCF
        let link_to = branch.latest_link();

        // Spongos must be copied because wrapping mutates it
        let mut linked_msg_spongos = self.state.spongos_store.get(link_to).copied().ok_or_else(|| {
            Error::linked_not_in_store(
                "branch-announcement",
                &from_topic,
                address,
                Address::new(stream_address.base(), *link_to),
            )
        })?;
        let header = HDF::new(
            message_types::BRANCH_ANNOUNCEMENT,
            new_cursor,
            identifier.clone(),
            from_topic.clone(),
        )
        .with_linked_msg_address(link_to.clone());
        let content = PCF::new_final_frame().with_content(branch_announcement::Wrap::new(
            &mut linked_msg_spongos,
            user_id,
            &to_topic,
        ));

        // Wrap message
        let (transport_msg, spongos) = LetsMessage::new(header, content)
            .wrap()
            .await
            .map_err(|e| Error::wrapping("branch-announcement", &from_topic, address, e))?;
        let send_response = self
            .transport
            .send_message(address, transport_msg)
            .await
            .map_err(|e| Error::transport("new_branch", address, e))?;

        // If message has been sent successfully, create the new branch in store
        let new_branch = self.state.cursor_store.new_branch(to_topic.clone());
        // Commit message to stores and update cursors
        branch.set_cursor(identifier.clone(), new_cursor);
        self.state.spongos_store.insert(address.relative(), spongos);
        // Collect permissions from previous branch and clone them into new branch
        let prev_permissions = self
            .cursors()
            .filter(|cursor| cursor.0 == &from_topic)
            .map(|(_, id, _)| id.clone())
            .collect::<Vec<Identifier>>();
        for id in prev_permissions {
            new_branch.set_cursor(id, INIT_MESSAGE_NUM);
        }

        // TODO: REMOVE
        // Update branch links
        // self.branch_mut(&from_topic)
        //     .expect("at this point <from_topic> branch should had been created if it didn't exist")
        //     .set_latest_link(address.relative());
        branch.set_latest_link(address.relative());
        Ok(SendResponse::new(address, send_response))
    }

    /// Prepare Subscribe message.
    pub async fn subscribe(&mut self) -> Result2<SendResponse<TSR>> {
        // Check conditions
        let stream_address = self
            .stream_address()
            .ok_or_else(|| Error::no_stream("subscribe to a stream"))?;
        // Confirm user has identity
        let user_id = self
            .identity()
            .as_ref()
            .ok_or_else(|| Error::no_identity("subscribe to a stream"))?;
        let identifier = user_id.to_identifier();
        // Get base branch topic
        let base_branch = &self.state.base_branch;
        // Link message to channel announcement
        let link_to = stream_address.relative();
        let rel_address = MsgId::gen(stream_address.base(), &identifier, base_branch, SUB_MESSAGE_NUM);

        // Prepare HDF and PCF
        // Spongos must be copied because wrapping mutates it
        let mut linked_msg_spongos =
            self.state.spongos_store.get(&link_to).copied().expect(
                "a subscriber that has received an stream announcement should have its spongos always in store",
            );
        let unsubscribe_key = StdRng::from_entropy().gen();
        let author_ke_pk = self
            .state
            .author_identifier
            .as_ref()
            .and_then(|author_id| self.state.exchange_keys.get(author_id))
            .expect("a user that already have an stream address must know the author identifier");
        let content = PCF::new_final_frame().with_content(subscription::Wrap::new(
            &mut linked_msg_spongos,
            unsubscribe_key,
            user_id,
            author_ke_pk,
        ));
        let header = HDF::new(
            message_types::SUBSCRIPTION,
            SUB_MESSAGE_NUM,
            identifier.clone(),
            base_branch.clone(),
        )
        .with_linked_msg_address(link_to);

        let message_address = Address::new(stream_address.base(), rel_address);

        // Wrap message
        let (transport_msg, _spongos) = LetsMessage::new(header, content)
            .wrap()
            .await
            .map_err(|e| Error::wrapping("subscription", base_branch, message_address, e))?;

        // Attempt to send message
        let send_response = self
            .transport
            .send_message(message_address, transport_msg)
            .await
            .map_err(|e| Error::transport("subscribe", message_address, e))?;

        // If message has been sent successfully, commit message to stores
        // - Subscription messages are not stored in the cursor store
        // - Subscription messages are never stored in spongos to maintain consistency about the view of the
        // set of messages of the stream between all the subscribers and across stateless recovers
        Ok(SendResponse::new(message_address, send_response))
    }

    pub async fn unsubscribe(&mut self) -> Result2<SendResponse<TSR>> {
        // Check conditions
        let stream_address = self
            .stream_address()
            .ok_or_else(|| Error::no_stream("unsubscribe from a stream"))?;

        let base_branch = &self.state.base_branch;

        let branch = self
            .state
            .cursor_store
            .branch_mut(base_branch)
            .expect("base branch should always be available");
        let link_to = branch.latest_link();

        let user_id = self
            .state
            .user_id
            .as_ref()
            .ok_or_else(|| Error::no_identity("unsubscribe from a stream"))?;
        let identifier = user_id.to_identifier();

        // Update own's cursor
        let current_cursor = branch
            .cursor(&identifier)
            .ok_or_else(|| Error::no_cursor(base_branch))?;
        let new_cursor = current_cursor.next();
        let rel_address = MsgId::gen(stream_address.base(), &identifier, base_branch, new_cursor);
        let message_address = Address::new(stream_address.base(), rel_address);

        // Prepare HDF and PCF
        // Spongos must be copied because wrapping mutates it
        let mut linked_msg_spongos = self.state.spongos_store.get(&link_to).copied().ok_or_else(|| {
            Error::linked_not_in_store(
                "unsubscription",
                base_branch,
                message_address,
                Address::new(stream_address.base(), *link_to),
            )
        })?;
        let content = PCF::new_final_frame().with_content(unsubscription::Wrap::new(&mut linked_msg_spongos, user_id));
        let header = HDF::new(
            message_types::UNSUBSCRIPTION,
            new_cursor,
            identifier.clone(),
            base_branch.clone(),
        )
        .with_linked_msg_address(*link_to);

        // Wrap message
        let (transport_msg, spongos) = LetsMessage::new(header, content)
            .wrap()
            .await
            .map_err(|e| Error::wrapping("unsubscription", base_branch, message_address, e))?;

        // Attempt to send message
        let send_response = self
            .transport
            .send_message(message_address, transport_msg)
            .await
            .map_err(|e| Error::transport("unsubscribe", message_address, e))?;

        // If message has been sent successfully, commit message to stores
        branch.set_cursor(identifier, new_cursor);
        self.state.spongos_store.insert(rel_address, spongos);
        Ok(SendResponse::new(message_address, send_response))
    }

    pub async fn send_keyload<'a, Subscribers, Psks, Top>(
        &mut self,
        topic: Top,
        subscribers: Subscribers,
        psk_ids: Psks,
    ) -> Result2<SendResponse<TSR>>
    where
        Subscribers: IntoIterator<Item = Permissioned<&'a Identifier>>,
        Top: Into<Topic>,
        Psks: IntoIterator<Item = PskId>,
    {
        let stream_address = self
            .stream_address()
            .ok_or_else(|| Error::no_stream("send a keyload"))?;
        let topic = topic.into();
        let branch = match self.state.cursor_store.branch_mut(&topic) {
            Some(branch) => branch,
            None => {
                self.new_branch(self.base_branch().clone(), topic.clone()).await?;
                self.state
                    .cursor_store
                    .branch_mut(&topic)
                    .expect("<topic> branch should exist, it was just created")
            }
        };
        let user_id = self
            .state
            .user_id
            .as_ref()
            .ok_or_else(|| Error::no_identity("send a keyload"))?;
        let identifier = user_id.to_identifier();
        // Link message to edge of branch
        let link_to = branch.latest_link();
        // Update own's cursor
        let current_cursor = branch.cursor(&identifier).ok_or_else(|| Error::no_cursor(&topic))?;
        let new_cursor = current_cursor.next();
        let rel_address = MsgId::gen(stream_address.base(), &identifier, &topic, new_cursor);
        let message_address = Address::new(stream_address.base(), rel_address);

        // Prepare HDF and PCF
        // All Keyload messages will attach to stream Announcement message spongos
        let mut announcement_msg_spongos = self
            .state
            .spongos_store
            .get(&stream_address.relative())
            .copied()
            .expect("a subscriber that has received an stream announcement should have its spongos always in store");

        let mut rng = StdRng::from_entropy();
        let encryption_key = rng.gen();
        let nonce = rng.gen();
        let exchange_keys = &self.state.exchange_keys; // partial borrow to avoid borrowing the whole self within the closure
        let subscribers_with_keys = subscribers
            .into_iter()
            .flat_map(|subscriber| {
                Some((
                    subscriber,
                    exchange_keys.get(subscriber.identifier())?,
                    // identifier will encapsulate the key-exchange logic and ke storage will be removed from the
                    // user. No point in implementing error-handling for it
                ))
            })
            .collect::<Vec<(_, _)>>();
        let psk_store = &self.state.psk_store; // partial borrow outside closure (this wouldn't be necessary with 2021 edition)
        let psk_ids_with_psks = psk_ids
            .into_iter()
            .map(|pskid| {
                Ok((
                    pskid,
                    psk_store
                        .get(&pskid)
                        .ok_or_else(|| Error::unknown_psk(message_address, pskid))?,
                ))
            })
            .collect::<Result2<Vec<(_, _)>>>()?; // collect to handle possible error
        let content = PCF::new_final_frame().with_content(keyload::Wrap::new(
            &mut announcement_msg_spongos,
            subscribers_with_keys.iter().copied(),
            &psk_ids_with_psks,
            encryption_key,
            nonce,
            user_id,
        ));
        let header = HDF::new(message_types::KEYLOAD, new_cursor, identifier.clone(), topic.clone())
            .with_linked_msg_address(*link_to);

        // Wrap message
        let (transport_msg, spongos) = LetsMessage::new(header, content)
            .wrap()
            .await
            .map_err(|e| Error::wrapping("keyload", &topic, message_address, e))?;

        // Attempt to send message
        let send_response = self
            .transport
            .send_message(message_address, transport_msg)
            .await
            .map_err(|e| Error::transport("send_keyload", message_address, e))?;

        // If message has been sent successfully, commit message to stores
        for (subscriber, _) in subscribers_with_keys {
            if Self::should_store_new_cursor(&branch, subscriber) {
                branch.set_cursor(subscriber.to_identifier().clone(), INIT_MESSAGE_NUM);
            }
        }
        branch.set_cursor(identifier, new_cursor);
        self.state.spongos_store.insert(rel_address, spongos);
        // Update Branch Links
        branch.set_latest_link(message_address.relative());
        Ok(SendResponse::new(message_address, send_response))
    }

    pub async fn send_keyload_for_all<Top>(&mut self, topic: Top) -> Result2<SendResponse<TSR>>
    where
        Top: Into<Topic>,
    {
        let psks: Vec<PskId> = self.state.psk_store.keys().copied().collect();
        let subscribers: Vec<Permissioned<Identifier>> =
            self.subscribers().map(|s| Permissioned::Read(s.clone())).collect();
        self.send_keyload(
            topic,
            // Alas, must collect to release the &self immutable borrow
            subscribers.iter().map(Permissioned::as_ref),
            psks,
        )
        .await
    }

    pub async fn send_keyload_for_all_rw<Top>(&mut self, topic: Top) -> Result2<SendResponse<TSR>>
    where
        Top: Into<Topic>,
    {
        let psks: Vec<PskId> = self.state.psk_store.keys().copied().collect();
        let subscribers: Vec<Permissioned<Identifier>> = self
            .subscribers()
            .map(|s| Permissioned::ReadWrite(s.clone(), PermissionDuration::Perpetual))
            .collect();
        self.send_keyload(
            topic,
            // Alas, must collect to release the &self immutable borrow
            subscribers.iter().map(Permissioned::as_ref),
            psks,
        )
        .await
    }

    pub async fn send_signed_packet<P, M, Top>(
        &mut self,
        topic: Top,
        public_payload: P,
        masked_payload: M,
    ) -> Result2<SendResponse<TSR>>
    where
        M: AsRef<[u8]>,
        P: AsRef<[u8]>,
        Top: Into<Topic>,
    {
        let stream_address = self
            .stream_address()
            .ok_or_else(|| Error::no_stream("send a signed-packet"))?;
        let topic = topic.into();
        let branch = match self.state.cursor_store.branch_mut(&topic) {
            Some(branch) => branch,
            None => {
                self.new_branch(self.base_branch().clone(), topic.clone()).await?;
                self.state
                    .cursor_store
                    .branch_mut(&topic)
                    .expect("<topic> branch should exist, it was just created")
            }
        };
        let user_id = self
            .state
            .user_id
            .as_ref()
            .ok_or_else(|| Error::no_identity("send a signed-packet"))?;
        let identifier = user_id.to_identifier();
        // Link message to latest message in branch
        let link_to = branch.latest_link();
        // Update own's cursor
        let current_cursor = branch.cursor(&identifier).ok_or_else(|| Error::no_cursor(&topic))?;
        let new_cursor = current_cursor.next();
        let rel_address = MsgId::gen(stream_address.base(), &identifier, &topic, new_cursor);
        let message_address = Address::new(stream_address.base(), rel_address);

        // Prepare HDF and PCF
        // Spongos must be copied because wrapping mutates it
        let mut linked_msg_spongos = self.state.spongos_store.get(&link_to).copied().ok_or_else(|| {
            Error::linked_not_in_store(
                "signed-packet",
                &topic,
                message_address,
                Address::new(stream_address.base(), *link_to),
            )
        })?;
        let content = PCF::new_final_frame().with_content(signed_packet::Wrap::new(
            &mut linked_msg_spongos,
            user_id,
            public_payload.as_ref(),
            masked_payload.as_ref(),
        ));
        let header = HDF::new(
            message_types::SIGNED_PACKET,
            new_cursor,
            identifier.clone(),
            topic.clone(),
        )
        .with_linked_msg_address(*link_to);

        // Wrap message
        let (transport_msg, spongos) = LetsMessage::new(header, content)
            .wrap()
            .await
            .map_err(|e| Error::wrapping("signed-packet", &topic, message_address, e))?;

        // Attempt to send message
        let send_response = self
            .transport
            .send_message(message_address, transport_msg)
            .await
            .map_err(|e| Error::transport("send_signed_packet", message_address, e))?;

        // If message has been sent successfully, commit message to stores
        branch.set_cursor(identifier.clone(), new_cursor);
        branch.set_latest_link(message_address.relative());
        self.state.spongos_store.insert(rel_address, spongos);
        Ok(SendResponse::new(message_address, send_response))
    }

    pub async fn send_tagged_packet<P, M, Top>(
        &mut self,
        topic: Top,
        public_payload: P,
        masked_payload: M,
    ) -> Result2<SendResponse<TSR>>
    where
        M: AsRef<[u8]>,
        P: AsRef<[u8]>,
        Top: Into<Topic>,
    {
        // Check conditions
        let stream_address = self
            .stream_address()
            .ok_or_else(|| Error::no_stream("send a tagged-packet"))?;
        let topic = topic.into();
        let branch = match self.state.cursor_store.branch_mut(&topic) {
            Some(branch) => branch,
            None => {
                self.new_branch(self.base_branch().clone(), topic.clone()).await?;
                self.state
                    .cursor_store
                    .branch_mut(&topic)
                    .expect("<topic> branch should exist, it was just created")
            }
        };
        let user_id = self
            .state
            .user_id
            .as_ref()
            .ok_or_else(|| Error::no_identity("send a tagged-packet"))?;
        let identifier = user_id.to_identifier();
        // Check Topic
        let topic = topic.into();
        // Link message to latest message in branch
        let link_to = branch.latest_link();
        // Update own's cursor
        let current_cursor = branch.cursor(&identifier).ok_or_else(|| Error::no_cursor(&topic))?;
        let new_cursor = current_cursor.next();
        let rel_address = MsgId::gen(stream_address.base(), &identifier, &topic, new_cursor);
        let message_address = Address::new(stream_address.base(), rel_address);

        // Prepare HDF and PCF
        // Spongos must be copied because wrapping mutates it
        let mut linked_msg_spongos = self.state.spongos_store.get(&link_to).copied().ok_or_else(|| {
            Error::linked_not_in_store(
                "signed-packet",
                &topic,
                message_address,
                Address::new(stream_address.base(), *link_to),
            )
        })?;
        let content = PCF::new_final_frame().with_content(tagged_packet::Wrap::new(
            &mut linked_msg_spongos,
            public_payload.as_ref(),
            masked_payload.as_ref(),
        ));
        let header = HDF::new(
            message_types::TAGGED_PACKET,
            new_cursor,
            identifier.clone(),
            topic.clone(),
        )
        .with_linked_msg_address(*link_to);

        // Wrap message
        let (transport_msg, spongos) = LetsMessage::new(header, content)
            .wrap()
            .await
            .map_err(|e| Error::wrapping("tagged-packet", &topic, message_address, e))?;

        // Attempt to send message
        let send_response = self
            .transport
            .send_message(message_address, transport_msg)
            .await
            .map_err(|e| Error::transport("send_tagged_packet", message_address, e))?;

        // If message has been sent successfully, commit message to stores
        branch.set_cursor(identifier, new_cursor);
        branch.set_latest_link(rel_address);
        self.state.spongos_store.insert(rel_address, spongos);
        Ok(SendResponse::new(message_address, send_response))
    }
}

#[async_trait(?Send)]
impl ContentSizeof<State> for sizeof::Context {
    async fn sizeof(&mut self, user_state: &State) -> Result<&mut Self> {
        self.mask(Maybe::new(user_state.user_id.as_ref()))?
            .mask(Maybe::new(user_state.stream_address.as_ref()))?
            .mask(Maybe::new(user_state.author_identifier.as_ref()))?
            .mask(&user_state.base_branch)?;

        let amount_spongos = user_state.spongos_store.len();
        self.mask(Size::new(amount_spongos))?;
        for (address, spongos) in &user_state.spongos_store {
            self.mask(address)?.mask(spongos)?;
        }

        let topics = user_state.cursor_store.topics();
        let amount_topics = topics.len();
        self.mask(Size::new(amount_topics))?;

        for topic in topics {
            self.mask(topic)?;
            let latest_link = user_state
                .cursor_store
                .get_latest_link(topic)
                .ok_or_else(|| anyhow!("No latest link found in branch <{}>", topic))?;
            self.mask(&latest_link)?;

            let cursors: Vec<(&Topic, &Identifier, usize)> = user_state
                .cursor_store
                .cursors()
                .filter(|(t, _, _)| *t == topic)
                .collect();
            let amount_cursors = cursors.len();
            self.mask(Size::new(amount_cursors))?;
            for (_, subscriber, cursor) in cursors {
                self.mask(subscriber)?.mask(Size::new(cursor))?;
            }
        }

        let keys = &user_state.exchange_keys;
        let amount_keys = keys.len();
        self.mask(Size::new(amount_keys))?;
        for (subscriber, ke_pk) in keys {
            self.mask(subscriber)?.mask(ke_pk)?;
        }

        let psks = user_state.psk_store.iter();
        let amount_psks = psks.len();
        self.mask(Size::new(amount_psks))?;
        for (pskid, psk) in psks {
            self.mask(pskid)?.mask(psk)?;
        }

        self.commit()?.squeeze(Mac::new(32))?;
        Ok(self)
    }
}

#[async_trait(?Send)]
impl<'a> ContentWrap<State> for wrap::Context<&'a mut [u8]> {
    async fn wrap(&mut self, user_state: &mut State) -> Result<&mut Self> {
        self.mask(Maybe::new(user_state.user_id.as_ref()))?
            .mask(Maybe::new(user_state.stream_address.as_ref()))?
            .mask(Maybe::new(user_state.author_identifier.as_ref()))?
            .mask(&user_state.base_branch)?;

        let amount_spongos = user_state.spongos_store.len();
        self.mask(Size::new(amount_spongos))?;
        for (address, spongos) in &user_state.spongos_store {
            self.mask(address)?.mask(spongos)?;
        }

        let topics = user_state.cursor_store.topics();
        let amount_topics = topics.len();
        self.mask(Size::new(amount_topics))?;

        for topic in topics {
            self.mask(topic)?;
            let latest_link = user_state
                .cursor_store
                .get_latest_link(topic)
                .ok_or_else(|| anyhow!("No latest link found in branch <{}>", topic))?;
            self.mask(&latest_link)?;

            let cursors: Vec<(&Topic, &Identifier, usize)> = user_state
                .cursor_store
                .cursors()
                .filter(|(t, _, _)| *t == topic)
                .collect();
            let amount_cursors = cursors.len();
            self.mask(Size::new(amount_cursors))?;
            for (_, subscriber, cursor) in cursors {
                self.mask(subscriber)?.mask(Size::new(cursor))?;
            }
        }

        let keys = &user_state.exchange_keys;
        let amount_keys = keys.len();
        self.mask(Size::new(amount_keys))?;
        for (subscriber, ke_pk) in keys {
            self.mask(subscriber)?.mask(ke_pk)?;
        }

        let psks = user_state.psk_store.iter();
        let amount_psks = psks.len();
        self.mask(Size::new(amount_psks))?;
        for (pskid, psk) in psks {
            self.mask(pskid)?.mask(psk)?;
        }

        self.commit()?.squeeze(Mac::new(32))?;
        Ok(self)
    }
}

#[async_trait(?Send)]
impl<'a> ContentUnwrap<State> for unwrap::Context<&'a [u8]> {
    async fn unwrap(&mut self, user_state: &mut State) -> Result<&mut Self> {
        self.mask(Maybe::new(&mut user_state.user_id))?
            .mask(Maybe::new(&mut user_state.stream_address))?
            .mask(Maybe::new(&mut user_state.author_identifier))?
            .mask(&mut user_state.base_branch)?;

        let mut amount_spongos = Size::default();
        self.mask(&mut amount_spongos)?;
        for _ in 0..amount_spongos.inner() {
            let mut address = MsgId::default();
            let mut spongos = Spongos::default();
            self.mask(&mut address)?.mask(&mut spongos)?;
            user_state.spongos_store.insert(address, spongos);
        }

        let mut amount_topics = Size::default();
        self.mask(&mut amount_topics)?;

        for _ in 0..amount_topics.inner() {
            let mut topic = Topic::default();
            self.mask(&mut topic)?;
            let mut latest_link = MsgId::default();
            self.mask(&mut latest_link)?;

            let branch = user_state.cursor_store.new_branch(topic);
            branch.set_latest_link(latest_link);

            let mut amount_cursors = Size::default();
            self.mask(&mut amount_cursors)?;
            for _ in 0..amount_cursors.inner() {
                let mut subscriber = Identifier::default();
                let mut cursor = Size::default();
                self.mask(&mut subscriber)?.mask(&mut cursor)?;
                branch.set_cursor(subscriber, cursor.inner());
            }
        }

        let mut amount_keys = Size::default();
        self.mask(&mut amount_keys)?;
        for _ in 0..amount_keys.inner() {
            let mut subscriber = Identifier::default();
            let mut key = x25519::PublicKey::from_bytes([0; x25519::PUBLIC_KEY_LENGTH]);
            self.mask(&mut subscriber)?.mask(&mut key)?;
            user_state.exchange_keys.insert(subscriber, key);
        }

        let mut amount_psks = Size::default();
        self.mask(&mut amount_psks)?;
        for _ in 0..amount_psks.inner() {
            let mut pskid = PskId::default();
            let mut psk = Psk::default();
            self.mask(&mut pskid)?.mask(&mut psk)?;
            user_state.psk_store.insert(pskid, psk);
        }

        self.commit()?.squeeze(Mac::new(32))?;
        Ok(self)
    }
}

impl<T> Debug for User<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FormatResult {
        write!(
            f,
            "\n* identifier: <{}>\n* topic: {}\n{:?}\n* PSKs: \n{}\n* messages:\n{}\n",
            self.identifier().unwrap_or_default(),
            self.base_branch(),
            self.state.cursor_store,
            self.state
                .psk_store
                .keys()
                .map(|pskid| format!("\t<{:?}>\n", pskid))
                .collect::<String>(),
            self.state
                .spongos_store
                .keys()
                .map(|key| format!("\t<{}>\n", key))
                .collect::<String>()
        )
    }
}

/// An streams user equality is determined by the equality of its state. The major consequence of
/// this fact is that two users with the same identity but different transport configurations are
/// considered equal
impl<T> PartialEq for User<T> {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
    }
}

/// An streams user equality is determined by the equality of its state. The major consequence of
/// this fact is that two users with the same identity but different transport configurations are
/// considered equal
impl<T> Eq for User<T> {}

trait Cursor {
    fn next(self) -> Self;
}

impl Cursor for usize {
    fn next(self) -> Self {
        self + 1
    }
}
