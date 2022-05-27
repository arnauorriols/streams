//! `Subscribe` message _wrapping_ and _unwrapping_.
//!
//! `Subscribe` messages are published by a user willing to become a subscriber to this channel.
//!
//! They contain the subscriber's identifier that will be used in keyload
//! messages to encrypt session keys.
//!
//! Subscriber's Ed25519 public key is encrypted with the `unsubscribe_key`
//! which in turn is encapsulated for channel owner using owner's Ed25519 public
//! key. The resulting spongos state will be used for unsubscription.
//! Subscriber must trust channel owner's Ed25519 public key in order to
//! maintain privacy.
//!
//! Channel Owner must maintain the resulting spongos state associated to the Subscriber's
//! Ed25519 public key.
//!
//! ```ddml
//! message Subscribe {
//!     skip                    link    msgid;
//!     join(msgid);
//!     x25519(pub/priv_key)    u8      x25519_auth_pubkey[32];
//!     commit;
//!     mask                    u8      unsubscribe_key[32];
//!     mask                    u8      pk[32];
//!     commit;
//!     squeeze external        u8      hash[64];
//!     ed25519(hash)           u8      signature[64];
//! }
//! ```
// Rust
use alloc::boxed::Box;

// 3rd-party
use anyhow::Result;
use async_trait::async_trait;

// IOTA
use crypto::keys::x25519;

// Streams
use lets::{
    id::{Identifier, Identity},
    message::{ContentSign, ContentSignSizeof, ContentSizeof, ContentUnwrap, ContentVerify, ContentWrap},
};
use spongos::{
    ddml::{
        commands::{sizeof, unwrap, wrap, Absorb, Join, Mask, X25519},
        io,
        types::NBytes,
    },
    Spongos,
};

pub(crate) struct Wrap<'a> {
    initial_state: &'a mut Spongos,
    unsubscribe_key: [u8; 32],
    subscriber_id: &'a Identity,
    author_ke_pk: &'a x25519::PublicKey,
}

impl<'a> Wrap<'a> {
    pub(crate) fn new(
        initial_state: &'a mut Spongos,
        unsubscribe_key: [u8; 32],
        subscriber_id: &'a Identity,
        author_ke_pk: &'a x25519::PublicKey,
    ) -> Self {
        Self {
            initial_state,
            unsubscribe_key,
            subscriber_id,
            author_ke_pk,
        }
    }
}

#[async_trait]
impl<'a> ContentSizeof<Wrap<'a>> for sizeof::Context {
    async fn sizeof(&mut self, subscription: &Wrap<'a>) -> Result<&mut Self> {
        self.x25519(subscription.author_ke_pk, NBytes::new(subscription.unsubscribe_key))?
            .mask(&subscription.subscriber_id.to_identifier())?
            .absorb(
                &subscription
                    .subscriber_id
                    ._ke_sk()
                    .expect("only users with an identity capable of key exchange can send subscriptions")
                    .public_key(),
            )?
            .sign_sizeof(subscription.subscriber_id)
            .await?;
        Ok(self)
    }
}

#[async_trait]
impl<'a, OS> ContentWrap<Wrap<'a>> for wrap::Context<OS>
where
    OS: io::OStream + Send,
{
    async fn wrap(&mut self, subscription: &mut Wrap<'a>) -> Result<&mut Self> {
        self.join(subscription.initial_state)?
            .x25519(subscription.author_ke_pk, NBytes::new(subscription.unsubscribe_key))?
            .mask(&subscription.subscriber_id.to_identifier())?
            .absorb(
                &subscription
                    .subscriber_id
                    ._ke_sk()
                    .expect("only users with an identity capable of key exchange can send subscriptions")
                    .public_key(),
            )?
            .sign(subscription.subscriber_id)
            .await?;
        Ok(self)
    }
}

pub(crate) struct Unwrap<'a> {
    initial_state: &'a mut Spongos,
    unsubscribe_key: [u8; 32],
    subscriber_identifier: Identifier,
    // TODO: REMOVE ONCE KE IS ENCAPSULATED WITHIN IDENTITY
    subscriber_ke_pk: x25519::PublicKey,
    author_ke_sk: &'a x25519::SecretKey,
}

impl<'a> Unwrap<'a> {
    pub(crate) fn new(initial_state: &'a mut Spongos, author_ke_sk: &'a x25519::SecretKey) -> Self {
        Self {
            initial_state,
            unsubscribe_key: Default::default(),
            subscriber_identifier: Default::default(),
            subscriber_ke_pk: x25519::PublicKey::from_bytes([0; x25519::PUBLIC_KEY_LENGTH]),
            author_ke_sk,
        }
    }

    pub(crate) fn subscriber_identifier(&self) -> Identifier {
        self.subscriber_identifier
    }

    // #[deprecated = "to be removed once ke is encapsulated within identity"]
    pub(crate) fn subscriber_ke_pk(&self) -> x25519::PublicKey {
        self.subscriber_ke_pk
    }
}

#[async_trait]
impl<'a, IS> ContentUnwrap<Unwrap<'a>> for unwrap::Context<IS>
where
    IS: io::IStream + Send,
{
    async fn unwrap(&mut self, subscription: &mut Unwrap<'a>) -> Result<&mut Self> {
        self.join(subscription.initial_state)?
            .x25519(
                subscription.author_ke_sk,
                NBytes::new(&mut subscription.unsubscribe_key),
            )?
            .mask(&mut subscription.subscriber_identifier)?
            .absorb(&mut subscription.subscriber_ke_pk)?
            .verify(&subscription.subscriber_identifier)
            .await?;
        Ok(self)
    }
}
