#![feature(io_error_more)]
#![feature(assert_matches)]

//! # Syre Core
//!
//! This package represents core functionality of the Syre software suite.
pub mod common;
pub mod constants;
pub mod error;
pub mod identifier;
pub mod lib_impl;
pub mod types;

#[cfg(feature = "project")]
pub mod project;

#[cfg(feature = "project")]
pub mod graph;

#[cfg(feature = "db")]
pub mod db;

#[cfg(feature = "runner")]
pub mod runner;

#[cfg(feature = "system")]
pub mod system;

// Re-exports
pub use error::{Error, Result};
