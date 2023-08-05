//! Collection of templates.
use crate::file_resource::SystemResource;
use crate::system::common::config_dir_path;
use crate::Result;
use std::collections::HashMap;
use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use thot_core::system::template::Project as ProjectTemplate;
use thot_core::types::ResourceId;

pub type TemplateMap = HashMap<ResourceId, ProjectTemplate>;

#[derive(Debug)]
pub struct Templates(TemplateMap);
impl Templates {
    pub fn load() -> Result<Self> {
        let fh = fs::OpenOptions::new().write(true).open(Self::path())?;
        serde_json::from_reader(fh)
    }

    pub fn save(&self) -> Result {
        let fh = fs::OpenOptions::new().write(true).open(Self::path())?;
        serde_json::to_writer_pretty(fh, &self.0)
    }
}

impl SystemResource<TemplateMap> for Templates {
    /// Returns the path to the system settings file.
    fn path() -> PathBuf {
        let settings_dir = config_dir_path().expect("could not get settings directory");
        settings_dir.join("templates.json")
    }
}

impl Deref for Templates {
    type Target = TemplateMap;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Templates {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
