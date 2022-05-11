use async_recursion::async_recursion;
use futures::{
    future,
    TryFutureExt,
    TryStreamExt,
};

use iota_streams_app::{
    identifier::Identifier,
    message::HasLink,
};
use iota_streams_core::{
    err,
    prelude::{
        Box,
        ToString,
        Vec,
    },
    prng,
    psk::{
        Psk,
        PskId,
    },
    try_or,
    unwrap_or_break,
    Errors::{
        ChannelDuplication,
        ChannelNotSingleDepth,
        NoPreviousMessage,
        UnknownMsgType,
        UserNotRegistered,
    },
    Result,
};

use super::*;
use crate::{
    api,
    message,
};

type UserImp = api::user::User<DefaultF, Address, LinkGen, LinkStore, KeyStore>;

const ENCODING: &str = "utf-8";
const PAYLOAD_LENGTH: usize = 32_000;

/// Baseline User api object. Contains the api user implementation as well as the transport object
pub struct User<Trans> {
    pub user: UserImp,
    pub transport: Trans,
}

impl<Trans> User<Trans> {
    /// Create a new User instance.
    ///
    /// # Arguments
    /// * `seed` - A string slice representing the seed of the user [Characters: A-Z, 9]
    /// * `channel_type` - Implementation type: [0: Single Branch, 1: Multi Branch , 2: Single Depth]
    /// * `transport` - Transport object used for sending and receiving
    pub fn new(seed: &str, channel_type: ChannelType, transport: Trans) -> Self {
        let nonce = "TANGLEUSERNONCE".as_bytes().to_vec();
        let user = UserImp::gen(
            prng::from_seed("IOTA Streams Channels user sig keypair", seed),
            nonce,
            channel_type,
            ENCODING.as_bytes().to_vec(),
            PAYLOAD_LENGTH,
        );
        Self { user, transport }
    }

    pub fn get_transport(&self) -> &Trans {
        &self.transport
    }

    // Attributes

    /// Fetch the Address (application instance) of the channel.
    pub fn channel_address(&self) -> Option<&ChannelAddress> {
        self.user.appinst.as_ref().map(|x| &x.appinst)
    }

    /// Fetch the Announcement Link of the channel.
    pub fn announcement_link(&self) -> &Option<TangleAddress> {
        &self.user.appinst
    }

    /// Channel Author's signature public key
    pub fn author_public_key(&self) -> Option<&ed25519::PublicKey> {
        self.user.author_public_key()
    }

    /// Return boolean representing the sequencing nature of the channel
    pub fn is_multi_branching(&self) -> bool {
        self.user.is_multi_branching()
    }

    /// Return boolean representing whether the implementation type is single depth
    pub fn is_single_depth(&self) -> bool {
        self.user.is_single_depth()
    }

    /// Fetch the user ed25519 public key
    pub fn get_public_key(&self) -> &PublicKey {
        &self.user.sig_kp.public
    }

    pub fn is_registered(&self) -> bool {
        self.user.appinst.is_some()
    }

    pub fn unregister(&mut self) {
        self.user.appinst = None;
        self.user.author_sig_pk = None;
    }

    // Utility

    /// Stores the provided link to the internal sequencing state for the provided participant
    /// [Used for multi-branching sequence state updates]
    /// [Author, Subscriber]
    ///
    ///   # Arguments
    ///   * `pk` - ed25519 Public Key of the sender of the message
    ///   * `link` - Address link to be stored in internal sequence state mapping
    pub fn store_state(&mut self, id: Identifier, link: &Address) -> Result<()> {
        // TODO: assert!(link.appinst == self.appinst.unwrap());
        self.user.store_state(id, link.msgid)
    }

    /// Stores the provided link and sequence number to the internal sequencing state for all participants
    /// [Used for single-branching sequence state updates]
    /// [Author, Subscriber]
    ///
    ///   # Arguments
    ///   * `link` - Address link to be stored in internal sequence state mapping
    ///   * `seq_num` - New sequence state to be stored in internal sequence state mapping
    pub fn store_state_for_all(&mut self, link: &Address, seq_num: u32) -> Result<()> {
        // TODO: assert!(link.appinst == self.appinst.unwrap());
        self.user.store_state_for_all(link.msgid, seq_num)
    }

