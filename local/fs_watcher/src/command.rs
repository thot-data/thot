use crossbeam::channel::Sender;
use std::path::PathBuf;

#[derive(Debug)]
pub enum Command {
    Watch(
        /// Path to watch.
        /// Must be absolute.
        PathBuf,
    ),
    Unwatch(
        /// Path to unwatch.
        /// Must be absolute.
        PathBuf,
    ),

    /// Clear all projects from beihng watched.
    /// Continues watching app config files.
    ClearProjects,

    /// Gets the final path of the given path if it is being tracked.
    FinalPath {
        path: PathBuf,
        tx: Sender<Result<Option<PathBuf>, file_path_from_id::Error>>,
    },

    /// Shutdown the watcher.
    Shutdown,
}

pub(crate) enum WatcherCommand {
    Watch {
        path: PathBuf,
        tx: Sender<notify::Result<()>>,
    },

    Unwatch {
        path: PathBuf,
        tx: Sender<notify::Result<()>>,
    },
}
