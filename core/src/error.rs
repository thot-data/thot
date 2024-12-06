//! Common error types.
use crate::types::ResourceId;
use std::path::PathBuf;
use std::result::Result as StdResult;
use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "serde")]
pub use serde_category::SerdeCategory;

// **********************
// *** Resource Error ***
// **********************

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Error, Debug)]
pub enum Resource {
    #[error("resource `{0}` does not exist")]
    DoesNotExist(String),

    #[error("id `{0}` already exists")]
    DuplicateId(ResourceId),

    #[error("resource `{0}` already exists")]
    AlreadyExists(String),
}

impl Resource {
    pub fn does_not_exist(msg: impl Into<String>) -> Self {
        Self::DoesNotExist(msg.into())
    }

    pub fn already_exists(msg: impl Into<String>) -> Self {
        Self::AlreadyExists(msg.into())
    }
}

// **********************
// *** Project Error ***
// **********************

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Error, Clone, Debug)]
pub enum Project {
    #[error("Project is not registered")]
    NotRegistered(Option<ResourceId>, Option<PathBuf>),

    #[error("Project is misconfigured: {0}")]
    Misconfigured(String),
}

impl Project {
    pub fn misconfigured(msg: impl Into<String>) -> Self {
        Self::Misconfigured(msg.into())
    }
}

// ******************
// *** GraphError ***
// ******************

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Error, Debug)]
pub enum Graph {
    #[error("invalid graph: {0}")]
    InvalidGraph(String),

    #[error("illegal operation: {0}")]
    IllegalOperation(String),
}

impl Graph {
    pub fn invalid_graph(msg: impl Into<String>) -> Self {
        Self::InvalidGraph(msg.into())
    }
}

// *******************
// *** Asset Error ***
// *******************

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Error, Debug)]
pub enum AssetError {
    #[error("Asset not registered")]
    NotRegistered(Option<ResourceId>, Option<PathBuf>),

    #[error("Asset path is not set")]
    PathNotSet,
}

#[derive(Error, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AnalysisError {
    #[error("unknown language `{0:?}`")]
    UnknownLanguage(Option<String>),
}

// ***************************
// *** Resource Path Error ***
// ***************************

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Error, Debug)]
pub enum ResourcePathError {
    #[error("{0}")]
    CouldNotParseMetalevel(String),
}

impl ResourcePathError {
    pub fn could_not_parse_meta_level(msg: impl Into<String>) -> Self {
        Self::CouldNotParseMetalevel(msg.into())
    }
}

// ******************
// *** Syre Error ***
// ******************

// TODO Put behind correct features.
#[derive(Error, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Error {
    #[error("{0}")]
    Asset(AssetError),

    #[error("{0}")]
    Project(Project),

    #[error("{0}")]
    Resource(Resource),

    #[error("{0}")]
    Graph(Graph),

    #[error("{0}")]
    ResourcePath(ResourcePathError),

    #[error("{0}")]
    Analysis(AnalysisError),

    #[cfg(feature = "serde")]
    #[error("{message} at line {line} column {column}")]
    Serde {
        #[serde(with = "SerdeCategory")]
        category: serde_json::error::Category,
        line: usize,
        column: usize,
        message: String,
    },

    /// Invalid value encountered.
    #[error("{0}")]
    Value(String),
}

impl Error {
    pub fn value(msg: impl Into<String>) -> Self {
        Self::Value(msg.into())
    }
}

#[cfg(feature = "serde")]
impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serde {
            category: err.classify(),
            line: err.line(),
            column: err.column(),
            message: format!("{err:?}"),
        }
    }
}

impl From<Resource> for Error {
    fn from(err: Resource) -> Self {
        Self::Resource(err)
    }
}

impl From<Graph> for Error {
    fn from(err: Graph) -> Self {
        Self::Graph(err)
    }
}

impl From<AnalysisError> for Error {
    fn from(err: AnalysisError) -> Self {
        Self::Analysis(err)
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

#[cfg(feature = "serde")]
pub mod serde_category {
    use serde::{Deserialize, Serialize};

    /// Copy of [`serde_json::error::Category`] for `serde` de/serialization.
    #[derive(Serialize, Deserialize)]
    #[serde(remote = "serde_json::error::Category")]
    pub enum SerdeCategory {
        Io,
        Syntax,
        Data,
        Eof,
    }
}