    /// Fetches the latest PublicKey -> Cursor state mapping from the implementation, allowing the
    /// user to see the latest messages present from each publisher
    /// [Author, Subscriber]
    pub fn fetch_state(&self) -> Result<Vec<(Identifier, Cursor<Address>)>> {
        self.user.fetch_state()
    }

    /// Resets the cursor state storage to allow a User to retrieve all messages in a channel
    /// from scratch
    /// [Author, Subscriber]
    pub fn reset_state(&mut self) -> Result<()> {
        self.user.reset_state()
    }

    /// Generate the next batch of message [`Address`] to poll
    ///
    /// Given the set of users registered as participants of the channel and their current registered
    /// sequencing position, this method generates a set of new [`Address`] to poll for new messages
    /// (one for each user, represented by its [`Identifier`]). However, beware that it is not recommended to
    /// use this method as a means to implement message traversal, as there's no guarantee that the addresses
    /// returned are the immediately next addresses to be processed. use [`User::messages()`] instead.
    ///
    /// Keep in mind that in multi-branch channels, the link returned corresponds to the next sequence message.
    ///
    /// The link is returned in a [`Cursor<Link>`] to carry over its sequencing information
    pub fn gen_next_msg_addresses(&self) -> Vec<(Identifier, Cursor<Address>)> {
        self.user.gen_next_msg_links()
    }

    /// Commit to state a wrapped message and type
    /// [Author, Subscriber]
    ///
    ///  # Arguments
    ///  * `wrapped` - A wrapped message intended to be committed to the link store
    ///  * `info` - The type of wrapped message being committed to the link store
    pub fn commit_wrapped(&mut self, wrapped: WrapState, info: MsgInfo) -> Result<Address> {
        self.user.commit_wrapped(wrapped, info)
    }

    pub async fn export(&self, flag: u8, pwd: &str) -> Result<Vec<u8>> {
        self.user.export(flag, pwd).await
    }
    pub async fn import(bytes: &[u8], flag: u8, pwd: &str, tsp: Trans) -> Result<Self> {
        UserImp::import(bytes, flag, pwd).await.map(|u| Self {
            user: u,
            transport: tsp,
        })
    }

    /// Store a PSK in the user instance
    ///
    ///   # Arguments
    ///   * `pskid` - An identifier representing a pre shared key
    ///   * `psk` - A pre shared key
    pub fn store_psk(&mut self, pskid: PskId, psk: Psk, use_psk: bool) -> Result<()> {
        self.user.store_psk(pskid, psk, use_psk)
    }

    /// Remove a PSK from the user instance
    ///
    ///   # Arguments
    ///   * `pskid` - An identifier representing a pre shared key
    pub fn remove_psk(&mut self, pskid: PskId) -> Result<()> {
        self.user.remove_psk(pskid)
    }

    /// Store a predefined Subscriber by their public key
    ///
    ///   # Arguments
    ///   * `pk` - ed25519 public key of known subscriber
    pub fn store_new_subscriber(&mut self, pk: PublicKey) -> Result<()> {
        self.user.insert_subscriber(pk)
    }

    /// Remove a Subscriber from the user instance
    ///
    ///   # Arguments
    ///   * `pk` - ed25519 public key of known subscriber
    pub fn remove_subscriber(&mut self, pk: PublicKey) -> Result<()> {
        self.user.remove_subscriber(pk)
    }

    /// Consume a binary sequence message and return the derived message link
    async fn process_sequence(&mut self, msg: &BinaryMessage, store: bool) -> Result<Address> {
        let unwrapped = self.user.handle_sequence(msg, MsgInfo::Sequence, store).await?;
        let msg_cursor = self.user.gen_link(
            unwrapped.body.id,
            &unwrapped.body.ref_link,
            unwrapped.body.seq_num.0 as u32,
        );
        Ok(msg_cursor.link)
    }
}

