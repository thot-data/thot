//! Projects collection.
use crate::error::IoSerde;
use crate::file_resource::SystemResource;
use crate::system::common::config_dir_path;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

#[derive(Deserialize, Serialize, Debug, derive_more::Deref, derive_more::DerefMut)]
#[serde(transparent)]
pub struct ProjectManifest {
    #[deref]
    #[deref_mut]
    inner: Vec<PathBuf>,

    /// Path to the project manifest file.
    #[serde(skip)]
    path: PathBuf,
}

impl ProjectManifest {
    const FILE_NAME: &'static str = "project_manifest.json";

    pub fn load() -> Result<Self, IoSerde> {
        let path = Self::default_path()?;
        let file = fs::File::open(&path)?;
        let reader = BufReader::new(file);
        Ok(Self {
            inner: serde_json::from_reader(reader)?,
            path,
        })
    }

    pub fn load_or_default() -> Result<Self, IoSerde> {
        let path = Self::default_path()?;
        match fs::File::open(&path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                Ok(Self {
                    inner: serde_json::from_reader(reader)?,
                    path,
                })
            }

            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Self {
                inner: vec![],
                path,
            }),

            Err(err) => Err(err.into()),
        }
    }

    pub fn save(&self) -> Result<(), io::Error> {
        let path = self.path();
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(&path, serde_json::to_string_pretty(&self).unwrap())?;
        Ok(())
    }

    pub fn push(&mut self, project: PathBuf) {
        if !self.contains(&project) {
            self.inner.push(project);
        }
    }

    pub fn remove(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        self.retain(|project| {
            let project: &Path = project.as_ref();
            project != path
        });
    }
}

impl ProjectManifest {
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
                inner: vec![],
                path,
            }),
            Err(err) => Err(err.into()),
        }
    }

    /// Saves the manifest to the path is was loaded from.
    pub fn save_to(&self) -> Result<(), IoSerde> {
        fs::write(&self.path, serde_json::to_string_pretty(&self)?)?;
        Ok(())
    }

    /// Consumes `self`, returning the underlying `Vec`.
    pub fn to_vec(self) -> Vec<PathBuf> {
        self.inner
    }
}

impl SystemResource<Vec<PathBuf>> for ProjectManifest {
    fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Returns the path to the system settings file that was loaded.
    fn default_path() -> Result<PathBuf, io::Error> {
        Ok(config_dir_path()?.join(Self::FILE_NAME))
    }
}
