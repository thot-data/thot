/// System level functionality related to Syre.
/// This includes handling system level resources such as users and Scripts,
/// as well as system settings.
#[cfg(feature = "fs")]
pub mod common;

#[cfg(feature = "fs")]
pub mod template;

#[cfg(feature = "fs")]
pub mod collections;

#[cfg(feature = "fs")]
pub mod config;

#[cfg(feature = "fs")]
pub mod project_manifest;

#[cfg(feature = "fs")]
pub mod scripts;

#[cfg(feature = "fs")]
pub mod user_manifest;

pub mod resources;