impl<Trans: Transport + Clone> User<Trans> {
    /// Send a message with sequencing logic. If channel is single-branched, then no secondary
    /// sequence message is sent and None is returned for the address.
    ///
    /// # Arguments
    /// * `wrapped` - A wrapped sequence object containing the sequence message and state
    async fn send_sequence(&mut self, wrapped_sequence: WrappedSequence) -> Result<Option<Address>> {
        match wrapped_sequence {
            WrappedSequence::MultiBranch(
                cursor,
                WrappedMessage {
                    message,
                    wrapped: wrapped_state,
                },
            ) => {
                self.transport.send_message(&message).await?;
                self.user.commit_sequence(cursor, wrapped_state, MsgInfo::Sequence)
            }
            WrappedSequence::SingleBranch(cursor) => {
                self.user.commit_sequence_to_all(cursor)?;
                Ok(None)
            }
            WrappedSequence::SingleDepth(cursor) => {
                self.user.commit_sequence_to_all(cursor)?;
                Ok(None)
            }
            WrappedSequence::None => Ok(None),
        }
    }

    /// Send a message without using sequencing logic. Reserved for Announce and Subscribe messages
    async fn send_message(&mut self, msg: WrappedMessage, info: MsgInfo) -> Result<Address> {
        self.transport.send_message(&msg.message).await?;
        self.commit_wrapped(msg.wrapped, info)
    }

    /// Send a message using sequencing logic.
    ///
    /// # Arguments
    /// * `msg` - Wrapped Message ready for sending
    /// * `ref_link` - Reference link to be included in sequence message
    /// * `info` - Enum denominating the type of message being sent and committed
    async fn send_message_sequenced(
        &mut self,
        msg: WrappedMessage,
        ref_link: &MsgId,
        info: MsgInfo,
    ) -> Result<(Address, Option<Address>)> {
        // Send & commit original message
        self.transport.send_message(&msg.message).await?;
        let msg_link = self.commit_wrapped(msg.wrapped, info)?;

        // Send & commit associated sequence message
        let seq = self.user.wrap_sequence(ref_link).await?;
        let seq_link = self.send_sequence(seq).await?;
        Ok((msg_link, seq_link))
    }

    /// Send an announcement message, generating a channel [Author].
    pub async fn send_announce(&mut self) -> Result<Address> {
        let msg = self.user.announce().await?;
        try_or!(
            self.transport.recv_message(&msg.message.link).await.is_err(),
            ChannelDuplication
        )?;
        self.send_message(msg, MsgInfo::Announce).await
    }

    /// Create and send a signed packet [Author, Subscriber].
    ///
    ///  # Arguments
    ///  * `link_to` - Address of the message the keyload will be attached to
    ///  * `public_payload` - Wrapped vector of Bytes to have public access
    ///  * `masked_payload` - Wrapped vector of Bytes to have masked access
    pub async fn send_signed_packet(
        &mut self,
        link_to: &Address,
        public_payload: &Bytes,
        masked_payload: &Bytes,
    ) -> Result<(Address, Option<Address>)> {
        let msg = self.user.sign_packet(link_to, public_payload, masked_payload).await?;
        self.send_message_sequenced(msg, link_to.rel(), MsgInfo::SignedPacket)
            .await
    }

    /// Create and send a tagged packet [Author, Subscriber].
    ///
    ///  # Arguments
    ///  * `link_to` - Address of the message the keyload will be attached to
    ///  * `public_payload` - Wrapped vector of Bytes to have public access
    ///  * `masked_payload` - Wrapped vector of Bytes to have masked access
    pub async fn send_tagged_packet(
        &mut self,
        link_to: &Address,
        public_payload: &Bytes,
        masked_payload: &Bytes,
    ) -> Result<(Address, Option<Address>)> {
        let msg = self.user.tag_packet(link_to, public_payload, masked_payload).await?;
        self.send_message_sequenced(msg, link_to.rel(), MsgInfo::TaggedPacket)
            .await
    }

