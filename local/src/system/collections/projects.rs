//! Projects collection.
use crate::file_resource::SystemResource;
use crate::system::common::config_dir_path;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, BufReader};
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use thot_core::types::ResourceMap;

/// Map from a [`Project`]'s id to its path.
pub type ProjectMap = ResourceMap<PathBuf>;

#[derive(Deserialize, Serialize, Default, Debug)]
#[serde(transparent)]
pub struct Projects(ProjectMap);

impl Projects {
    pub fn load() -> Result<Self> {
        let file = fs::File::open(Self::path())?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader(reader)?)
    }

    pub fn load_or_default() -> Result<Self> {
        match fs::File::open(Self::path()) {
            Ok(file) => {
                let reader = BufReader::new(file);
                Ok(serde_json::from_reader(reader)?)
            }

            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(err.into()),
        }
    }

    pub fn save(&self) -> Result {
        fs::write(Self::path(), serde_json::to_string_pretty(&self)?)?;
        Ok(())
    }
}

impl Deref for Projects {
    type Target = ProjectMap;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Projects {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl SystemResource<ProjectMap> for Projects {
    /// Returns the path to the system settings file.
    fn path() -> PathBuf {
        let settings_dir = config_dir_path().expect("could not get settings directory");
        settings_dir.join("projects.json")
    }
}

#[cfg(test)]
#[path = "./projects_test.rs"]
mod projects_test;
