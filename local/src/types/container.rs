use crate::common;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};
use syre_core::{
    project::{AnalysisAssociation, Asset, Container, ContainerProperties},
    types::{ResourceId, ResourceMap, UserId, UserPermissions},
};

/// Properties for a Container.
#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
pub struct StoredProperties {
    pub rid: ResourceId,
    pub properties: ContainerProperties,
    pub analyses: Vec<AnalysisAssociation>,
}

impl From<Container> for StoredProperties {
    fn from(container: Container) -> Self {
        Self {
            rid: container.rid().clone(),
            properties: container.properties,
            analyses: container.analyses,
        }
    }
}

/// Settings for a Container
#[derive(PartialEq, Serialize, Deserialize, Clone, Default, Debug)]
pub struct Settings {
    pub creator: Option<UserId>,
    pub created: DateTime<Utc>,
    pub permissions: ResourceMap<UserPermissions>,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            creator: None,
            created: Utc::now(),
            permissions: ResourceMap::new(),
        }
    }
}

/// Container assets.
#[derive(
    derive_more::From,
    derive_more::Deref,
    derive_more::DerefMut,
    Serialize,
    Deserialize,
    Clone,
    Default,
    Debug,
)]
#[serde(transparent)]
pub struct Assets(Vec<Asset>);

impl Assets {
    pub fn into_inner(self) -> Vec<Asset> {
        self.0
    }

    /// Save the container properties.
    ///
    /// # Arguments
    /// 1. `base_path`: Base path of the container the properties represent.
    pub fn save(&self, base_path: impl AsRef<Path>) -> Result<(), io::Error> {
        let path = common::assets_file_of(base_path);
        fs::create_dir_all(path.parent().expect("invalid Container path"))?;
        fs::write(path, serde_json::to_string_pretty(self).unwrap())?;
        Ok(())
    }
}
