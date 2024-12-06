//! User settings related to default actions.
use crate::types::FsResourceAction;
use serde::{Deserialize, Serialize};

// ************************
// *** User Preferences ***
// ************************

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct UserPreferences {
    project: ProjectUserPreferences,
    analysis: AnalysisUserPreferences,
}

// ********************************
// *** Project User Preferences ***
// ********************************

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProjectUserPreferences {
    /// How asset files should be manipulated when changed into an asset
    asset_file_action: FsResourceAction,

    /// Rename container flder when name is changed
    rename_folder_on_name_change: bool,

    /// Delete folder and files when a subtree is excluded
    delete_on_exclude: bool,

    /// Pretty print and validate metadata objects in the editor
    format_metdata_objects: bool,

    /// Show inherited metadata when viewing object details
    show_inherited_metadata: bool,
}

impl Default for ProjectUserPreferences {
    fn default() -> Self {
        Self {
            asset_file_action: FsResourceAction::Move,
            rename_folder_on_name_change: true,
            delete_on_exclude: false,
            format_metdata_objects: true,
            show_inherited_metadata: false,
        }
    }
}

// *********************************
// *** Analysis User Preferences ***
// *********************************

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnalysisUserPreferences {}

impl Default for AnalysisUserPreferences {
    fn default() -> Self {
        Self {}
    }
}
