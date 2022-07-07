// Rust
use alloc::{boxed::Box, vec::Vec};

// 3rd-party
use anyhow::{anyhow, ensure, Result};
use async_trait::async_trait;

// IOTA

// Streams
use lets::{
    address::{Address, AppAddr, MsgId},
    id::{Identity, Psk, PskId},
    message::{Message, Topic, TransportMessage, HDF, PCF},
    transport::Transport,
};

// Local
use crate::{
    api::user::{self, User},
    message::{announcement, message_types},
};

/// Builder instance for a Streams User
pub struct UserBuilder<T> {
    /// Base Identity that will be used to Identifier a Streams User
    id: Option<Identity>,
    /// Transport Client instance
    transport: Option<T>,
    /// Pre Shared Keys
    psks: Vec<(PskId, Psk)>,
}

impl<T> Default for UserBuilder<T> {
    fn default() -> Self {
        UserBuilder {
            id: None,
            transport: None,
            psks: Default::default(),
        }
    }
}

impl UserBuilder<()> {
    /// Create a new User Builder instance
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

impl<T> UserBuilder<T> {
    /// Inject Base Identity into the User Builder
    ///
    /// # Arguments
    /// * `id` - UserIdentity to be used for base identification of the Streams User
    pub fn with_identity<I>(mut self, id: I) -> Self
    where
        I: Into<Identity>,
    {
        self.id = Some(id.into());
        self
    }

    /// Inject Transport Client instance into the User Builder
    ///
    /// # Arguments
    /// * `transport` - Transport Client to be used by the Streams User
    pub fn with_transport<NewTransport>(self, transport: NewTransport) -> UserBuilder<NewTransport>
    where
        NewTransport: for<'a> Transport<'a>,
    {
        UserBuilder {
            transport: Some(transport),
            id: self.id,
            psks: self.psks,
        }
    }

    /// Use the default version of the Transport Client
    pub async fn with_default_transport<NewTransport>(self) -> Result<UserBuilder<NewTransport>>
    where
        NewTransport: for<'a> Transport<'a> + DefaultTransport,
    {
        // Separated as a method instead of defaulting at the build method to avoid requiring the bespoke
        // bound T: DefaultTransport for all transports
        Ok(UserBuilder {
            transport: Some(NewTransport::try_default().await?),
            id: self.id,
            psks: self.psks,
        })
    }

    /// Inject a new Pre Shared Key and Id into the User Builder
    ///
    /// # Examples
    /// ## Add Multiple Psks
    /// ```
    /// # use anyhow::Result;
    /// use lets::id::Psk;
    /// use streams::{id::Ed25519, transport::tangle, User};
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let psk1 = Psk::from_seed(b"Psk1");
    /// let psk2 = Psk::from_seed(b"Psk2");
    /// let user = User::builder()
    ///     .with_default_transport::<tangle::Client>()
    ///     .await?
    ///     .with_psk(psk1.to_pskid(), psk1)
    ///     .with_psk(psk2.to_pskid(), psk2)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Arguments
    /// * `pskid` - Pre Shared Key Identifier
    /// * `psk` - Pre Shared Key shared outside of Streams scope
    pub fn with_psk(mut self, pskid: PskId, psk: Psk) -> Self {
        self.psks.push((pskid, psk));
        self
    }

    /// Build a [`User`] instance using the Builder parameters.
    ///
    /// If a [`Transport`] is not provided the builder will use a default client
    /// ([`Client`](streams_app::transport::tangle::client::Client) at <https://chrysalis-nodes.iota.org>
    /// if the `tangle` feature is enabled,
    /// [`BucketTransport`](streams_app::transport::BucketTransport) if not)
    ///
    /// # Errors
    /// This function will error out if the [`UserIdentity`] parameter is missing, as this makes up
    /// the essence of a [`User`] and is required for any use case.
    ///
    /// # Examples
    /// ## User from Ed25519
    /// ```
    /// # use anyhow::Result;
    /// use streams::{id::Ed25519, transport::tangle, User};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let user_seed = "cryptographically-secure-random-user-seed";
    /// let mut user = User::builder()
    ///     .with_identity(Ed25519::from_seed(user_seed))
    ///     .with_default_transport::<tangle::Client>()
    ///     .await?
    ///     .with_identity(Ed25519::from_seed(user_seed))
    ///     .build()?;
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> Result<User<T>> {
        let transport = self
            .transport
            .ok_or_else(|| anyhow!("transport not specified, cannot build User without Transport"))?;

    //     let stream_base_address = AppAddr::gen(&identifier, &topic);
    //     let stream_rel_address = MsgId::gen(stream_base_address, &identifier, &topic, user::ANN_MESSAGE_NUM);
    //     let stream_address = Address::new(stream_base_address, stream_rel_address);
        
        Ok(User::new(self.id, stream_address, stream_topic, author_identifier, self.psks, transport))
    }

