use core::cell::RefCell;

use iota_streams_core::{
    async_trait,
    prelude::{
        Box,
        Rc,
        Vec,
    },
    Result,
};

/// Network transport abstraction.
/// Parametrized by the type of message links.
/// Message link is used to identify/locate a message (eg. like URL for HTTP).
#[async_trait(?Send)]
pub trait Transport<Link, Msg> {
    /// Send a message with default options.
    async fn send_message(&mut self, msg: &Msg) -> Result<()>;

    /// Receive a message with default options.
    async fn recv_message(&mut self, link: &Link) -> Result<Msg>;
}

#[async_trait(?Send)]
impl<Link, Msg, Tsp: Transport<Link, Msg>> Transport<Link, Msg> for Rc<RefCell<Tsp>> {
    // Send a message.
    async fn send_message(&mut self, msg: &Msg) -> Result<()> {
        self.borrow_mut().send_message(msg).await
    }

    // Receive a message with default options.
    async fn recv_message(&mut self, link: &Link) -> Result<Msg> {
        self.borrow_mut().recv_message(link).await
    }
}

#[cfg(any(feature = "sync-spin", feature = "sync-parking-lot"))]
mod sync {
    use super::Transport;
    use iota_streams_core::{
        async_trait,
        prelude::{
            Arc,
            Box,
            Mutex,
        },
        Result,
    };

    #[async_trait(?Send)]
    impl<Link, Msg, Tsp: Transport<Link, Msg>> Transport<Link, Msg> for Arc<Mutex<Tsp>> {
        // Send a message.
        async fn send_message(&mut self, msg: &Msg) -> Result<()> {
            self.lock().send_message(msg).await
        }

        // Receive a message with default options.
        async fn recv_message(&mut self, link: &Link) -> Result<Msg> {
            self.lock().recv_message(link).await
        }
    }
}

mod bucket;
pub use bucket::BucketTransport;
use iota_streams_core::try_or;

#[cfg(feature = "tangle")]
pub mod tangle;
