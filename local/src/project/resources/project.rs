//! Project and project settings.
use super::super::config::Settings;
use crate::{common, error::IoSerde as IoSerdeError, file_resource::LocalResource};
use std::{
    fs,
    io::{self, BufReader, Write},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};
use syre_core::project::Project as CoreProject;

/// Represents a Syre project.
pub struct Project {
    inner: CoreProject,
    base_path: PathBuf,
    settings: Settings,
}

impl Project {
    /// Create a new `Project` with the given path.
    /// Name of the `Project` is taken from the last component of the path.
    ///
    /// # Errors
    /// + `io::ErrorKind::InvalidFilename`: If file name can not be extracted
    /// from path.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, io::Error> {
        let path = path.into();
        let Some(name) = path.file_name() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidFilename,
                "file name could not be extracted from path",
            )
            .into());
        };

        let Some(name) = name.to_str() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidFilename,
                "file name could not be converted to string",
            )
            .into());
        };

        let name = name.to_string();
        Ok(Self {
            base_path: path,
            inner: CoreProject::new(name),
            settings: Settings::new(),
        })
    }

    /// Create a new local project from the provided project.
    pub fn from(path: impl Into<PathBuf>, project: CoreProject) -> Self {
        Self {
            base_path: path.into(),
            inner: project,
            settings: Settings::new(),
        }
    }

    pub fn load_from(base_path: impl Into<PathBuf>) -> Result<Self, LoadError> {
        let Ok(base_path) = fs::canonicalize(base_path.into()) else {
            return Err(LoadError {
                properties: Err(io::ErrorKind::NotFound.into()),
                settings: Err(io::ErrorKind::NotFound.into()),
            });
        };

        let project = 'project: {
            let path = base_path.join(<Project as LocalResource<CoreProject>>::rel_path());
            let file = match fs::File::open(path) {
                Ok(file) => file,
                Err(err) => break 'project Err(err.into()),
            };

            let reader = BufReader::new(file);
            serde_json::from_reader(reader).map_err(|err| err.into())
        };

        let settings = 'settings: {
            let path = base_path.join(<Project as LocalResource<Settings>>::rel_path());
            let file = match fs::File::open(path) {
                Ok(file) => file,
                Err(err) => break 'settings Err(err.into()),
            };

            let reader = BufReader::new(file);
            serde_json::from_reader(reader).map_err(|err| err.into())
        };

        match (project, settings) {
            (Ok(project), Ok(settings)) => Ok(Self {
                base_path,
                inner: project,
                settings,
            }),

            (project, settings) => Err(LoadError {
                properties: project,
                settings,
            }),
        }
    }

    /// Save all data.
    pub fn save(&self) -> Result<(), io::Error> {
        let project_path = <Project as LocalResource<CoreProject>>::path(self);
        let settings_path = <Project as LocalResource<Settings>>::path(self);
        let Some(parent) = project_path.parent() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidFilename,
                "project path does not have a parent",
            )
            .into());
        };

        fs::create_dir_all(parent)?;

        #[cfg(target_os = "windows")]
        if let Err(err) = common::fs::hide_folder(parent) {
            tracing::error!(?err);
        };

        fs::write(
            project_path,
            serde_json::to_string_pretty(&self.inner).unwrap(),
        )?;
        fs::write(
            settings_path,
            serde_json::to_string_pretty(&self.settings).unwrap(),
        )?;

        Ok(())
    }

    pub fn properties(&self) -> &CoreProject {
        &self.inner
    }

    pub fn properties_mut(&mut self) -> &mut CoreProject {
        &mut self.inner
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }

    pub fn base_path(&self) -> &Path {
        self.base_path.as_path()
    }

    pub fn set_base_path(&mut self, path: impl Into<PathBuf>) {
        self.base_path = path.into();
    }

    /// Get the full path of the data root.
    pub fn data_root_path(&self) -> PathBuf {
        self.base_path.join(&self.data_root)
    }

    /// Get the full path of the analysis root.
    pub fn analysis_root_path(&self) -> Option<PathBuf> {
        let Some(analysis_root) = self.analysis_root.as_ref() else {
            return None;
        };

        Some(self.base_path.join(analysis_root))
    }

    /// Breaks self into parts.
    ///
    /// # Returns
    /// Tuple of (properties, settings, base path).
    pub fn into_parts(self) -> (CoreProject, Settings, PathBuf) {
        let Self {
            inner,
            base_path,
            settings,
        } = self;

        (inner, settings, base_path)
    }
}

impl Project {
    /// Only load the project's properties.
    pub fn load_from_properties_only(
        base_path: impl Into<PathBuf>,
    ) -> Result<CoreProject, IoSerdeError> {
        let base_path = fs::canonicalize(base_path.into())?;
        let path = base_path.join(<Project as LocalResource<CoreProject>>::rel_path());
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader(reader)?)
    }

    pub fn save_properties_only(
        base_path: impl AsRef<Path>,
        properties: &CoreProject,
    ) -> Result<(), io::Error> {
        let path = common::project_file_of(base_path);
        let mut file = fs::File::options().write(true).truncate(true).open(path)?;
        file.write(serde_json::to_string_pretty(properties).unwrap().as_bytes())?;
        Ok(())
    }

    /// Only load the project's settings.
    pub fn load_from_settings_only(
        base_path: impl Into<PathBuf>,
    ) -> Result<Settings, IoSerdeError> {
        let base_path = fs::canonicalize(base_path.into())?;
        let path = base_path.join(<Project as LocalResource<Settings>>::rel_path());
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader(reader)?)
    }
}

impl Deref for Project {
    type Target = CoreProject;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Project {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Into<CoreProject> for Project {
    fn into(self: Self) -> CoreProject {
        self.inner
    }
}

impl LocalResource<CoreProject> for Project {
    fn rel_path() -> PathBuf {
        common::project_file()
    }

    fn base_path(&self) -> &Path {
        &self.base_path
    }
}

impl LocalResource<Settings> for Project {
    fn rel_path() -> PathBuf {
        common::project_settings_file()
    }

    fn base_path(&self) -> &Path {
        &self.base_path
    }
}

pub struct Builder {
    base_path: PathBuf,
    properties: Option<CoreProject>,
    settings: Option<Settings>,
}

impl Builder {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            properties: None,
            settings: None,
        }
    }

    pub fn with_properties(&mut self, properties: CoreProject) {
        let _ = self.properties.insert(properties);
    }

    pub fn build(self) -> Result<Project, io::Error> {
        let Self {
            base_path,
            properties,
            settings,
        } = self;

        let mut project = Project::new(base_path)?;
        if let Some(CoreProject {
            name,
            description,
            data_root,
            analysis_root,
            meta_level,
            ..
        }) = properties
        {
            project.properties_mut().name = name;
            project.properties_mut().description = description;
            project.properties_mut().data_root = data_root;
            project.properties_mut().analysis_root = analysis_root;
            project.properties_mut().meta_level = meta_level;
        }

     
        Ok(project)
    }
}

#[derive(PartialEq, Debug)]
pub struct LoadError {
    pub properties: Result<CoreProject, IoSerdeError>,
    pub settings: Result<Settings, IoSerdeError>,
}
