use crate::identifier::Identifier;
use std::{io, path::PathBuf};
use syre_core::identifier::Identifier as CoreIdentifier;

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
