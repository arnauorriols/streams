//! `Keyload` message _wrapping_ and _unwrapping_.
//!
//! The `Keyload` message is the means to securely exchange the encryption key of a branch with a
//! set of subscribers.
//!
//! ```ddml
//! message Keyload {
//!     skip link msgid;
//!     join(msgid);
//!     absorb                      u8  nonce[32];
//!     absorb repeated(n):
//!       fork;
//!       match identifier:
//!         EdPubKey:
//!           mask                  u8  id_type(0);
//!           mask                  u8  ed25519_pubkey[32];
//!           x25519(pub/priv_key)  u8  x25519_pubkey[32];
//!           commit;
//!           mask                  u8  key[32];
//!         PskId:
//!           mask                  u8  id_type(1);          
//!           mask                  u8  psk_id[16];
//!           commit;
//!           mask                  u8  key[32];
//!       commit;
//!       squeeze external          u8  ids_hash[64];
//!     absorb external             u8  key[32];
//!     fork;
//!     absorb external             u8  ids_hash[64];
//!     commit;
//!     squeeze external            u8  hash[64];
//!     ed25519(hash)               u8  signature[64];
//!     commit;
//! }
//! ```
// Rust
use alloc::{boxed::Box, vec::Vec};
use core::iter::IntoIterator;

// 3rd-party
use anyhow::Result;
use async_trait::async_trait;

// IOTA
use crypto::keys::x25519;

// Streams
use lets::{
    id::{Identifier, Identity, Permissioned},
    message::{
        self, ContentDecrypt, ContentEncrypt, ContentEncryptSizeOf, ContentSign, ContentSignSizeof, ContentVerify,
    },
};
use spongos::{
    ddml::{
        commands::{sizeof, unwrap, wrap, Absorb, Commit, Fork, Join, Mask},
        io,
        modifiers::External,
        types::{NBytes, Size, Uint64},
    },
    Spongos,
};

// Local

const NONCE_SIZE: usize = 16;
const KEY_SIZE: usize = 32;

pub(crate) struct Wrap<'a, Subscribers> {
    initial_state: &'a mut Spongos,
    nonce: [u8; NONCE_SIZE],
    key: [u8; KEY_SIZE],
    subscribers: Subscribers,
    author_id: &'a Identity,
}

impl<'a, Subscribers> Wrap<'a, Subscribers> {
    pub(crate) fn new(
        initial_state: &'a mut Spongos,
        subscribers: Subscribers,
        key: [u8; KEY_SIZE],
        nonce: [u8; NONCE_SIZE],
        author_id: &'a Identity,
    ) -> Self
    where
        Subscribers: IntoIterator<Item = &'a (Permissioned<Identifier>, usize, &'a [u8])> + Clone,
        Subscribers::IntoIter: ExactSizeIterator,
    {
        Self {
            initial_state,
            subscribers,
            key,
            nonce,
            author_id,
        }
    }
}

#[async_trait]
impl<'a, Subscribers> message::ContentSizeof<Wrap<'a, Subscribers>> for sizeof::Context
where
    Subscribers: IntoIterator<Item = &'a (Permissioned<Identifier>, usize, &'a [u8])> + Clone + Send + Sync,
    Subscribers::IntoIter: ExactSizeIterator + Send,
{
    async fn sizeof(&mut self, keyload: &Wrap<'a, Subscribers>) -> Result<&mut sizeof::Context> {
        let subscribers = keyload.subscribers.clone().into_iter();
        let n_subscribers = Size::new(subscribers.len());
        self.absorb(NBytes::new(keyload.nonce))?.absorb(n_subscribers)?;
        // Loop through provided identifiers, masking the shared key for each one
        for (subscriber, cursor, exchange_key) in subscribers {
            self.fork()
                .mask(subscriber)?
                .absorb(Uint64::new(*cursor as u64))?
                .encrypt_sizeof(subscriber.identifier(), exchange_key, &keyload.key)
                .await?;
        }
        self.absorb(External::new(&NBytes::new(&keyload.key)))?
            .sign_sizeof(keyload.author_id)
            .await?
            .commit()?;
        Ok(self)
    }
}

