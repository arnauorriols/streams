//! High-level Implementation of Streams Channel Protocol.
//!
//! API functions can be found through the [User](api::tangle::User)
//!
//! User implementations will require a Transport
//! [Client](../iota_streams_app/transport/tangle/client/struct.Client.html)
//!
//! ## Starting a new Channel
//! ```
//! # use anyhow::Result;
//! use iota_streams::{
//!     transport::tangle,
//!     id::Ed25519,
//!     User,
//! };
//! # use iota_streams::transport::bucket;
//! #[tokio::main]
//! async fn main() -> Result<()> {
//! let transport: tangle::Client = tangle::Client::for_node("https://chrysalis-nodes.iota.org").await?;
//! # let test_transport = bucket::Client::new();
//! let mut author = User::builder()
//!     .with_identity(Ed25519::from_seed("A cryptographically secure seed"))
//!     .with_transport(transport)
//! #     .with_transport(test_transport)
//!     .build()?;
//!
//! author.create_stream(1)?;
//! let announcement = author.announce().await?;
//! # Ok(())
//! # }
//! ```

#![no_std]

// Uncomment to enable printing for development
// #[macro_use]
// extern crate std;

#[macro_use]
extern crate alloc;

/// Protocol message types and encodings
mod message;

/// [`User`] API.
mod api;

pub use api::{message::Message, send_response::SendResponse, user::User};
pub use LETS::{address::Address, id, message::TransportMessage, transport};
