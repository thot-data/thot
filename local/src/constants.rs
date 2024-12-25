//! Crate wide constant values.

/// Current project format version.
pub static PROJECT_FORMAT_VERSION: &str = "0.11.1";

// ***********
// *** app ***
// ***********

pub const APP_DIR: &str = ".syre";
pub const PROJECT_FILE: &str = "project.json";
pub const PROJECT_SETTINGS_FILE: &str = "project_settings.json";
pub const PROJECT_RUNNER_SETTINGS_FILE: &str = "runner_settings.json";
pub const CONTAINER_FILE: &str = "container.json";
pub const CONTAINER_SETTINGS_FILE: &str = "container_settings.json";
pub const ASSETS_FILE: &str = "assets.json";
pub const ANALYSES_FILE: &str = "analyses.json";
pub const IGNORE_FILE: &str = ".syreignore";

// ************
// *** misc ***
// ************

pub const WINDOWS_UNC_PREFIX: &str = "\\\\?\\";

/// Common prefix for temporary files and directories.
pub const TEMPFILE_PREFIX: &str = ".~syretmp.";