#[async_trait]
impl<'a, OS, Subscribers> message::ContentWrap<Wrap<'a, Subscribers>> for wrap::Context<OS>
where
    Subscribers: IntoIterator<Item = &'a (Permissioned<Identifier>, usize, &'a [u8])> + Clone + Send + Sync,
    Subscribers::IntoIter: ExactSizeIterator + Send,
    OS: io::OStream + Send,
{
    async fn wrap(&mut self, keyload: &mut Wrap<'a, Subscribers>) -> Result<&mut Self> {
        let subscribers = keyload.subscribers.clone().into_iter();
        let n_subscribers = Size::new(subscribers.len());
        self.join(keyload.initial_state)?
            .absorb(NBytes::new(keyload.nonce))?
            .absorb(n_subscribers)?;
        // Loop through provided identifiers, masking the shared key for each one
        for (subscriber, cursor, exchange_key) in subscribers {
            self.fork()
                .mask(subscriber)?
                .absorb(Uint64::new(*cursor as u64))?
                .encrypt(subscriber.identifier(), exchange_key, &keyload.key)
                .await?;
        }
        self.absorb(External::new(&NBytes::new(&keyload.key)))?
            .sign(keyload.author_id)
            .await?
            .commit()?;
        Ok(self)
    }
}

pub(crate) struct Unwrap<'a> {
    initial_state: &'a mut Spongos,
    subscribers: Vec<(Permissioned<Identifier>, usize)>,
    author_id: Identifier,
    user_id: &'a Identity,
    user_ke_key: &'a [u8],
}

impl<'a> Unwrap<'a> {
    pub(crate) fn new(
        initial_state: &'a mut Spongos,
        user_id: &'a Identity,
        user_ke_key: &'a [u8],
        author_id: Identifier,
    ) -> Self {
        Self {
            initial_state,
            subscribers: Default::default(),
            author_id,
            user_id,
            user_ke_key,
        }
    }

    pub(crate) fn subscribers(&self) -> &[(Permissioned<Identifier>, usize)] {
        &self.subscribers
    }

    pub(crate) fn into_subscribers(self) -> Vec<(Permissioned<Identifier>, usize)> {
        self.subscribers
    }
}

#[async_trait]
impl<'a, IS> message::ContentUnwrap<Unwrap<'a>> for unwrap::Context<IS>
where
    IS: io::IStream + Send,
{
    async fn unwrap(&mut self, keyload: &mut Unwrap<'a>) -> Result<&mut Self> {
        let mut nonce = [0u8; NONCE_SIZE];
        let mut key = None;
        let mut n_subscribers = Size::default();
        self.join(keyload.initial_state)?
            .absorb(NBytes::new(&mut nonce))?
            .absorb(&mut n_subscribers)?;

        for _ in 0..n_subscribers.inner() {
            let mut fork = self.fork();
            // Loop through provided number of identifiers and subsequent keys
            let mut subscriber_id = Permissioned::<Identifier>::default();
            let mut cursor = Uint64::default();
            fork.mask(&mut subscriber_id)?.absorb(&mut cursor)?;

            if subscriber_id.identifier() == &keyload.user_id.to_identifier() {
                fork.decrypt(keyload.user_id, keyload.user_ke_key, key.get_or_insert([0; KEY_SIZE]))
                    .await?;
            } else {
                // Key is meant for another subscriber, skip it
                if subscriber_id.identifier().is_psk() {
                    fork.drop(KEY_SIZE)?;
                } else {
                    fork.drop(KEY_SIZE + x25519::PUBLIC_KEY_LENGTH)?;
                }
            }
            keyload.subscribers.push((subscriber_id, cursor.inner() as usize));
        }
        if let Some(key) = key {
            self.absorb(External::new(&NBytes::new(&key)))?
                .verify(&keyload.author_id)
                .await?;
        }
        self.commit()?;
        Ok(self)
    }
}
