//! App state types.
// NB: All mutation functions are implemented in `server::state`.
use serde::{Deserialize, Serialize};
use std::{ffi::OsString, ops::Deref, path::PathBuf};
use syre_core::{
    project::{
        AnalysisAssociation, Asset as CoreAsset, Container as CoreContainer, ContainerProperties,
        Project as CoreProject,
    },
    types::ResourceId,
};
use syre_local::{
    error::IoSerde,
    types::{AnalysisKind, ContainerSettings, ProjectSettings, StoredContainerProperties},
};

pub type ManifestState<T> = Result<Vec<T>, IoSerde>;
pub type ConfigState<T> = Result<T, IoSerde>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Project {
    pub(crate) path: PathBuf,
    pub(crate) fs_resource: FolderResource<ProjectData>,
}

impl Project {
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn fs_resource(&self) -> &FolderResource<ProjectData> {
        &self.fs_resource
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProjectData {
    pub(crate) properties: DataResource<CoreProject>,
    pub(crate) settings: DataResource<ProjectSettings>,
    pub(crate) analyses: DataResource<Vec<Analysis>>,
}

impl ProjectData {
    pub fn properties(&self) -> DataResource<&CoreProject> {
        self.properties.as_ref().map_err(|err| err.clone())
    }

    pub fn settings(&self) -> DataResource<&ProjectSettings> {
        self.settings.as_ref().map_err(|err| err.clone())
    }

    pub fn analyses(&self) -> DataResource<&Vec<Analysis>> {
        self.analyses.as_ref().map_err(|err| err.clone())
    }
}

#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
pub struct Analysis {
    pub(crate) properties: AnalysisKind,
    pub(crate) fs_resource: FileResource,
}

impl Analysis {
    pub fn properties(&self) -> &AnalysisKind {
        &self.properties
    }

    pub fn fs_resource(&self) -> &FileResource {
        &self.fs_resource
    }

    pub fn is_present(&self) -> bool {
        matches!(self.fs_resource, FileResource::Present)
    }
}

impl Deref for Analysis {
    type Target = AnalysisKind;
    fn deref(&self) -> &Self::Target {
        &self.properties
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Container {
    /// Name of the container's folder.
    #[serde(with = "crate::serde_os_string")]
    pub(crate) name: OsString,
    pub(crate) properties: DataResource<StoredContainerProperties>,
    pub(crate) settings: DataResource<ContainerSettings>,
    pub(crate) assets: DataResource<Vec<Asset>>,
}

impl Container {
    pub fn name(&self) -> &OsString {
        &self.name
    }

    pub fn rid(&self) -> DataResource<&ResourceId> {
        self.properties
            .as_ref()
            .map(|props| &props.rid)
            .map_err(|err| err.clone())
    }

    pub fn properties(&self) -> DataResource<&ContainerProperties> {
        self.properties
            .as_ref()
            .map(|props| &props.properties)
            .map_err(|err| err.clone())
    }

    pub fn settings(&self) -> DataResource<&ContainerSettings> {
        self.settings
            .as_ref()
            .map(|settings| settings)
            .map_err(|err| err.clone())
    }

    pub fn assets(&self) -> DataResource<&Vec<Asset>> {
        self.assets
            .as_ref()
            .map(|assets| assets)
            .map_err(|err| err.clone())
    }

    pub fn analyses(&self) -> DataResource<&Vec<AnalysisAssociation>> {
        self.properties
            .as_ref()
            .map(|props| &props.analyses)
            .map_err(|err| err.clone())
    }

    /// Creates a `[syre_core::project::Container]`.
    ///
    /// # Returns
    /// `Some` if all states are valid, `None` otherwise.
    pub fn as_container(&self) -> Option<CoreContainer> {
        let DataResource::Ok(rid) = self.rid() else {
            return None;
        };
        let DataResource::Ok(properties) = self.properties() else {
            return None;
        };
        let DataResource::Ok(assets) = self.assets() else {
            return None;
        };
        let DataResource::Ok(analyses) = self.analyses() else {
            return None;
        };

        let assets = assets
            .into_iter()
            .map(|asset| asset.properties.clone())
            .collect();
        
        Some(CoreContainer::from_parts(
            rid.clone(),
            properties.clone(),
            assets,
            analyses.clone(),
        ))
    }
}

#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
pub struct Asset {
    pub(crate) properties: CoreAsset,
    pub(crate) fs_resource: FileResource,
}

impl Asset {
    pub fn is_present(&self) -> bool {
        match self.fs_resource {
            FileResource::Present => true,
            FileResource::Absent => false,
        }
    }
}

impl Deref for Asset {
    type Target = CoreAsset;
    fn deref(&self) -> &Self::Target {
        &self.properties
    }
}

/// # Notes
/// + Root node is at index 0.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Graph {
    pub nodes: Vec<Container>,

    /// Children of the node at the same index in `nodes`.
    /// Elements are indices of `nodes`.
    pub children: Vec<Vec<usize>>,
}

pub type DataResource<T> = Result<T, IoSerde>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum FolderResource<T> {
    Present(T),
    Absent,
}

impl<T> FolderResource<T> {
    pub fn is_present(&self) -> bool {
        matches!(self, FolderResource::Present(_))
    }
}

impl<T> FolderResource<T> {
    pub fn as_ref(&self) -> FolderResource<&T> {
        match *self {
            Self::Present(ref x) => FolderResource::Present(x),
            Self::Absent => FolderResource::Absent,
        }
    }

    #[track_caller]
    pub fn unwrap(self) -> T {
        if let Self::Present(x) = self {
            x
        } else {
            panic!("called `FolderResource::unwrap` on an `Absent` value");
        }
    }

    pub fn map<U, F>(&self, f: F) -> FolderResource<U>
    where
        F: FnOnce(&T) -> U,
    {
        match self {
            Self::Present(ref x) => FolderResource::Present(f(x)),
            Self::Absent => FolderResource::Absent,
        }
    }

    pub fn or_else<F>(self, f: F) -> FolderResource<T>
    where
        F: FnOnce() -> FolderResource<T>,
    {
        match self {
            x @ Self::Present(_) => x,
            Self::Absent => f(),
        }
    }
}

#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
pub enum FileResource {
    Present,
    Absent,
}

impl FileResource {
    pub fn is_present(&self) -> bool {
        matches!(self, FileResource::Present)
    }
}
