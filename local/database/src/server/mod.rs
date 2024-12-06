//! Database that watches file system for changes, publishing them for clients.
pub(self) mod database;
mod state;
pub(self) mod store;
mod types;

pub use database::{config, Builder, Config, Database};
use state::State;
