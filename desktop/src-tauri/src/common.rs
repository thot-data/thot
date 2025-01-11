use crate::{constants::*, identifier::Identifier};
use std::{
    io,
    path::{Path, PathBuf},
};
use syre_core::identifier::Identifier as CoreIdentifier;
use syre_local as local;

/// Returns app config directories for the system user.
pub fn system_dirs() -> Result<directories::ProjectDirs, io::Error> {
    let dirs = directories::ProjectDirs::from(
        &CoreIdentifier::qualifier(),
        &CoreIdentifier::organization(),
        &Identifier::application(),
    );

    match dirs {
        Some(dirs) => Ok(dirs),
        None => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "system settings directory not found",
        )),
    }
}

/// Returns the path to the system user's app config directory.
pub fn config_dir_path() -> Result<PathBuf, io::Error> {
    Ok(system_dirs()?.config_dir().to_path_buf())
}

/// Path to the project desktop settings file for a given path.
pub fn project_desktop_settings_file() -> PathBuf {
    local::common::app_dir().join(PROJECT_DESKTOP_SETTINGS_FILE)
}

/// Path to the project file for a given path.
/// `app_dir_of(path)/\<PROJECT_DESKTOP_SETTINGS_FILE\>`
pub fn project_desktop_settings_file_of(path: impl AsRef<Path>) -> PathBuf {
    local::common::app_dir_of(path).join(PROJECT_DESKTOP_SETTINGS_FILE)
}
