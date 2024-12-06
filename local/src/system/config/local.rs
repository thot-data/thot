use crate::{
    error::IoSerde,
    file_resource::SystemResource,
    system::{common::config_dir_path, resources::Config as ConfigData},
};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};
use std::{
    fs,
    io::{self, BufReader},
    path::PathBuf,
};

/// User settings.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct Config {
    #[serde(skip)]
    path: PathBuf,
    inner: ConfigData,
}

impl Config {
    pub fn load() -> Result<Self, IoSerde> {
        let path = Self::default_path()?;
        let file = fs::File::open(&path)?;
        let reader = BufReader::new(file);
        let inner = serde_json::from_reader(reader)?;
        Ok(Self { path, inner })
    }

    pub fn load_or_default() -> Result<Self, IoSerde> {
        let path = Self::default_path()?;
        match fs::File::open(&path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                let inner = serde_json::from_reader(reader)?;
                Ok(Self { path, inner })
            }

            Err(_) => Ok(Self {
                path,
                inner: ConfigData::default(),
            }),
        }
    }

    pub fn save(&self) -> Result<(), IoSerde> {
        fs::write(self.path(), serde_json::to_string_pretty(&self)?)?;
        Ok(())
    }
}

impl Config {
    /// Load the manifest from the given path.
    pub fn load_from(path: impl Into<PathBuf>) -> Result<Self, IoSerde> {
        let path = path.into();
        let file = fs::File::open(&path)?;
        let reader = BufReader::new(file);
        Ok(Self {
            inner: serde_json::from_reader(reader)?,
            path,
        })
    }

    /// Load the manifest from the given path or create the default if the file does not exist.
    pub fn load_from_or_default(path: impl Into<PathBuf>) -> Result<Self, IoSerde> {
        let path = path.into();
        match fs::File::open(&path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                Ok(Self {
                    inner: serde_json::from_reader(reader)?,
                    path,
                })
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Self {
                path,
                inner: ConfigData::default(),
            }),
            Err(err) => Err(err.into()),
        }
    }

    /// Saves the manifest to the path is was loaded from.
    pub fn save_to(&self) -> Result<(), IoSerde> {
        fs::write(&self.path, serde_json::to_string_pretty(&self)?)?;
        Ok(())
    }

    /// Consumes self, returning the inner data.
    pub fn to_data(self) -> ConfigData {
        self.inner
    }
}

impl SystemResource<Config> for Config {
    fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Returns the path to the system settings file.
    fn default_path() -> Result<PathBuf, io::Error> {
        Ok(config_dir_path()?.join("local_config.json"))
    }
}

impl Deref for Config {
    type Target = ConfigData;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Config {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