    /// Create and send a new keyload for a list of subscribers [Author].
    ///
    ///  # Arguments
    ///  * `link_to` - Address of the message the keyload will be attached to
    ///  * `keys`  - Iterable of [`Identifier`] to be included in message
    pub async fn send_keyload<'a, I>(&mut self, link_to: &Address, keys: I) -> Result<(Address, Option<Address>)>
    where
        I: IntoIterator<Item = &'a Identifier>,
    {
        let msg = self.user.share_keyload(link_to, keys).await?;
        self.send_message_sequenced(msg, link_to.rel(), MsgInfo::Keyload).await
    }

    /// Create and send keyload for all subscribed subscribers [Author].
    ///
    ///  # Arguments
    ///  * `link_to` - Address of the message the keyload will be attached to
    pub async fn send_keyload_for_everyone(&mut self, link_to: &Address) -> Result<(Address, Option<Address>)> {
        let msg = self.user.share_keyload_for_everyone(link_to).await?;
        self.send_message_sequenced(msg, link_to.rel(), MsgInfo::Keyload).await
    }

    /// Create and Send a Subscribe message to a Channel app instance [Subscriber].
    ///
    /// # Arguments
    /// * `link_to` - Address of the Channel Announcement message
    pub async fn send_subscribe(&mut self, link_to: &Address) -> Result<Address> {
        let msg = self.user.subscribe(link_to).await?;
        self.send_message(msg, MsgInfo::Subscribe).await
    }

    /// Create and Send an Unsubscribe message to a Channel app instance [Subscriber].
    ///
    /// # Arguments
    /// * `link_to` - Address of the user subscription message
    pub async fn send_unsubscribe(&mut self, link_to: &Address) -> Result<Address> {
        let msg = self.user.unsubscribe(link_to).await?;
        self.send_message(msg, MsgInfo::Unsubscribe).await
    }

    // Receive

    /// Receive and process a sequence message [Author, Subscriber].
    ///
    ///  # Arguments
    ///  * `link` - Address of the message to be processed
    pub async fn receive_sequence(&mut self, link: &Address) -> Result<Address> {
        let msg = self.transport.recv_message(link).await?;
        if let Some(_addr) = &self.user.appinst {
            let seq_msg = self.user.handle_sequence(&msg, MsgInfo::Sequence, true).await?.body;
            let msg_cursor = self
                .user
                .gen_link(seq_msg.id, &seq_msg.ref_link, seq_msg.seq_num.0 as u32);
            Ok(msg_cursor.link)
        } else {
            err!(UserNotRegistered)
        }
    }

    /// Receive and process a signed packet message [Author, Subscriber].
    ///
    ///  # Arguments
    ///  * `link` - Address of the message to be processed
    pub async fn receive_signed_packet(&mut self, link: &Address) -> Result<(PublicKey, Bytes, Bytes)> {
        let msg = self.transport.recv_message(link).await?;
        let m = self.user.handle_signed_packet(&msg, MsgInfo::SignedPacket).await?;
        Ok(m.body)
    }

    /// Receive and process a tagged packet message [Author, Subscriber].
    ///
    ///  # Arguments
    ///  * `link` - Address of the message to be processed
    pub async fn receive_tagged_packet(&mut self, link: &Address) -> Result<(Bytes, Bytes)> {
        let msg = self.transport.recv_message(link).await?;
        let m = self.user.handle_tagged_packet(&msg, MsgInfo::TaggedPacket).await?;
        Ok(m.body)
    }

    /// Receive and process a subscribe message [Author].
    ///
    ///  # Arguments
    ///  * `link` - Address of the message to be processed
    pub async fn receive_subscribe(&mut self, link: &Address) -> Result<()> {
        let msg = self.transport.recv_message(link).await?;
        self.user.handle_subscribe(&msg, MsgInfo::Subscribe).await
    }

    /// Receive and process an unsubscribe message [Author].
    ///
    ///  # Arguments
    ///  * `link` - Address of the message to be processed
    pub async fn receive_unsubscribe(&mut self, link: &Address) -> Result<()> {
        let msg = self.transport.recv_message(link).await?;
        self.user.handle_unsubscribe(msg, MsgInfo::Unsubscribe).await
    }

