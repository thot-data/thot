//! Container and container settings.
use super::{
    super::config::{ContainerSettings, StoredContainerProperties},
    Analyses,
};
use crate::{
    common,
    error::{Error, Result},
    file_resource::LocalResource,
};
use has_id::HasId;
use std::{
    fs,
    hash::{Hash, Hasher},
    io,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    result::Result as StdResult,
};
use syre_core::{
    error::{Error as CoreError, Resource as ResourceError},
    project::{AnalysisAssociation, Asset, Container as CoreContainer, ContainerProperties},
    types::ResourceId,
};

#[derive(Debug)]
pub struct Container {
    pub(crate) base_path: PathBuf,
    pub inner: CoreContainer,
    pub settings: ContainerSettings,
}

impl Container {
    /// Create a new Container at the given base path.
    ///
    /// # Arguments
    /// 1. Path to the Container.
    ///
    /// # Notes
    /// + No changes or checks are made to the file system.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let name = PathBuf::from(path.clone());
        let name = name.file_name().expect("invalid path");
        let name: String = name.to_string_lossy().to_string();

        Self {
            base_path: path,
            inner: CoreContainer::new(name),
            settings: ContainerSettings::new(),
        }
    }

    /// Save all data.
    pub fn save(&self) -> StdResult<(), error::Save> {
        let properties_path = <Self as LocalResource<StoredContainerProperties>>::path(self);
        let assets_path = <Self as LocalResource<Vec<Asset>>>::path(self);
        let settings_path = <Self as LocalResource<ContainerSettings>>::path(self);

        let app_folder = properties_path.parent().expect("invalid Container path");
        fs::create_dir_all(app_folder).map_err(error::Save::CreateDir)?;

        #[cfg(target_os = "windows")]
        if let Err(err) = common::fs::hide_folder(app_folder) {
            tracing::error!("could not hide folder {app_folder:?}: {err:?}");
        }

        let properties: StoredContainerProperties = self.inner.clone().into();

        let save_properties = fs::write(
            properties_path,
            serde_json::to_string_pretty(&properties).unwrap(),
        );

        let save_assets = fs::write(
            assets_path,
            serde_json::to_string_pretty(&self.assets).unwrap(),
        );

        let save_settings = fs::write(
            settings_path,
            serde_json::to_string_pretty(&self.settings).unwrap(),
        );

        if save_properties.is_err() || save_assets.is_err() || save_settings.is_err() {
            Err(error::Save::SaveFiles {
                properties: save_properties.err(),
                assets: save_assets.err(),
                settings: save_settings.err(),
            })
        } else {
            Ok(())
        }
    }

    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    pub fn set_base_path(&mut self, path: impl Into<PathBuf>) {
        self.base_path = path.into();
    }

    pub fn buckets(&self) -> Vec<PathBuf> {
        self.assets
            .iter()
            .filter_map(|asset| asset.bucket())
            .collect()
    }

    /// Returns if the container is already associated with the analysis with the given id,
    /// regardless of the associations priority or autorun status.
    pub fn contains_analysis_association(&self, rid: &ResourceId) -> bool {
        self.analyses
            .iter()
            .any(|association| association.analysis() == rid)
    }

    /// Adds an association to the Container.
    /// Errors if an association with the analysis already exists.
    ///
    /// # See also
    /// + `set_analysis_association`
    pub fn add_analysis_association(&mut self, association: AnalysisAssociation) -> Result {
        if self.contains_analysis_association(association.analysis()) {
            return Err(Error::Core(CoreError::Resource(
                ResourceError::already_exists("Association with analysis already exists"),
            )));
        }

        self.analyses.push(association);
        Ok(())
    }

    /// Sets or adds an analysis association with the Container.
    ///
    /// # See also
    /// + [`add_analysis_association`]
    pub fn set_analysis_association(&mut self, association: AnalysisAssociation) {
        self.analyses
            .retain(|a| a.analysis() != association.analysis());
        self.analyses.push(association);
    }

    /// Removes an association with the given analysis.
    pub fn remove_analysis_association(&mut self, rid: &ResourceId) {
        self.analyses
            .retain(|association| association.analysis() != rid);
    }

    pub fn settings(&self) -> &ContainerSettings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut ContainerSettings {
        &mut self.settings
    }

    /// Breaks self into parts.
    ///
    /// # Returns
    /// Tuple of (properties, settings, base path).
    pub fn into_parts(self) -> (CoreContainer, ContainerSettings, PathBuf) {
        let Self {
            inner: container,
            base_path,
            settings,
        } = self;

        (container, settings, base_path)
    }
}

impl PartialEq for Container {
    fn eq(&self, other: &Container) -> bool {
        self.inner == other.inner
    }
}

impl Eq for Container {}

impl Deref for Container {
    type Target = CoreContainer;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Container {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Hash for Container {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.rid().hash(state);
    }
}

impl HasId for Container {
    type Id = ResourceId;

    fn id(&self) -> &Self::Id {
        &self.inner.id()
    }
}

impl StoredContainerProperties {
    /// # Arguments
    /// 1. `base_path`: Base path of the container.
    pub fn save(&self, base_path: impl AsRef<Path>) -> StdResult<(), io::Error> {
        let path = common::container_file_of(base_path);
        fs::create_dir_all(path.parent().expect("invalid Container path"))?;
        fs::write(path, serde_json::to_string_pretty(self).unwrap())?;
        Ok(())
    }
}

impl LocalResource<StoredContainerProperties> for Container {
    fn rel_path() -> PathBuf {
        common::container_file()
    }

    fn base_path(&self) -> &Path {
        &self.base_path
    }
}

impl LocalResource<Vec<Asset>> for Container {
    fn rel_path() -> PathBuf {
        common::assets_file()
    }

    fn base_path(&self) -> &Path {
        &self.base_path
    }
}

impl LocalResource<ContainerSettings> for Container {
    fn rel_path() -> PathBuf {
        common::container_settings_file()
    }

    fn base_path(&self) -> &Path {
        &self.base_path
    }
}

pub struct Builder {
    base_path: PathBuf,
    properties: Option<ContainerProperties>,
    analyses: Option<Vec<AnalysisAssociation>>,
    settings: Option<ContainerSettings>,
}

impl Builder {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            properties: None,
            analyses: None,
            settings: None,
        }
    }

    pub fn with_properties(&mut self, properties: ContainerProperties) {
        let _ = self.properties.insert(properties);
    }

    pub fn with_analyses(&mut self, associations: Vec<AnalysisAssociation>) {
        let _ = self.analyses.insert(associations);
    }

    pub fn with_settings(&mut self, settings: ContainerSettings) {
        let _ = self.settings.insert(settings);
    }

    pub fn build(self) -> Container {
        let Builder {
            base_path,
            properties,
            analyses,
            settings,
        } = self;

        let mut container = Container::new(base_path);
        if let Some(properties) = properties {
            container.inner.properties = properties;
        }

        if let Some(associations) = analyses {
            container.inner.analyses = associations;
        }

        if let Some(settings_src) = settings {
            let ContainerSettings {
                creator,
                permissions,
                ..
            } = settings_src;
            let mut settings = ContainerSettings::new();
            settings.creator = creator;
            settings.permissions = permissions;
            container.settings = settings;
        }

        container
    }
}

pub mod error {
    use std::io;

    #[derive(Debug)]
    pub enum Save {
        CreateDir(io::Error),
        SaveFiles {
            properties: Option<io::Error>,
            assets: Option<io::Error>,
            settings: Option<io::Error>,
        },
    }
}

#[cfg(test)]
#[path = "./container_test.rs"]
mod container_test;
