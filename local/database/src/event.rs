//! Database update messages.
//!
//! Topic should be `project:` followed by the resource id of the affected project, if known,
//! otherwise `unknown`.
//! e.g. `project/123-4567-890`, `project/unknown`
use crate::state;
use serde::{Deserialize, Serialize};
use std::{ffi::OsString, path::PathBuf};
use syre_core::{project::Project as CoreProject, system::User, types::ResourceId};
use syre_local::{
    error::IoSerde,
    project::config::{ContainerSettings, Settings, StoredContainerProperties},
    system::resources::Config as ConfigData,
};
use uuid::Uuid;

/// Update types.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Update {
    /// Id of the update.
    id: Uuid,

    /// Id of the event that triggered the update.
    parent: Uuid,

    kind: UpdateKind,
}

impl Update {
    /// # Arguments
    /// 1. `id`: Project id.
    /// 2. `path`: Project base path.
    /// 3. `update`
    /// 4. `parent`: Event's parent event id.
    pub fn project(
        id: Option<ResourceId>,
        path: impl Into<PathBuf>,
        update: Project,
        parent: Uuid,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            parent,
            kind: UpdateKind::Project {
                project: id,
                path: path.into(),
                update,
            },
        }
    }

    /// # Arguments
    /// 1. `id`: Project id.
    /// 2. `path`: Project base path.
    /// 3. `update`
    /// 4. `parent`: Event's parent event id.
    pub fn project_with_id(
        id: ResourceId,
        path: impl Into<PathBuf>,
        update: Project,
        parent: Uuid,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            parent,
            kind: UpdateKind::Project {
                project: Some(id),
                path: path.into(),
                update,
            },
        }
    }

    /// # Arguments
    /// 1. `path`: Project base path.
    /// 2. `update`
    /// 3. `parent`: Event's parent event id.
    pub fn project_no_id(path: impl Into<PathBuf>, update: Project, parent: Uuid) -> Self {
        Self {
            id: Uuid::now_v7(),
            parent,
            kind: UpdateKind::Project {
                project: None,
                path: path.into(),
                update,
            },
        }
    }

    /// # Arguments
    /// 1. `update`
    /// 2. `parent`: Event's parent event id.
    pub fn app(update: impl Into<App>, parent: Uuid) -> Self {
        Self {
            id: Uuid::now_v7(),
            parent,
            kind: UpdateKind::App(update.into()),
        }
    }
}

impl Update {
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn kind(&self) -> &UpdateKind {
        &self.kind
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum UpdateKind {
    App(App),
    Project {
        /// Project id.
        project: Option<ResourceId>,

        /// Project base path.
        path: PathBuf,
        update: Project,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, derive_more::From)]
pub enum App {
    UserManifest(UserManifest),
    ProjectManifest(ProjectManifest),
    LocalConfig(LocalConfig),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum UserManifest {
    /// The manifest became readable.
    Ok(Vec<User>),

    // The manifest became unreadable.
    Error,

    /// Users were added to the manifest.
    Added(Vec<User>),

    /// Users were removed from the manifest
    Removed(Vec<ResourceId>),

    /// User properties were updated.
    Updated(Vec<User>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ProjectManifest {
    /// Paths were added to the manifest.
    Added(Vec<PathBuf>),
    /// Paths were removed from the manifest.
    Removed(Vec<PathBuf>),

    /// The manifest was repaired.
    Repaired,

    /// The manifest can not be loaded.
    Corrupted,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum LocalConfig {
    /// The config became readable.
    Ok(ConfigData),

    /// The config became unreadable.
    Error,

    /// Config settings were changed.
    Updated,
}

#[derive(Serialize, Deserialize, Debug, Clone, derive_more::From)]
pub enum Project {
    /// The base folder of the project was removed.
    FolderRemoved,
    Moved(PathBuf),
    Properties(DataResource<CoreProject>),
    Settings(DataResource<Settings>),
    Analyses(DataResource<Vec<state::Analysis>>),

    #[from]
    Graph(Graph),

    #[from]
    Container {
        /// Absolute path from the data root.
        ///
        /// # Notes
        /// Root container's path is `/`.
        path: PathBuf,
        update: Container,
    },

    /// Events assocated with a tracked asset.
    Asset {
        /// Absolute path from the data root.
        ///
        /// # Notes
        /// Root container's path is `/`.
        container: PathBuf,
        asset: ResourceId,
        update: Asset,
    },

    /// Events associated with files not currently tracked as an asset.
    #[from]
    AssetFile(AssetFile),

    #[from]
    AnalysisFile(AnalysisFile),

    Flag {
        resource: ResourceId,
        message: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Graph {
    /// The root graph was created.
    Created(state::Graph),

    /// A subgraph was created.
    Inserted {
        /// Absolute path from the project's data root to the parent container.
        /// i.e. The root path is the data root container.
        parent: PathBuf,
        graph: state::Graph,
    },

    /// A subgraph root was renamed.
    ///
    /// # Fields
    /// + `from`: Absolute path from the the data root.
    /// + `to`: New name.
    ///
    /// # Notes
    /// + The parent container of the root has not changed.
    Renamed {
        from: PathBuf,

        #[serde(with = "crate::serde_os_string")]
        to: OsString,
    },

    /// A subgraph was moved within the `Project`.
    ///
    /// # Fields
    /// Paths are absolute from the the data root.
    ///
    /// # Notes
    /// + The parent container of the root changed.
    Moved { from: PathBuf, to: PathBuf },

    /// The subgraph at the path was removed.
    /// Path is absolute from the project's data root.
    Removed(PathBuf),
}

/// Container updates.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Container {
    /// `Container`'s properties were modified.
    Properties(DataResource<StoredContainerProperties>),
    Settings(DataResource<ContainerSettings>),
    Assets(DataResource<Vec<state::Asset>>),
}

/// Asset state updates.
/// Indicates the associated file is being tracked as an asset.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Asset {
    FileCreated,
    FileRemoved,
    Properties(state::Asset),
}

/// Asset file updates.
/// Indicates the file is not associated with an asset.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AssetFile {
    Created(
        /// Absolute path from the project's data root.
        PathBuf,
    ),

    Removed(
        /// Absolute path from the project's data root.
        PathBuf,
    ),

    /// File name changed, but parent directory remained the same.
    ///
    /// # Fields
    /// + `from`: Absolute path from the project's data root.
    /// + `to`: New file name.
    Renamed {
        from: PathBuf,

        #[serde(with = "crate::serde_os_string")]
        to: OsString,
    },

    /// File changed locations.
    ///
    /// # Fields
    /// Paths are absolute from the project's data root.
    Moved { from: PathBuf, to: PathBuf },
}

/// Analysis updates.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AnalysisFile {
    Created(PathBuf),
    Removed(PathBuf),

    /// An `Analysis`'s path changed.
    ///
    /// # Notes
    /// + The `Analysis` remains in the same project.
    Moved {
        analysis: ResourceId,
        path: PathBuf,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum DataResource<T> {
    Created(Result<T, IoSerde>),
    Removed,
    Corrupted(IoSerde),
    Repaired(T),
    Modified(T),
}
