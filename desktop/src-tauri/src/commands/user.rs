//! Commands related to users.
use crate::error::Result;
use crate::settings::{UserAppState, UserSettings};
use crate::state::AppState;
use tauri::State;
use thot_core::system::User;
use thot_core::types::ResourceId;
use thot_local::system::users;

/// Get the active user.
/// Retrieves the active user from [system settings](users::get_active_user).
#[tauri::command]
pub fn get_active_user() -> Result<Option<User>> {
    let active_user = users::get_active_user()?;
    Ok(active_user)
}

/// Set the active user.
/// Sets the active user on the [system settings](users::set_active_user).
/// Sets the active user on the [`AppState`].
/// Loads the user's [`UserAppState`] and [`UserSettings`].
#[tracing::instrument(skip(app_state))]
#[tauri::command]
pub fn set_active_user(app_state: State<AppState>, rid: ResourceId) -> Result {
    // settings user
    users::set_active_user(&rid)?;

    // set app user
    let user = users::user_by_id(&rid)?;
    *app_state
        .user
        .lock()
        .expect("could not lock `AppState.user`") = user;

    // settings
    let user_app_state = UserAppState::load_or_new(&rid)?;
    *app_state
        .user_app_state
        .lock()
        .expect("could not lock `AppState.user_app_state`") = Some(user_app_state);

    let user_settings = UserSettings::load_or_new(&rid)?.into();
    *app_state
        .user_settings
        .lock()
        .expect("could not lock `AppState.user_settings`") = Some(user_settings);

    Ok(())
}

/// Unset the active user.
/// Unsets the active user on the [system settings](users::set_active_user).
/// Unsets the active user on the [`AppState`].
/// Unsets the user's [`UserAppState`] and [`UserSettings`].
#[tauri::command]
pub fn unset_active_user(app_state: State<AppState>) -> Result {
    users::unset_active_user()?;
    *app_state
        .user
        .lock()
        .expect("could not lock `AppState.user`") = None;

    *app_state
        .user_app_state
        .lock()
        .expect("could not lock `AppState.user_app_state`") = None;

    *app_state
        .user_settings
        .lock()
        .expect("could not lock `AppState.user_settings`") = None;

    Ok(())
}
