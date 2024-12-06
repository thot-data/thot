//! High level functions associated to the projects list.
use super::collections::project_manifest::ProjectManifest;
use crate::error::IoSerde as IoSerdeError;
use crate::project::resources::Project;
use std::fs;
use std::path::{Path, PathBuf};
use syre_core::types::ResourceId;

/// Adds a [`Project`] to the registry collection.
///
/// # See also
/// + `insert_project`
pub fn register_project(path: impl AsRef<Path>) -> Result<(), IoSerdeError> {
    let path = fs::canonicalize(path.as_ref())?;
    let mut projects = ProjectManifest::load_or_default()?;

    if !projects.contains(&path) {
        projects.push(path);
        projects.save()?;
    }

    Ok(())
}

/// Deregister a [`Project`].
pub fn deregister_project(path: impl AsRef<Path>) -> Result<(), IoSerdeError> {
    let mut projects = ProjectManifest::load()?;
    projects.remove(&path.as_ref());
    projects.save()?;
    Ok(())
}

/// Get the path of a Project.
///
/// # Errors
/// + If the project manifest could not be loaded.
///
/// # Notes
/// + If a Project can not be loaded it is ignored.
pub fn get_path(rid: &ResourceId) -> Result<Option<PathBuf>, IoSerdeError> {
    let project_manifest = ProjectManifest::load()?;
    for path in project_manifest.iter() {
        let Ok(project) = Project::load_from(path) else {
            continue;
        };

        if project.rid() == rid {
            return Ok(Some(path.to_path_buf()));
        }
    }

    Ok(None)
}
