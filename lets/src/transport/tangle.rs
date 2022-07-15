// Rust
use alloc::{boxed::Box, vec::Vec};
use core::{
    any::Any,
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};

// 3rd-party
use anyhow::{anyhow, ensure};
use async_trait::async_trait;
use futures::{
    future::{ready, try_join_all},
    TryFutureExt,
};

// IOTA
use crypto::hashes::{blake2b::Blake2b256, Digest};
use iota_client::bee_message::{payload::Payload, Message as IotaMessage};

// Streams

// Local
use crate::{address::Address, message::TransportMessage, transport::Transport};

#[derive(Debug)]
pub struct Client<Message = TransportMessage, SendResponse = TransportMessage>(
    iota_client::Client,
    PhantomData<(Message, SendResponse)>,
);

impl<Message, SendResponse> Client<Message, SendResponse> {
    // Create an instance of Client with a ready client and its send options
    pub fn new(client: iota_client::Client) -> Self {
        Self(client, PhantomData)
    }

    // Shortcut to create an instance of Client connecting to a node with default parameters
    pub async fn for_node(node_url: &str) -> Result<Client<Message, SendResponse>, Box<dyn Any + Send + Sync>> {
        Ok(Self(
            iota_client::ClientBuilder::new()
                .with_node(node_url)
                .map_err(|e| Box::new(anyhow::Error::from(e)) as Box<dyn Any + Send + Sync>)?
                .with_local_pow(true)
                .finish()
                .map_err(|e| Box::new(anyhow::Error::from(e)) as Box<dyn Any + Send + Sync>)
                .await?,
            PhantomData,
        ))
    }

    pub fn client(&self) -> &iota_client::Client {
        &self.0
    }

    pub fn client_mut(&mut self) -> &mut iota_client::Client {
        &mut self.0
    }
}

#[async_trait(?Send)]
impl<Message, SendResponse> Transport<'_> for Client<Message, SendResponse>
where
    Message: Into<Vec<u8>> + TryFrom<IotaMessage, Error = anyhow::Error>,
    SendResponse: TryFrom<IotaMessage, Error = anyhow::Error>,
{
    type Msg = Message;
    type SendResponse = SendResponse;

    async fn send_message(&mut self, address: Address, msg: Message) -> Result<SendResponse, Box<dyn Any + Send + Sync>>
    where
        Message: 'async_trait,
    {
        self.client()
            .message()
            .with_index(address.to_msg_index())
            .with_data(msg.into())
            .finish()
            .map_err(|e| Box::new(e) as Box<dyn Any + Send + Sync>)
            .await?
            .try_into()
            .map_err(|e| Box::new(e) as Box<dyn Any + Send + Sync>)
    }

    async fn recv_messages(&mut self, address: Address) -> Result<Vec<Message>, Box<dyn Any + Send + Sync>> {
        let msg_ids = self
            .client()
            .get_message()
            .index(address.to_msg_index())
            .await
            .map_err(|e| Box::new(e) as Box<dyn Any + Send + Sync>)?;
        if msg_ids.is_empty() {
            return Err(Box::new(anyhow!("no message found at index '{}'", address)));
        }

        try_join_all(msg_ids.iter().map(|msg| {
            self.client()
                .get_message()
                .data(msg)
                .map_err(Into::into)
                .and_then(|iota_message| ready(iota_message.try_into()))
        }))
        .map_err(|e| Box::new(e) as Box<dyn Any + Send + Sync>)
        .await
    }
}

impl TryFrom<IotaMessage> for TransportMessage {
    type Error = anyhow::Error;
    fn try_from(message: IotaMessage) -> Result<Self, Self::Error> {
        if let Some(Payload::Indexation(indexation)) = message.payload() {
            Ok(Self::new(indexation.data().into()))
        } else {
            Err(anyhow!(
                "expected an indexation payload from the Tangle, received something else"
            ))
        }
    }
}

impl Address {
    /// Hash the content of the [`Address`] using `Blake2b256`
    pub fn to_blake2b(self) -> [u8; 32] {
        let hasher = Blake2b256::new();
        hasher.chain(self.base()).chain(self.relative()).finalize().into()
    }

    /// An `Address` is used as index of the message over the Tangle. For that,
    /// its content is hashed using [`Address::to_blake2b()`].
    ///
    /// ```
    /// # use lets::address::Address;
    /// #
    /// # fn main() -> anyhow::Result<()> {
    /// let address = Address::new([172; 40], [171; 12]);
    /// assert_eq!(
    ///     address.to_msg_index().as_ref(),
    ///     &[
    ///         44, 181, 155, 1, 109, 141, 169, 177, 209, 70, 226, 18, 190, 121, 40, 44, 90, 108, 159,
    ///         109, 241, 37, 30, 0, 185, 80, 245, 59, 235, 75, 128, 97
    ///     ],
    /// );
    /// assert_eq!(
    ///     &format!("{}", hex::encode(address.to_msg_index())),
    ///     "2cb59b016d8da9b1d146e212be79282c5a6c9f6df1251e00b950f53beb4b8061"
    /// );
    /// #   Ok(())
    /// # }
    /// ```
    pub fn to_msg_index(self) -> [u8; 32] {
        self.to_blake2b()
    }
}
