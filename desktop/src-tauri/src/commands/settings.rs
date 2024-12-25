use crate::settings;
use std::path::PathBuf;
use syre_core::types::ResourceId;
use syre_desktop_lib as lib;

/// Retrieve the desktop settings for the active user.
/// If none are set, uses default.
#[tauri::command]
pub fn user_settings(state: tauri::State<crate::State>) -> Option<lib::settings::User> {
    let user = state.user();
    let user = user.lock().unwrap();
    let Some(user) = user.as_ref() else {
        return None;
    };

    let settings = settings::User::load(user.rid()).replace_not_found_with_default();
    Some(settings.into())
}

/// Update the desktop settings for the active user.
#[tauri::command]
pub fn user_settings_desktop_update(
    state: tauri::State<crate::State>,
    user: ResourceId,
    update: lib::settings::user::Desktop,
) -> Result<(), lib::command::error::IoErrorKind> {
    let state_user = state.user();
    let state_user = state_user.lock().unwrap();
    let Some(ref state_user) = *state_user else {
        panic!("invalid state");
    };
    assert_eq!(user, *state_user.rid());

    let settings: settings::user::Desktop = update.into();
    settings.save(&user).map_err(|err| err.into())
}

/// Update the runner settings for the active user.
#[tauri::command]
pub fn user_settings_runner_update(
    state: tauri::State<crate::State>,
    user: ResourceId,
    update: lib::settings::user::Runner,
) -> Result<(), lib::command::error::IoErrorKind> {
    let state_user = state.user();
    let state_user = state_user.lock().unwrap();
    let Some(ref state_user) = *state_user else {
        panic!("invalid state");
    };
    assert_eq!(user, *state_user.rid());

    settings::user::Runner::save(&user, update).map_err(|err| err.into())
}

/// Retrieve the project settings.
/// If none are set, uses default.
#[tauri::command]
pub fn project_settings(project: PathBuf) -> lib::settings::Project {
    settings::Project::load(&project)
        .replace_not_found_with_default()
        .into()
}

/// Update the runner settings for the project.
#[tauri::command]
pub fn project_settings_runner_update(
    state: tauri::State<crate::State>,
    project: PathBuf,
    update: lib::settings::project::Runner,
) -> Result<(), lib::command::error::IoErrorKind> {
    settings::project::Runner::save(&project, update).map_err(|err| err.into())
}
