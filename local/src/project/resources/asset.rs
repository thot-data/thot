/// Asset and Assets.
use crate::{common, error::IoSerde, file_resource::LocalResource, system::config::Config, Result};
use std::{
    fs, io,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    result::Result as StdResult,
};
use syre_core::{
    project::{Asset as CoreAsset, AssetProperties as CoreAssetProperties},
    types::{Creator, UserId},
};

pub struct AssetProperties;
impl AssetProperties {
    /// Creates a new [`AssetProperties`](CoreAssetProperties) with fields actively filled from system settings.
    pub fn new() -> Result<CoreAssetProperties> {
        let settings = Config::load()?;
        let creator = match settings.user.as_ref() {
            Some(uid) => Some(UserId::Id(uid.clone().into())),
            None => None,
        };

        let creator = Creator::User(creator);
        let mut props = CoreAssetProperties::new();
        props.creator = creator;

        Ok(props)
    }
}

pub struct Asset;
impl Asset {
    /// Creates an [Asset](CoreAsset) with the `properties` field filled actively from
    /// [`LocalStandardProperties`].
    pub fn new(path: impl Into<PathBuf>) -> Result<CoreAsset> {
        let properties = AssetProperties::new()?;
        Ok(CoreAsset::with_properties(path, properties))
    }
}

/// Assets for a given [`Container`].
///
/// # Notes
/// + A [`Container`] may only reference a file in a single [`Asset`].
/// This functionality is enforced in the `insert_asset` method, which
/// should be prefered over `insert`.
pub struct Assets {
    base_path: PathBuf,
    assets: Vec<CoreAsset>,
}

impl Assets {
    pub fn load_from(base_path: impl Into<PathBuf>) -> StdResult<Self, IoSerde> {
        let base_path = base_path.into();
        let path = base_path.join(Self::rel_path());
        let file = fs::File::open(path)?;
        let reader = io::BufReader::new(file);
        let assets = serde_json::from_reader(reader)?;

        Ok(Self { base_path, assets })
    }

    pub fn save(&self) -> StdResult<(), io::Error> {
        let file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(self.path())?;
        Ok(serde_json::to_writer_pretty(file, &self.assets).unwrap())
    }
}

impl Deref for Assets {
    type Target = Vec<CoreAsset>;
    fn deref(&self) -> &Self::Target {
        &self.assets
    }
}

impl DerefMut for Assets {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.assets
    }
}

impl LocalResource<Vec<CoreAsset>> for Assets {
    fn rel_path() -> PathBuf {
        common::assets_file()
    }

    fn base_path(&self) -> &Path {
        &self.base_path
    }
}
