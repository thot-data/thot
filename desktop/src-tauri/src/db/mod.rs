//! Interaction with a [`Database`](syre_local::db::Database).
pub mod actor;
mod init;

pub use init::start_database_if_needed;

/// Event to listen to to recieve file system event updates.
pub const FS_EVENT_TOPIC: &str = "fs-updates";
