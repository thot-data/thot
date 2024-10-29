//! Setup functionality for the app.
use crate::state;
use std::path::PathBuf;
use syre_core::types::ResourceId;
use syre_local_database::{self as db, state::ConfigState};
use tauri::{Listener, Manager};

const DB_CONNECTION_ATTEMPTS: usize = 50;
const DB_CONNECTION_DELAY_MS: u64 = 100;

/// Runs setup tasks:
/// 1. Launches the local database if needed.
/// 2. Launches the update listener.
/// 3. Creates the inital app state.
pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    if let Some((_rx, _child)) = crate::db::start_database_if_needed(app.handle()) {
        tracing::trace!("initializing local database");
        let mut attempt = 0;
        while !db::Client::server_available() {
            attempt += 1;
            if attempt > DB_CONNECTION_ATTEMPTS {
                panic!("could not connect to database");
            }

            std::thread::sleep(std::time::Duration::from_millis(DB_CONNECTION_DELAY_MS));
        }

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
