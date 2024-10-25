//! Setup functionality for the app.
use crate::state;
use std::path::PathBuf;
use syre_core::types::ResourceId;
use syre_local_database::{self as db, state::ConfigState};
use tauri::{Listener, Manager};

/// Runs setup tasks:
/// 1. Launches the local database if needed.
/// 2. Launches the update listener.
/// 3. Creates the inital app state.
pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    if let Some((_rx, _child)) = crate::db::start_database_if_needed(app.handle()) {
        tracing::debug!("initialized local database");
    } else {
        tracing::debug!("database already running");
    };

    let actor = crate::db::actor::Builder::new(app.handle().clone());
    std::thread::Builder::new()
        .name("syre desktop update listener".to_string())
        .spawn(move || actor.run())?;

    let main = app.get_webview_window("main").unwrap();
    main.listen(crate::db::FS_EVENT_TOPIC, move |event| {
        tracing::debug!(?event);
    });

    let db = app.state::<db::Client>();
    let state = crate::State::new();
    if let ConfigState::Ok(local_config) = db.state().local_config().unwrap() {
        if let Some(user) = local_config.user {
            let projects = state::load_user_state(&db, &user);
            let _ = state
                .user()
                .lock()
                .unwrap()
                .insert(state::User::new(user, projects));
        }
    }
    assert!(app.manage(state));

    Ok(())
}