    /// Receive and Process an announcement message [Subscriber].
    ///
    /// # Arguments
    /// * `link_to` - Address of the Channel Announcement message
    pub async fn receive_announcement(&mut self, link: &Address) -> Result<()> {
        let msg = self.transport.recv_message(link).await?;
        self.user.handle_announcement(&msg, MsgInfo::Announce).await
    }

    /// Receive and process a keyload message [Subscriber].
    ///
    ///  # Argument&s
    ///  * `link` - Address of the message to be processed
    pub async fn receive_keyload(&mut self, link: &Address) -> Result<bool> {
        let msg = self.transport.recv_message(link).await?;
        let m = self.user.handle_keyload(&msg, MsgInfo::Keyload).await?;
        Ok(m.body)
    }

    /// Receive and process a message of unknown type. Message will be handled appropriately and
    /// the unwrapped contents returned [Author, Subscriber].
    ///
    ///   # Arguments
    ///   * `link` - Address of the message to be processed
    ///   * `pk` - Optional ed25519 Public Key of the sending participant. None if unknown
    pub async fn receive_message(&mut self, link: &Address) -> Result<UnwrappedMessage> {
        let msg = self.transport.recv_message(link).await?;
        self.handle_message(msg, true).await
    }

    /// Start a [`Messages`] stream to traverse the channel messages
    ///
    /// See the documentation in [`Messages`] for more details and examples.
    pub fn messages(&mut self) -> Messages<Trans> {
        IntoMessages::messages(self)
    }

    /// Iteratively fetches all the next messages until internal state has caught up
    ///
    /// If succeeded, returns the number of messages advanced.
    pub async fn sync_state(&mut self) -> Result<usize> {
        // ignoring the result is sound as Drain::Error is Infallible
        self.messages().try_fold(0, |n, _| future::ok(n + 1)).await
    }

    /// Iteratively fetches all the pending messages from the transport
    ///
    /// Return a vector with all the messages collected. This is a convenience
    /// method around the [`Messages`] stream. Check out its docs for more
    /// advanced usages.
    pub async fn fetch_next_msgs(&mut self) -> Result<Vec<UnwrappedMessage>> {
        self.messages().try_collect().await
    }

    /// Retrieves the previous message from the message specified (provided the user has access to it) [Author,
    /// Subscriber]
    ///
    /// # Arguments
    /// * `link` - Address of message to act as root of previous message fetching
    pub async fn fetch_prev_msg(&mut self, link: &Address) -> Result<UnwrappedMessage> {
        let msg = self.transport.recv_message(link).await?;
        let preparsed: Preparsed = msg.parse_header().await?;
        let header = preparsed.header;
        let prev_msg_link = Address::try_from_bytes(&header.previous_msg_link.0)
            .or_else(|_| err!(NoPreviousMessage(link.to_string())))?;
        let prev_msg = self.transport.recv_message(&prev_msg_link).await?;
        let unwrapped = self.handle_message(prev_msg, false).await?;
        Ok(unwrapped)
    }

    /// Retrieves a specified number of previous messages from an original specified messsage link [Author, Subscriber]
    /// # Arguments
    /// * `link` - Address of message to act as root of previous message fetching
    /// * `max` - The number of msgs to try and parse
    pub async fn fetch_prev_msgs(&mut self, link: &Address, max: usize) -> Result<Vec<UnwrappedMessage>> {
        let mut msg_info: (Address, u8, Message) = self.parse_msg_info(link).await?;
        let mut to_process = Vec::new();
        let mut msgs = Vec::new();

        for _ in 0..max {
            msg_info = unwrap_or_break!(self.parse_msg_info(&msg_info.0).await);
            if msg_info.1 == message::SEQUENCE {
                let msg_link = self.process_sequence(&msg_info.2, false).await?;
                msg_info = self.parse_msg_info(&msg_link).await?;
            }
            to_process.push(msg_info.2);
        }

        to_process.reverse();
        for msg in to_process {
            let unwrapped = self.handle_message(msg, false).await?;
            msgs.push(unwrapped);
        }

        Ok(msgs)
    }

