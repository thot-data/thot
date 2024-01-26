//! Resources for common commands.
use super::types::ResourcePropertiesUpdate;
use crate::common::invoke_result;
use serde::Serialize;
use std::path::PathBuf;
use thot_core::types::ResourceId;
use thot_local_database::command::{
    asset::PropertiesUpdate as AssetPropertiesUpdate,
    container::PropertiesUpdate as ContainerPropertiesUpdate,
};

pub async fn open_file(path: PathBuf) -> Result<(), String> {
    invoke_result("open_file", PathBufArgs { path }).await
}

/// Used for functions that do not accept arguments.
#[derive(Serialize)]
pub struct EmptyArgs {}

/// Used for functions that require a [`ResourceId`] named `rid` as its only argument.
#[derive(Serialize)]
pub struct ResourceIdArgs {
    pub rid: ResourceId,
}

/// Used for functions that require a [`PathBuf`] named `path` as its only argument.
#[derive(Serialize)]
pub struct PathBufArgs {
    /// Path to the project root.
    pub path: PathBuf,
}

/// Bulk update resources.
#[derive(Clone, Serialize)]
pub struct BulkUpdateResourcePropertiesArgs {
    pub rids: Vec<ResourceId>,
    pub update: ResourcePropertiesUpdate,
}

impl From<ResourcePropertiesUpdate> for ContainerPropertiesUpdate {
    fn from(update: ResourcePropertiesUpdate) -> Self {
        Self {
            name: update.name,
            kind: update.kind,
            description: update.description,
            tags: update.tags,
            metadata: update.metadata,
        }
    }
}

impl From<ResourcePropertiesUpdate> for AssetPropertiesUpdate {
    fn from(update: ResourcePropertiesUpdate) -> Self {
        Self {
            name: update.name.map(|name| Some(name)),
            kind: update.kind,
            description: update.description,
            tags: update.tags,
            metadata: update.metadata,
        }
    }
}
