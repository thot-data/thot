//! # Syre Local
//! This package contains local functionality and types of the Syre software suite.
#![feature(io_error_more)]
#![feature(path_file_prefix)]

pub mod common;
pub mod constants;
pub mod error;
pub mod identifier;
pub mod loader;
pub mod project;
pub mod system;
pub mod types;

#[cfg(feature = "fs")]
pub mod graph;

#[cfg(feature = "fs")]
pub mod file_resource;

// Re-exports
pub use error::{Error, Result};

/// Indicates the state of the object can be modified by the given action.
/// The state transition must not fail.
pub trait Reducible {
    type Action;
    fn reduce(&mut self, action: Self::Action);
}

/// Indicates the state of the object can be modified by the given action.
/// The state transition may fail.
pub trait TryReducible {
    type Action;
    type Error;
    fn try_reduce(&mut self, action: Self::Action) -> std::result::Result<(), Self::Error>;
}
