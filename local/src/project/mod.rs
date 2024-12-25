//! Functionality and resources related to Syre Projects.
pub mod config;
pub mod container;

#[cfg(feature = "fs")]
pub mod asset;

#[cfg(feature = "fs")]
pub mod project;

#[cfg(feature = "fs")]
pub mod resources;
