// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::sync::Mutex;
use syre_desktop::{
    commands::{
        analyses, asset, auth, common, container, fs, graph, mixed_bulk, project, settings, user,
    },
    setup, state,
};

fn main() {
    let builder = tauri::Builder::default();

    #[cfg(debug_assertions)] // only enable devtools instrumentation in development builds
    let builder = builder.plugin(tauri_plugin_devtools::init());
    #[cfg(not(debug_assertions))]
    let _log_guard = logging::enable();

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(syre_local_database::Client::new())
        .manage(state::new_slice(Option::<state::AnalyzerAction>::None))
        .invoke_handler(tauri::generate_handler![
            analyses::project_add_analyses,
            asset::asset_properties_update_bulk,
            asset::asset_properties_update,
            asset::asset_remove_file,
            auth::login,
            auth::logout,
            auth::register_user,
            common::file_size,
            common::open_file,
            common::target_os,
            container::container_analysis_associations_update_bulk,
            container::container_analysis_associations_update,
            container::container_properties_update_bulk,
            container::container_properties_update,
            container::container_rename_bulk,
            container::container_rename,
            fs::pick_file_with_location,
            fs::pick_folder_with_location,
            fs::pick_folder,
            graph::add_file_system_resources,
            graph::container_duplicate,
            graph::container_trash,
            graph::create_child_container,
            mixed_bulk::properties_update_bulk_mixed,
            project::trigger_analysis,
            project::cancel_analysis,
            project::kill_analysis,
            project::create_project,
            project::delete_project,
            project::deregister_project,
            project::import_project,
            project::initialize_project,
            project::project_analysis_remove,
            project::project_properties_update,
            project::project_resources,
            settings::user_settings,
            settings::user_settings_desktop_update,
            settings::user_settings_runner_update,
            user::active_user,
            user::user_count,
            user::user_projects,
        ])
        .setup(setup)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(not(debug_assertions))]
mod logging {
    use syre_desktop as desktop;
    use tracing_subscriber::{
        fmt::{self, time},
        prelude::*,
        Registry,
    };

    const LOG_PREFIX: &str = "desktop.log";

    pub fn enable() -> tracing_appender::non_blocking::WorkerGuard {
        let config_dir = desktop::common::config_dir_path().unwrap();
        let file_logger = tracing_appender::rolling::daily(config_dir, LOG_PREFIX);
        let (file_logger, _log_guard) = tracing_appender::non_blocking(file_logger);
        let file_logger = fmt::layer()
            .with_writer(file_logger)
            .with_timer(time::UtcTime::rfc_3339())
            .json();

        let console_logger = fmt::layer()
            .with_writer(std::io::stdout)
            .with_timer(time::UtcTime::rfc_3339())
            .pretty();

        let subscriber = Registry::default().with(console_logger).with(file_logger);
        tracing::subscriber::set_global_default(subscriber).unwrap();

        _log_guard
    }
}
