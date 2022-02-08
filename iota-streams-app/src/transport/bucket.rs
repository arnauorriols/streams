use super::*;
use crate::message::LinkedMessage;
use core::hash;

use iota_streams_core::{
    err,
    prelude::{
        string::ToString,
        HashMap,
    },
    Errors::MessageLinkNotFoundInBucket,
};

use iota_streams_core::{
    async_trait,
    prelude::Box,
    Errors::MessageNotUnique,
};

#[derive(Clone, Debug)]
pub struct BucketTransport<Link, Msg> {
    bucket: HashMap<Link, Vec<Msg>>,
}

impl<Link, Msg> Default for BucketTransport<Link, Msg>
where
    Link: Eq + hash::Hash,
{
    fn default() -> Self {
        Self { bucket: HashMap::new() }
    }
}

impl<Link, Msg> BucketTransport<Link, Msg>
where
    Link: Eq + hash::Hash + Clone + ToString,
    Msg: Clone,
{
    pub fn new() -> Self {
        Self { bucket: HashMap::new() }
    }

    async fn recv_messages(&mut self, link: &Link) -> Result<Vec<Msg>> {
        if let Some(msgs) = self.bucket.get(link) {
            Ok(msgs.clone())
        } else {
            err!(MessageLinkNotFoundInBucket(link.to_string()))
        }
    }
}

#[async_trait(?Send)]
impl<Link, Msg> Transport<Link, Msg> for BucketTransport<Link, Msg>
where
    Link: ToString + Eq + hash::Hash + Clone,
    Msg: LinkedMessage<Link> + Clone,
{
    async fn send_message(&mut self, msg: &Msg) -> Result<()> {
        if let Some(msgs) = self.bucket.get_mut(msg.link()) {
            msgs.push(msg.clone());
            Ok(())
        } else {
            self.bucket.insert(msg.link().clone(), vec![msg.clone()]);
            Ok(())
        }
    }

    async fn recv_message(&mut self, link: &Link) -> Result<Msg> {
        let mut msgs = self.recv_messages(link).await?;
        if let Some(msg) = msgs.pop() {
            try_or!(msgs.is_empty(), MessageNotUnique(link.to_string())).unwrap();
            Ok(msg)
        } else {
            err!(MessageLinkNotFoundInBucket(link.to_string()))?
        }
    }
}
