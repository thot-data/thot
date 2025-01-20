//! Errors
use serde::{Deserialize, Serialize};
use std::io;
use syre_core::Error as CoreError;
use syre_local::error::{Error as Local, IoSerde};
use thiserror::Error;

#[cfg(feature = "server")]
use crate::types::SocketType;

#[derive(Serialize, Deserialize, Debug)]
pub struct InvalidPath;

#[derive(Serialize, Deserialize, Debug)]
pub struct ContainerState {
    pub properties: Option<IoSerde>,
    pub settings: Option<IoSerde>,
    pub assets: Option<IoSerde>,
}

/// [`Database`](crate::db) related errors.
#[derive(Serialize, Deserialize, Error, Debug)]
pub enum Error {
    /// A ZMQ [`Context`](zmq::Context) does not exist where expected.
    #[error("ZMQ context does not exist")]
    ContextDoesNotExist,

    /// A ZMQ error.
    #[error("{0}")]
    ZMQ(String),

    /// A type of socket is required, but has not yet been created.
    #[cfg(feature = "server")]
    #[error("{0:?}")]
    SocketDoesNotExist(SocketType),

    #[cfg(feature = "server")]
    #[error("{0}")]
    SettingsError(String),

    #[error("{0:?}")]
    Core(CoreError),

    #[error("{0}")]
    Local(Local),

    #[error("{0}")]
    TrashError(String),

    /// Issue with the database.
    #[error("{0}")]
    Database(String),

    /// The database has become out of sync.
    #[error("database out of sync")]
    OutOfSync,

    #[error("{0}")]
    IoSerde(IoSerde),

    #[error("{message}: {error}")]
    Deserialize { message: String, error: String },
}

#[cfg(any(feature = "server", feature = "client"))]
impl From<zmq::Error> for Error {
    fn from(err: zmq::Error) -> Self {
        Self::ZMQ(err.message().to_string())
    }
}

impl From<CoreError> for Error {
    fn from(err: CoreError) -> Self {
        Self::Core(err)
    }
}

impl From<Local> for Error {
    fn from(err: Local) -> Self {
        Self::Local(err)
    }
}

impl From<IoSerde> for Error {
    fn from(value: IoSerde) -> Self {
        Self::IoSerde(value)
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Local(value.into())
    }
}

#[cfg(feature = "server")]
impl From<trash::Error> for Error {
    fn from(err: trash::Error) -> Self {
        Error::TrashError(format!("{err:?}"))
    }
}

pub type Result<T = ()> = std::result::Result<T, Error>;