    pub fn build_as_stream_author(self, stream_topic: Topic) -> Result<User<T>> {
        let transport = self
            .transport
            .ok_or_else(|| anyhow!("transport not specified, cannot build User without Transport"))?;

        let identity = self
            .id
            .ok_or_else(|| anyhow!("user must have an identity to create a stream"))?;
        
        Ok(User::new_as_stream_author(identity, stream_topic, self.psks, transport))
    }

    // pub async fn create_stream<Top: Into<Topic>>(&mut self, topic: Top) -> Result<User<T>>
    // where
    //     T: for<'a> Transport<'a, Msg = TransportMessage>,
    // {
    //     let transport = self
    //         .transport
    //         .ok_or_else(|| anyhow!("transport not specified, cannot build user without transport"))?;

    //     let identity = self
    //         .id
    //         .ok_or_else(|| anyhow!("user must have an identity to create a stream"))?;
    //     let identifier = identity.to_identifier();
    //     // Convert topic
    //     let topic = topic.into();
    //     // Generate stream address
    //     let stream_base_address = AppAddr::gen(&identifier, &topic);
    //     let stream_rel_address = MsgId::gen(stream_base_address, &identifier, &topic, user::ANN_MESSAGE_NUM);
    //     let stream_address = Address::new(stream_base_address, stream_rel_address);

    //     // Prepare HDF and PCF
    //     let header = HDF::new(
    //         message_types::ANNOUNCEMENT,
    //         user::ANN_MESSAGE_NUM,
    //         identifier.clone(),
    //         topic.clone(),
    //     )?;
    //     let content = PCF::new_final_frame().with_content(announcement::Wrap::new(&identity));

    //     // Wrap message
    //     let (transport_msg, spongos) = Message::new(header, content).wrap().await?;

    //     // Attempt to send message
    //     ensure!(
    //         // TODO: only on not-found error
    //         transport.recv_message(stream_address).await.is_err(),
    //         anyhow!("stream with address '{}' already exists", stream_address)
    //     );
    //     let send_response = transport.send_message(stream_address, transport_msg).await?;
    //     Ok(User::new(Some(identity), stream_address, topic, self.psks, transport))
    // }

    /// Recover a user instance from the builder parameters.
    ///
    /// # Arguements
    /// * `announcement` - An existing announcement message link from which to recover the state of
    ///   the user
    ///
    /// # Caveats
    /// Under the hood, this method recovers the user by rereading all the
    /// messages of the Stream. Besides the obvious caveat of the potential cost
    /// of execution, keep in mind that only the information present as messages
    /// in the stream will be recovered; OOB actions, particularly manually
    /// added or removed subscribers and PSK, will not be recovered and will
    /// need to be reapplied manually.
    ///
    /// # Errors
    /// This function will produce errors if the [`User`] tries to recover their
    /// instance without a proper [`Identity`]. It will also return an error
    /// if the provided announcement link is not present on the transport layer.
    ///
    /// # Example
    /// ```
    /// # use std::cell::RefCell;
    /// # use std::rc::Rc;
    /// # use anyhow::Result;
    /// # use streams::transport::bucket;
    /// use streams::{id::Ed25519, transport::tangle, User};
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// # let test_transport = Rc::new(RefCell::new(bucket::Client::new()));
    /// let author_seed = "author_secure_seed";
    /// let transport: tangle::Client =
    ///     tangle::Client::for_node("https://chrysalis-nodes.iota.org").await?;
    /// #
    /// # let transport = test_transport.clone();
    /// # let mut author = User::builder()
    /// #     .with_identity(Ed25519::from_seed(author_seed))
    /// #     .with_transport(transport.clone())
    /// #     .build()?;
    /// # let announcement_address = author.create_stream("BASE_BRANCH").await?.address();
    ///
    /// let author = User::builder()
    ///     .with_identity(Ed25519::from_seed(author_seed))
    ///     .with_transport(transport)
    ///     .recover(announcement_address)
    ///     .await?;
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub async fn recover(self, announcement: Address) -> Result<User<T>>
    where
        T: for<'a> Transport<'a, Msg = TransportMessage>,
    {
        let mut user = self.build()?;
        user.receive_message(announcement).await?;
        user.sync().await?;
        Ok(user)
    }
}

#[async_trait(?Send)]
pub trait DefaultTransport
where
    Self: Sized,
{
    async fn try_default() -> Result<Self>;
}

#[async_trait(?Send)]
#[cfg(any(feature = "tangle-client", feature = "tangle-client-wasm"))]
impl<Message, SendResponse> DefaultTransport for lets::transport::tangle::Client<Message, SendResponse> {
    async fn try_default() -> Result<Self> {
        Self::for_node("https://chrysalis-nodes.iota.org").await
    }
}
