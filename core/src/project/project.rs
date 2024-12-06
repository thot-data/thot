//! Project and project settings.
use crate::types::ResourceId;
use std::path::PathBuf;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Represents a Syre project.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    rid: ResourceId,
    pub name: String,
    pub description: Option<String>,
    pub data_root: PathBuf,
    pub analysis_root: Option<PathBuf>,
    pub meta_level: u16,
}

impl Project {
    /// Creates a new Project.
    ///
    /// # Notes:
    /// + `data_root` defaults to `data`.
    pub fn new(name: impl Into<String>) -> Project {
        Project {
            rid: ResourceId::new(),
            name: name.into(),
            description: None,
            data_root: PathBuf::from("data"),
            analysis_root: None,
            meta_level: 0,
        }
    }

    /// Creates a project with the given id.
    ///  
    /// # Notes:
    /// + `data_root` defaults to `data`.
    pub fn with_id(id: ResourceId, name: impl Into<String>) -> Self {
        Self {
            rid: id,
            name: name.into(),
            description: None,
            data_root: PathBuf::from("data"),
            analysis_root: None,
            meta_level: 0,
        }
    }

    pub fn rid(&self) -> &ResourceId {
        &self.rid
    }

    pub fn set_analysis_root(&mut self, analysis_root: impl Into<PathBuf>) {
        let _ = self.analysis_root.insert(analysis_root.into());
    }

    pub fn clear_analysis_root(&mut self) {
        let _ = self.analysis_root.take();
    }
}
