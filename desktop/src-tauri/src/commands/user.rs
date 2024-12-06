use std::path::PathBuf;

use syre_core::{system::User, types::ResourceId};
use syre_local::error::IoSerde;
use syre_local_database as db;

/// # Returns
/// The active user.
#[tauri::command]
pub fn active_user(
    state: tauri::State<crate::State>,
    db: tauri::State<db::Client>,
) -> Option<User> {
    state
        .user()
        .lock()
        .unwrap()
        .as_ref()
        .map(|user| db.user().get(user.rid().clone()).unwrap())?
}

/// # Returns
/// User count in the user manifest.
#[tauri::command]
pub fn user_count(db: tauri::State<db::Client>) -> Result<usize, IoSerde> {
    db.state()
        .user_manifest()
        .unwrap()
        .map(|manifest| manifest.len())
}

/// # Returns
/// All the projects belonging to the user.
#[tauri::command]
pub fn user_projects(
    db: tauri::State<db::Client>,
    user: ResourceId,
) -> Vec<(PathBuf, db::state::ProjectData)> {
    db.user().projects(user).unwrap()
}
