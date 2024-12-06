#![feature(assert_matches)]
//! File system event handler.
mod command;
pub mod error;
pub mod event;

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "server")]
pub mod server;

pub use command::Command;
pub use error::Error;
pub use event::{Event, EventKind, EventResult};

pub type Result<T = ()> = std::result::Result<T, Error>;

#[cfg(feature = "client")]
pub use client::Client;
