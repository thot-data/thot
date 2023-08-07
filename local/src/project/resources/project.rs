//! Project and project settings.
use crate::common::{project_file, project_settings_file};
use crate::file_resource::LocalResource;
use crate::types::ProjectSettings;
use crate::Result;
use std::fs;
use std::io::BufReader;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use thot_core::project::Project as CoreProject;

/// Represents a Thot project.
pub struct Project {
    base_path: PathBuf,
    project: CoreProject,
    settings: ProjectSettings,
}

impl Project {
    pub fn load_from(base_path: impl Into<PathBuf>) -> Result<Self> {
        let base_path = base_path.into();
        let project_path = base_path.join(<Project as LocalResource<CoreProject>>::rel_path());
        let settings_path = base_path.join(<Project as LocalResource<ProjectSettings>>::rel_path());

        let project_file = fs::File::open(project_path)?;
        let settings_file = fs::File::open(settings_path)?;

        let project_reader = BufReader::new(project_file);
        let settings_reader = BufReader::new(settings_file);

        let project = serde_json::from_reader(project_reader)?;
        let settings = serde_json::from_reader(settings_reader)?;

        Ok(Self {
            base_path,
            project,
            settings,
        })
    }

    /// Save all data.
    pub fn save(&self) -> Result {
        let project_path = <Project as LocalResource<CoreProject>>::path(self);
        let settings_path = <Project as LocalResource<ProjectSettings>>::path(self);

        let project_file = fs::OpenOptions::new().write(true).open(project_path)?;
        let settings_file = fs::OpenOptions::new().write(true).open(settings_path)?;

        serde_json::to_writer_pretty(project_file, &self.project)?;
        serde_json::to_writer_pretty(settings_file, &self.settings)?;
        Ok(())
    }

    pub fn settings(&self) -> &ProjectSettings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut ProjectSettings {
        &mut self.settings
    }

    pub fn base_path(&self) -> &Path {
        self.base_path.as_path()
    }
}

impl Deref for Project {
    type Target = CoreProject;

    fn deref(&self) -> &Self::Target {
        &self.project
    }
}

impl DerefMut for Project {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.project
    }
}

impl Into<CoreProject> for Project {
    fn into(self: Self) -> CoreProject {
        self.project
    }
}

impl LocalResource<CoreProject> for Project {
    fn rel_path() -> PathBuf {
        project_file()
    }

    fn base_path(&self) -> &Path {
        &self.base_path
    }
}

impl LocalResource<ProjectSettings> for Project {
    fn rel_path() -> PathBuf {
        project_settings_file()
    }

    fn base_path(&self) -> &Path {
        &self.base_path
    }
}

#[cfg(test)]
#[path = "./project_test.rs"]
mod project_test;
