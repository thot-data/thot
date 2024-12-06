//! Functionality and resources related to Syre Projects.

#[cfg(feature = "fs")]
pub mod asset;

pub mod container;

#[cfg(feature = "fs")]
pub mod project;

#[cfg(feature = "fs")]
pub mod resources;
