#![feature(io_error_more)]
#![feature(assert_matches)]
#![feature(result_flattening)]
//! # Syre Local Database
//! Implements a local database for Syre.
pub mod common;
pub mod event;
pub mod query;
pub mod state;

#[cfg(any(feature = "client", feature = "server"))]
pub mod constants;

#[cfg(any(feature = "client", feature = "server", feature = "error"))]
pub mod error;

#[cfg(any(feature = "client", feature = "server"))]
pub mod types;

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "server")]
pub mod server;

pub use event::Update;
pub use query::Query;

#[cfg(any(feature = "client", feature = "server", feature = "error"))]
pub use error::{Error, Result};

#[cfg(feature = "client")]
pub use client::Client;

#[cfg(feature = "server")]
pub use server::Database;

// #[cfg(target_arch = "wasm32")]
pub mod serde_os_string {
    use serde::{de::Visitor, Deserializer, Serializer};
    use std::{ffi::OsString, fmt, str::FromStr};

    pub fn serialize<S>(value: &OsString, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string_lossy().to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<OsString, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(OsStringVisitor)
    }

    struct OsStringVisitor;
    impl<'de> Visitor<'de> for OsStringVisitor {
        type Value = OsString;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("os string")
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            OsString::from_str(&v).map_err(|err| serde::de::Error::custom(format!("{err:?}")))
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            OsString::from_str(&v).map_err(|err| serde::de::Error::custom(format!("{err:?}")))
        }
    }
}
