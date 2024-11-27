//! Runner settings.
use crate::{error, file_resource::UserResource, system::common::config_dir_path};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, BufReader},
    path::{Path, PathBuf},
};
use syre_core::types::ResourceId;

/// Represents Syre runner settings.
///
/// # Default
/// RunnerSettings::default is derived so does not automatically obtain a file lock.
/// This is done intentionally as it may not reflect the current state of the persisted settings.
/// To obtain the file lock use the `RunnerSettings#acquire_lock` method.
///
/// # Fields
/// + **python_path:** Option for the python binary path.
/// + **r_path:** Option for the r binary path.
#[derive(Serialize, Deserialize, Clone, Default, derive_more::Deref)]
#[serde(transparent)]
pub struct RunnerSettings {
    /// Path to the user's runner settings.
    #[serde(skip)]
    path: PathBuf,

    #[deref]
    inner: Settings,
}

impl RunnerSettings {
    const SETTINGS_DIR: &'static str = "settings";

    pub fn load(user: ResourceId) -> Result<Self, error::IoSerde> {
        let mut path = PathBuf::from(user.to_string());
        path.set_extension("json");

        let path_abs = Self::base_path()?.join(&path);
        let file = fs::File::open(&path_abs)?;
        let reader = BufReader::new(file);
        let inner = serde_json::from_reader(reader)?;
        Ok(Self { path, inner })
    }

    pub fn save(&self) -> Result<(), io::Error> {
        let path = self.path()?;
        let dir = path.parent().unwrap();
        fs::create_dir_all(dir)?;
        fs::write(path, serde_json::to_string_pretty(&self)?)?;
        Ok(())
    }
}

impl UserResource<Settings> for RunnerSettings {
    fn base_path() -> Result<PathBuf, io::Error> {
        let base_path = config_dir_path()?;
        Ok(base_path.join(Self::SETTINGS_DIR))
    }

    fn rel_path(&self) -> &Path {
        self.path.as_path()
    }
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Settings {
    /// Path to python executable runner should use.
    pub python_path: Option<PathBuf>,

    /// Path to R executable runner should use.
    pub r_path: Option<PathBuf>,

    /// Continue or halt analysis when an error occurs.
    pub continue_on_error: bool,
}
