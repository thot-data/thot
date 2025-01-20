//! Runs a local [`Database`].
//!
//! Must be run with the `server` feature enabled.
use syre_local::{self as local, system::collections::ProjectManifest};
use syre_local_database::server;

/// Run the database with the default config.
fn main() {
    logging::enable();
    let default_panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        panic_hook(panic_info);
        default_panic_hook(panic_info);
    }));

    let config = server::config::Builder::try_default().unwrap();
    let projects = match ProjectManifest::load_or_default() {
        Ok(projects) => projects.to_vec(),
        Err(err) => {
            tracing::error!(?err);
            vec![]
        }
    };

    let mut db = server::Builder::new(config.build());
    db.add_paths(projects);
    db.add_ignore_path(format!("**/{}*", local::constants::TEMPFILE_PREFIX))
        .unwrap();

    db.run().unwrap();
}

fn panic_hook(panic_info: &std::panic::PanicHookInfo) {
    let payload = if let Some(payload) = panic_info.payload().downcast_ref::<&str>() {
        Some(&**payload)
    } else if let Some(payload) = panic_info.payload().downcast_ref::<String>() {
        Some(payload.as_str())
    } else {
        None
    };

    let location = panic_info.location().map(|location| location.to_string());
    tracing::error!("local/database panicked: {location:?} : {payload:?}");
}

mod logging {
    use std::io;
    use syre_local::system::common;
    use tracing_subscriber::{
        fmt::{self, time::UtcTime},
        prelude::*,
        EnvFilter, Layer, Registry,
    };

    const LOG_PREFIX: &str = "database.local.log";

    /// Enable logging.
    pub fn enable() {
        let config_dir = common::config_dir_path().expect("could not get config dir path");
        let file_logger = tracing_appender::rolling::daily(config_dir, LOG_PREFIX);
        let (file_logger, _log_guard) = tracing_appender::non_blocking(file_logger);
        let file_logger = fmt::layer()
            .with_writer(file_logger)
            .with_timer(UtcTime::rfc_3339())
            .json()
            .with_filter(EnvFilter::from_default_env());

        let console_logger = fmt::layer()
            .with_writer(io::stdout)
            .with_timer(UtcTime::rfc_3339())
            .pretty()
            .with_filter(EnvFilter::from_default_env());

        let subscriber = Registry::default().with(console_logger).with(file_logger);
        tracing::subscriber::set_global_default(subscriber).unwrap();
    }
}
