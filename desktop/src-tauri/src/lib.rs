#![feature(io_error_more)]
#![feature(assert_matches)]
#![feature(result_flattening)]

pub mod commands;
pub mod common;
pub(crate) mod db;
pub mod settings;
mod setup;
pub mod state;

pub use setup::setup;
pub use state::State;

pub mod identifier {
    /// Identifier information for Syre desktop related to storing app data.
    pub struct Identifier;

    impl Identifier {
        pub fn application() -> String {
            String::from("syre-desktop")
        }
    }
}
