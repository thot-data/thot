//! Common error types.
pub use io_error_kind::IoErrorKind;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;
use std::result::Result as StdResult;
use syre_core::Error as CoreError;
use thiserror::Error;

// ***********************
// *** Settings Errors ***
// ***********************

#[cfg(feature = "fs")]
#[derive(Serialize, Deserialize, Error, Debug)]
pub enum SettingsFileError {
    #[error("could not load `{0}`")]
    CouldNotLoad(PathBuf),

    #[error("could not save `{0}`")]
    CouldNotSave(PathBuf),
}

#[derive(Serialize, Deserialize, Error, Debug)]
pub enum SettingsValidationError {
    #[error("invalid settings")]
    InvalidSetting,
}

// **********************
// *** Project Error ***
// **********************

#[cfg(feature = "fs")]
#[derive(Serialize, Deserialize, Error, Debug)]
pub enum Project {
    #[error("`{0}` already registered")]
    DuplicatePath(PathBuf),

    #[error("`{0}` not a Project root")]
    PathNotAProjectRoot(PathBuf),

    #[error("`{0}` not in a Project")]
    PathNotInProject(PathBuf),

    #[error("`{0}` is not a resource")]
    PathNotAResource(PathBuf),

    #[error("`{0}` is not registered")]
    PathNotRegistered(PathBuf),
}

// ***********************
// *** Container Error ***
// ***********************

#[derive(Serialize, Deserialize, Error, Debug)]
pub enum ContainerError {
    #[error("`{0}` is not a child Container")]
    InvalidChildPath(PathBuf),

    /// If a path is expected to represent a [`Container`](crate::project::resources::Container)
    /// but does not.
    #[error("`{0}` is not a Container")]
    PathNotAContainer(PathBuf),

    /// If two Containers with the same parent have the same name.
    #[error("clashing Container names")]
    ContainerNameConflict,
}

// *******************
// *** Asset Error ***
// *******************

#[derive(Serialize, Deserialize, Error, Debug)]
pub enum AssetError {
    #[error("`{0}` is not a Container")]
    PathNotAContainer(PathBuf),

    #[error("`{0}` is already an Asset")]
    FileAlreadyAsset(PathBuf),

    #[error("`{0}` not found")]
    ContainerNotFound(PathBuf),

    #[error("`{0}` is invalid: {1}")]
    InvalidPath(PathBuf, String),

    /// The [`AssetFileAction`](crate::types::AssetFileAction) is
    /// incompatible with the path.
    #[error("invalid action: {0}")]
    IncompatibleAction(String),

    /// An error occured in the process of using the
    /// [`AssetBuilder`](crate::project::asset::AssetBuilder).
    #[error("builder errored: {0}")]
    BuilderError(String),
}

// ********************
// *** Users Errors ***
// ********************

#[derive(Serialize, Deserialize, Error, Debug)]
pub enum Users {
    #[error("email `{0}` already exists")]
    DuplicateEmail(String),

    #[error("`{0}` is not a valid email")]
    InvalidEmail(String),
}

// ***************
// *** IoSerde ***
// ***************

#[derive(Serialize, Deserialize, Error, PartialEq, Clone, Debug)]
pub enum IoSerde {
    #[error("{0:?}")]
    Io(#[serde(with = "IoErrorKind")] io::ErrorKind),

    #[error("{0}")]
    Serde(String),
}

impl From<io::ErrorKind> for IoSerde {
    fn from(value: io::ErrorKind) -> Self {
        Self::Io(value)
    }
}

impl From<io::Error> for IoSerde {
    fn from(value: io::Error) -> Self {
        Self::Io(value.kind())
    }
}

impl From<serde_json::Error> for IoSerde {
    fn from(value: serde_json::Error) -> Self {
        if let Some(kind) = value.io_error_kind() {
            Self::Io(kind)
        } else {
            Self::Serde(value.to_string())
        }
    }
}

// *******************
// *** Local Error ***
// *******************

#[derive(Serialize, Deserialize, Error, Debug, derive_more::From)]
pub enum Error {
    #[error("{0}")]
    Core(CoreError),

    #[from(ignore)]
    #[error("{0}")]
    InvalidPath(PathBuf),

    #[error("{0}")]
    SettingsValidationError(SettingsValidationError),

    #[error("{0}")]
    Users(Users),

    #[error("{0}")]
    Io(#[serde(with = "IoErrorKind")] io::ErrorKind),

    #[error("{0}")]
    IoSerde(IoSerde),

    #[error("{0}")]
    AssetError(AssetError),

    #[error("{0}")]
    ContainerError(ContainerError),

    #[cfg(feature = "fs")]
    #[error("{0}")]
    Project(Project),

    #[cfg(feature = "fs")]
    #[error("{0}")]
    SettingsFileError(SettingsFileError),

    /// Could not load the resource correctly from disk.
    #[cfg(feature = "fs")]
    #[error("Load")]
    Load,

    #[cfg(feature = "fs")]
    #[error("Load")]
    ContainerBuild(crate::project::container::error::Build),
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value.kind())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::IoSerde(err.into())
    }
}

#[cfg(feature = "fs")]
impl From<crate::project::resources::project::LoadError> for Error {
    fn from(_value: crate::project::resources::project::LoadError) -> Self {
        Self::Load
    }
}

// *******************
// *** Syre Result ***
// *******************

pub type Result<T = ()> = StdResult<T, Error>;

impl From<Error> for Result {
    fn from(err: Error) -> Self {
        Err(err)
    }
}

pub mod io_error_kind {
    use serde::{Deserialize, Serialize};
    use std::io;

    /// Copy of [`io::ErrorKind`] for `serde` de/serialization.
    #[non_exhaustive]
    #[derive(Serialize, Deserialize, PartialEq)]
    #[serde(remote = "io::ErrorKind")]
    pub enum IoErrorKind {
        NotFound,
        PermissionDenied,
        ConnectionRefused,
        ConnectionReset,
        HostUnreachable,
        NetworkUnreachable,
        ConnectionAborted,
        NotConnected,
        AddrInUse,
        AddrNotAvailable,
        NetworkDown,
        BrokenPipe,
        AlreadyExists,
        WouldBlock,
        NotADirectory,
        IsADirectory,
        DirectoryNotEmpty,
        ReadOnlyFilesystem,
        FilesystemLoop,
        StaleNetworkFileHandle,
        InvalidInput,
        InvalidData,
        TimedOut,
        WriteZero,
        StorageFull,
        NotSeekable,
        FilesystemQuotaExceeded,
        FileTooLarge,
        ResourceBusy,
        ExecutableFileBusy,
        Deadlock,
        CrossesDevices,
        TooManyLinks,
        InvalidFilename,
        ArgumentListTooLong,
        Interrupted,
        Unsupported,
        UnexpectedEof,
        OutOfMemory,
        Other,
    }

    impl serde_with::SerializeAs<io::ErrorKind> for IoErrorKind {
        fn serialize_as<S>(value: &io::ErrorKind, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            IoErrorKind::serialize(value, serializer)
        }
    }

    impl<'de> serde_with::DeserializeAs<'de, io::ErrorKind> for IoErrorKind {
        fn deserialize_as<D>(deserializer: D) -> Result<io::ErrorKind, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            IoErrorKind::deserialize(deserializer)
        }
    }
}
