//! Initialization functionality with a [`Database`].
use syre_local_database::Client as DbClient;
use tauri::async_runtime::Receiver;
use tauri_plugin_shell::{
    process::{CommandChild, CommandEvent},
    ShellExt,
};

/// Init
pub fn start_database_if_needed(
    app: &tauri::AppHandle,
) -> Option<(Receiver<CommandEvent>, CommandChild)> {
    // try to connect to database
    if DbClient::server_available() {
        return None;
    }

    // database not running
    // create one
    let handler = init_database(app);
    Some(handler)
}

// IMPORTANT: On macOS m1, not dropping the `Receiver` (part of the _db_handler), causes ZMQ issues.
/// Initializes a [`Database`] as a sidecar process.
fn init_database(app: &tauri::AppHandle) -> (Receiver<CommandEvent>, CommandChild) {
    app.shell()
        .sidecar("syre-local-database")
        .expect("failed to create `syre-local-database` binary command")
        .spawn()
        .expect("failed to spawn sidecar")
}
