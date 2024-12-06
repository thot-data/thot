use crate::{
    error::IoSerde,
    file_resource::LocalResource,
    project::resources::container::Container,
    types::{Assets, ContainerSettings, StoredContainerProperties},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use syre_core::project::{Asset, Container as CoreContainer};

/// Loads a [`Container`].
pub struct Loader;
impl Loader {
    pub fn load(base_path: impl AsRef<Path>) -> Result<Container, State> {
        let Ok(base_path) = fs::canonicalize(base_path) else {
            return Err(State::not_found());
        };

        match Self::load_resources(&base_path) {
            State {
                properties: Ok(container),
                settings: Ok(settings),
                assets: Ok(assets),
            } => {
                let container = CoreContainer::from_parts(
                    container.rid,
                    container.properties,
                    assets,
                    container.analyses,
                );

                Ok(Container {
                    base_path,
                    container,
                    settings,
                })
            }

            state => Err(state),
        }
    }

    pub fn load_resources(base_path: impl AsRef<Path>) -> State {
        let base_path = base_path.as_ref();
        let properties_path =
            base_path.join(<Container as LocalResource<StoredContainerProperties>>::rel_path());

        let assets_path = base_path.join(<Container as LocalResource<Vec<Asset>>>::rel_path());
        let settings_path =
            base_path.join(<Container as LocalResource<ContainerSettings>>::rel_path());

        let container = Self::load_json::<StoredContainerProperties>(properties_path);
        let assets = Self::load_json(assets_path);
        let settings = Self::load_json(settings_path);

        State::new(container, settings, assets)
    }

    pub fn load_from_only_properties(
        base_path: impl AsRef<Path>,
    ) -> Result<StoredContainerProperties, IoSerde> {
        let base_path = base_path.as_ref();
        let path =
            base_path.join(<Container as LocalResource<StoredContainerProperties>>::rel_path());

        Ok(Self::load_json::<StoredContainerProperties>(path)?)
    }

    pub fn load_from_only_settings(
        base_path: impl AsRef<Path>,
    ) -> Result<ContainerSettings, IoSerde> {
        let base_path = base_path.as_ref();
        let path = base_path.join(<Container as LocalResource<ContainerSettings>>::rel_path());
        Ok(Self::load_json::<ContainerSettings>(path)?)
    }

    pub fn load_from_only_assets(base_path: impl AsRef<Path>) -> Result<Assets, IoSerde> {
        let base_path = base_path.as_ref();
        let path = base_path.join(<Container as LocalResource<Vec<Asset>>>::rel_path());
        Ok(Self::load_json::<Vec<Asset>>(path)?.into())
    }

    /// Convenience function for loading data from a JSON file.
    fn load_json<T: DeserializeOwned>(path: PathBuf) -> Result<T, IoSerde> {
        let file = fs::File::open(&path)?;
        let reader = io::BufReader::new(file);
        Ok(serde_json::from_reader(reader)?)
    }
}

pub struct AssetValidator;
impl AssetValidator {
    pub fn validate(container: &Container) -> Result<(), Vec<error::AssetFile>> {
        let mut errors = Vec::new();
        for asset in container.assets.iter() {
            let path = container.base_path().join(asset.path.as_path());
            if let Err(err) = fs::canonicalize(path) {
                errors.push(error::AssetFile {
                    asset: asset.rid().clone(),
                    kind: err.kind(),
                });
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub properties: Result<StoredContainerProperties, IoSerde>,
    pub settings: Result<ContainerSettings, IoSerde>,
    pub assets: Result<Vec<Asset>, IoSerde>,
}

impl State {
    pub fn new(
        properties: Result<StoredContainerProperties, IoSerde>,
        settings: Result<ContainerSettings, IoSerde>,
        assets: Result<Vec<Asset>, IoSerde>,
    ) -> Self {
        Self {
            properties,
            settings,
            assets,
        }
    }

    /// Initialize all fields to be not found.
    pub fn not_found() -> Self {
        Self {
            properties: Err(io::ErrorKind::NotFound.into()),
            settings: Err(io::ErrorKind::NotFound.into()),
            assets: Err(io::ErrorKind::NotFound.into()),
        }
    }

    pub fn properties(&self) -> &Result<StoredContainerProperties, IoSerde> {
        &self.properties
    }

    pub fn settings(&self) -> &Result<ContainerSettings, IoSerde> {
        &self.settings
    }

    pub fn assets(&self) -> &Result<Vec<Asset>, IoSerde> {
        &self.assets
    }
}

pub mod error {
    use serde::{Deserialize, Serialize};
    use std::io;
    use syre_core::types::ResourceId;

    #[derive(Serialize, Deserialize, thiserror::Error, Clone, Debug)]
    #[error("file for Asset {asset} {kind:?}")]
    pub struct AssetFile {
        pub(crate) asset: ResourceId,

        #[serde(with = "io_error_serde::ErrorKind")]
        pub(crate) kind: io::ErrorKind,
    }
}
