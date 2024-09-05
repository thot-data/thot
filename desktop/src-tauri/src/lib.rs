#![feature(io_error_more)]
#![feature(assert_matches)]

pub mod commands;
pub(crate) mod db;
mod setup;
pub mod state;

use crate::state::State;
pub use setup::setup;
