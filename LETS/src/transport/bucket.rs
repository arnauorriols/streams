// Rust
use alloc::{
    boxed::Box,
    collections::BTreeMap,
    vec::Vec,
};
use core::fmt::Display;

// 3rd-party
use anyhow::{
    anyhow,
    ensure,
    Result,
};
use async_trait::async_trait;

// IOTA

// Streams

// Local
use crate::{
    link::Addressable,
    transport::Transport,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Client<Address, Msg> {
    // Use BTreeMap instead of HashMap to make BucketTransport nostd without pulling hashbrown
    // (this transport is for hacking purposes only, performance is no concern)
    bucket: BTreeMap<Address, Vec<Msg>>,
}

impl<Address, Msg> Client<Address, Msg> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<Link, Msg> Default for Client<Link, Msg> {
    // Implement default manually because derive puts Default bounds in type parameters
    fn default() -> Self {
        Self {
            bucket: BTreeMap::default(),
        }
    }
}

#[async_trait(?Send)]
impl<'a, Address, Msg> Transport<&'a Address, Msg, Msg> for Client<Address, Msg>
where
    Address: Ord + Display + Clone,
    Msg: Clone,
{
    async fn send_message(&mut self, addr: &'a Address, msg: Msg) -> Result<Msg> {
        self.bucket.entry(addr.clone()).or_default().push(msg.clone());
        Ok(msg)
    }

    async fn recv_messages(&mut self, address: &'a Address) -> Result<Vec<Msg>> {
        self.bucket
            .get(address)
            .cloned()
            .ok_or_else(|| anyhow!("No messages found at address {}", address))
    }
}