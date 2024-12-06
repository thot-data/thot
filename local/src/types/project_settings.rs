use std::{fs, io, path::Path};

use crate::{common, constants::PROJECT_FORMAT_VERSION};
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use syre_core::types::{ResourceMap, UserId, UserPermissions};

/// Settings for a local Project.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ProjectSettings {
    /// Format standard for the Project.
    pub local_format_version: String,
    pub created: DateTime<Utc>,
    pub creator: Option<UserId>,
    pub permissions: ResourceMap<UserPermissions>,
}

impl ProjectSettings {
    pub fn new() -> Self {
        Self {
            local_format_version: PROJECT_FORMAT_VERSION.to_string(),
            created: Utc::now(),
            creator: None,
            permissions: ResourceMap::new(),
        }
    }

    /// # Arguments
    /// 1. `base_path`: Base path of the project.
    pub fn save(&self, base_path: impl AsRef<Path>) -> Result<(), io::Error> {
        let path = common::project_settings_file_of(base_path);
        fs::create_dir_all(path.parent().expect("invalid project path"))?;
        fs::write(path, serde_json::to_string_pretty(self).unwrap())?;
        Ok(())
    }
}
