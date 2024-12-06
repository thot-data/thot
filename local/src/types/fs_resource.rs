//! File actions for an [`Asset`](syre_core::project::Asset).
use crate::Result;
use serde::{Deserialize, Serialize};
use syre_core::Error as CoreError;

/// How a file system resource's file system resource should be handled when created.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum FsResourceAction {
    /// Move original fs resource to new location.
    Move,

    /// Copy fs resource into location, leaving original in place.
    Copy,

    /// Reference original fs resource, do not move.
    Reference,
}

impl Default for FsResourceAction {
    fn default() -> Self {
        Self::Copy
    }
}

impl Into<String> for FsResourceAction {
    fn into(self) -> String {
        match self {
            FsResourceAction::Move => "Move".to_string(),
            FsResourceAction::Copy => "Copy".to_string(),
            FsResourceAction::Reference => "Reference".to_string(),
        }
    }
}

impl FsResourceAction {
    pub fn from_string(action: String) -> Result<Self> {
        let action = action.to_lowercase();

        match action.as_str() {
            "move" => Ok(FsResourceAction::Move),
            "copy" => Ok(FsResourceAction::Copy),
            "reference" => Ok(FsResourceAction::Reference),
            _ => Err(CoreError::value("invalid `FsResourceAction` string").into()),
        }
    }
}