    /// Handle message of unknown type. Ingests a message and unwraps it according to its determined
    /// content type [Author, Subscriber].
    ///
    /// # Arguments
    /// * `msg` - Binary message of unknown type
    #[async_recursion(?Send)]
    pub async fn handle_message<M>(&mut self, msg: M, store: bool) -> Result<UnwrappedMessage>
    where
        M: AsRef<BinaryMessage>,
    {
        let msg = msg.as_ref();
        let preparsed: Preparsed = msg.parse_header().await?;
        match preparsed.header.content_type {
            message::SIGNED_PACKET => Ok(self
                .user
                .handle_signed_packet(msg, MsgInfo::SignedPacket)
                .await?
                .map(|(pk, public, masked)| MessageContent::new_signed_packet(pk, public, masked))),
            message::TAGGED_PACKET => Ok(self
                .user
                .handle_tagged_packet(msg, MsgInfo::TaggedPacket)
                .await?
                .map(|(public, masked)| MessageContent::new_tagged_packet(public, masked))),
            message::KEYLOAD => {
                // So long as the unwrap has not failed, we will return a blank object to
                // inform the user that a message was present, even if the user wasn't part of
                // the keyload itself. This is to prevent sequencing failures
                Ok(self
                    .user
                    .handle_keyload(msg, MsgInfo::Keyload)
                    .await?
                    // TODO: Verify content, whether user is allowed or not
                    .map(|_allowed| MessageContent::new_keyload()))
            }
            message::SEQUENCE => {
                let msg_link = self.process_sequence(msg, store).await?;
                let sequenced_msg = self.transport.recv_message(&msg_link).await?;
                let unwrapped_msg = self.handle_message(&sequenced_msg, store).await;
                // future::ready artificiality is needed to be able to move sequence_msg into the async closure
                future::ready(unwrapped_msg)
                    .or_else(|_| async {
                        // As long as the sequence message is readable, we return Ok even if the referenced msg is not.
                        // This lets subscribers advance the cursor of the publisher even if they don't have access
                        // to this particular msg
                        let preparsed: Preparsed<'_> = sequenced_msg.parse_header().await?;
                        let link = preparsed.header.link;
                        let prev_link = TangleAddress::try_from_bytes(&preparsed.header.previous_msg_link.0)?;
                        Ok(UnwrappedMessage::new(
                            link,
                            prev_link,
                            MessageContent::unreadable(sequenced_msg),
                        ))
                    })
                    .await
            }
            unknown_content => err!(UnknownMsgType(unknown_content)),
        }
    }

    /// Get the previous msg link and msg type from header of message and return in a tuple alongside
    /// the message itself
    async fn parse_msg_info(&mut self, link: &Address) -> Result<(Address, u8, Message)> {
        let msg = self.transport.recv_message(link).await?;
        let preparsed: Preparsed = msg.parse_header().await?;
        let header = preparsed.header;
        let link = Address::try_from_bytes(&header.previous_msg_link.0)
            .or_else(|_| err!(NoPreviousMessage(link.to_string())))?;
        Ok((link, header.content_type, msg))
    }

    /// Receive and process a message with a known anchor link and message number. This can only
    /// be used if the channel is a single depth channel. [Author, Subscriber]
    ///
    ///   # Arguments
    ///   * `anchor_link` - Address of the anchor message for the channel
    ///   * `msg_num` - Sequence of sent message (not counting announce or any keyloads)
    pub async fn receive_msg_by_sequence_number(
        &mut self,
        anchor_link: &Address,
        msg_num: u32,
    ) -> Result<UnwrappedMessage> {
        if !self.is_single_depth() {
            return err(ChannelNotSingleDepth);
        }
        match self.author_public_key() {
            Some(pk) => {
                let seq_no = self.user.fetch_anchor()?.seq_no;
                let msg_cursor = self.user.gen_link(pk, anchor_link.rel(), seq_no + msg_num);
                let msg = self.transport.recv_message(&msg_cursor.link).await?;
                self.handle_message(msg, false).await
            }
            None => err(UserNotRegistered),
        }
    }
}

impl<Trans> IntoMessages<Trans> for User<Trans> {
    fn messages(&mut self) -> Messages<'_, Trans>
    where
        Trans: Transport,
    {
        Messages::new(self)
    }
}
