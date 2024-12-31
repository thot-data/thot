//! File system watcher.
const DEBOUNCE_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(100);

#[cfg(target_os = "windows")]
pub use windows::FileSystemActor;

#[cfg(target_os = "macos")]
pub use macos::FileSystemActor;

#[cfg(target_os = "linux")]
pub use linux::FileSystemActor;

#[cfg(target_os = "windows")]
mod windows {
    use super::DEBOUNCE_TIMEOUT;
    use crate::command::WatcherCommand as Command;
    use crossbeam::channel::{Receiver, Sender};
    use notify_debouncer_full::{DebounceEventResult, Debouncer, FileIdMap};
    use std::{
        io,
        path::{Path, PathBuf},
    };

    type FileSystemWatcher = notify::RecommendedWatcher;
    pub struct FileSystemActor {
        command_rx: Receiver<Command>,
        watcher: Debouncer<FileSystemWatcher, FileIdMap>,
    }

    impl FileSystemActor {
        /// Create a new actor to watch the file system.
        /// Begins watching upon creation.
        pub fn new(event_tx: Sender<DebounceEventResult>, command_rx: Receiver<Command>) -> Self {
            let watcher =
                notify_debouncer_full::new_debouncer(DEBOUNCE_TIMEOUT, None, event_tx).unwrap();

            Self {
                command_rx,
                watcher,
            }
        }

        pub fn run(&mut self) {
            loop {
                let cmd = match self.command_rx.recv() {
                    Ok(cmd) => cmd,
                    Err(err) => break,
                };

                match cmd {
                    Command::Watch { path, tx } => self.watch(path, tx),
                    Command::Unwatch { path, tx } => self.unwatch(path, tx),
                }
            }

            tracing::debug!("command channel closed, shutting down");
        }

        fn watch(&mut self, path: impl AsRef<Path>, tx: Sender<notify::Result<()>>) {
            let path = path.as_ref();
            if let Err(err) = self.watcher.watch(path, notify::RecursiveMode::Recursive) {
                if let Err(err) = tx.send(Err(err)) {
                    tracing::error!(?err);
                }

                return;
            }

            tracing::debug!("watching {path:?}");
            if let Err(err) = tx.send(Ok(())) {
                tracing::error!(?err);
            }
        }

        fn unwatch(&mut self, path: impl AsRef<Path>, tx: Sender<notify::Result<()>>) {
            let path = path.as_ref();
            if let Err(err) = self.watcher.unwatch(path) {
                if let Err(err) = tx.send(Err(err)) {
                    tracing::error!(?err);
                }

                return;
            }

            tracing::debug!("unwatching {path:?}");
            if let Err(err) = tx.send(Ok(())) {
                tracing::error!(?err);
            }
        }

        // TODO: Respond with an `Err` if can not get file id.
        /// Gets the final path of a file.
        ///
        /// # Response
        ///
        fn final_path(
            &mut self,
            path: impl AsRef<Path>,
            tx: Sender<Result<Option<PathBuf>, file_path_from_id::Error>>,
        ) {
            let path = path.as_ref();
            let id = match file_id::get_file_id(path) {
                Ok(id) => id,
                Err(err) => {
                    tracing::error!(?err);
                    match tx.send(Ok(None)) {
                        Ok(_) => {}
                        Err(err) => tracing::error!(?err),
                    };
                    return;
                }
            };

            let path_res = match file_path_from_id::path_from_id(&id) {
                Ok(path) => Ok(Some(path)),
                Err(err) => Err(err),
            };

            match tx.send(path_res) {
                Ok(_) => {}
                Err(err) => tracing::debug!(?err),
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::DEBOUNCE_TIMEOUT;
    use crate::command::WatcherCommand as Command;
    use crossbeam::channel::{Receiver, Sender};
    use notify::{notify::RecursiveMode, Watcher};
    use notify_debouncer_full::{DebounceEventResult, Debouncer, FileIdCache, FileIdMap};
    use std::path::Path;

    type FileSystemWatcher = notify::PollWatcher;

    pub struct FileSystemActor {
        command_rx: Receiver<Command>,
        watcher: Debouncer<FileSystemWatcher, FileIdMap>,
    }

    impl FileSystemActor {
        /// Create a new actor to watch the file system.
        /// Begins watching upon creation.
        ///
        /// # Notes
        /// + On macOS, PollWatcher is used for more informative events.
        pub fn new(event_tx: Sender<DebounceEventResult>, command_rx: Receiver<Command>) -> Self {
            let watcher: Debouncer<FileSystemWatcher, _> = {
                let event_tx = event_tx.clone();
                let config = notify::Config::default()
                    .with_poll_interval(DEBOUNCE_TIMEOUT)
                    .with_compare_contents(true);

                notify_debouncer_full::new_debouncer_opt(
                    DEBOUNCE_TIMEOUT,
                    None,
                    move |event: DebounceEventResult| {
                        event_tx.send(event).unwrap();
                    },
                    notify_debouncer_full::FileIdMap::new(),
                    config,
                )
                .unwrap()
            };

            Self {
                command_rx,
                watcher,
            }
        }

        pub fn run(&mut self) {
            loop {
                let Ok(cmd) = self.command_rx.recv() else {
                    tracing::debug!("command channel closed, shutting down");
                    break;
                };

                match cmd {
                    Command::Watch(path) => self.watch(path),
                    Command::Unwatch(path) => self.unwatch(path),
                    Command::FileId { path, tx } => {
                        if let Err(err) =
                            tx.send(self.watcher.cache().cached_file_id(&path).cloned())
                        {
                            tracing::error!(?err);
                        };
                    }
                }
            }
        }

        fn watch(&mut self, path: impl AsRef<Path>) {
            let path = path.as_ref();
            self.watcher
                .watcher()
                .watch(path, notify::RecursiveMode::Recursive)
                .unwrap();

            self.watcher
                .cache()
                .add_root(path, notify::RecursiveMode::Recursive);
        }

        fn unwatch(&mut self, path: impl AsRef<Path>) {
            let path = path.as_ref();
            self.watcher.watcher().unwatch(path).unwrap();
            self.watcher.cache().remove_root(path);
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::DEBOUNCE_TIMEOUT;
    use crate::command::WatcherCommand as Command;
    use crossbeam::channel::{Receiver, Sender};
    use notify::{self, notify::RecursiveMode, Watcher};
    use notify_debouncer_full::{DebounceEventResult, Debouncer, FileIdCache, FileIdMap};
    use std::path::{Path, PathBuf};

    type FileSystemWatcher = notify::RecommendedWatcher;

    pub struct FileSystemActor {
        command_rx: Receiver<Command>,
        watcher: Debouncer<FileSystemWatcher, FileIdMap>,
    }

    impl FileSystemActor {
        /// Create a new actor to watch the file system.
        /// Begins watching upon creation.
        pub fn new(event_tx: Sender<DebounceEventResult>, command_rx: Receiver<Command>) -> Self {
            let watcher =
                notify_debouncer_full::new_debouncer(DEBOUNCE_TIMEOUT, None, event_tx).unwrap();

            Self {
                command_rx,
                watcher,
            }
        }

        pub fn run(&mut self) {
            loop {
                let cmd = match self.command_rx.recv() {
                    Ok(cmd) => cmd,
                    Err(err) => break,
                };

                match cmd {
                    Command::Watch { path, tx } => self.watch(path, tx),
                    Command::Unwatch { path, tx } => self.unwatch(path, tx),
                    Command::FileId { path, tx } => {
                        if let Err(err) =
                            tx.send(self.watcher.cache().cached_file_id(&path).cloned())
                        {
                            tracing::error!(?err);
                        };
                    }
                }
            }

            tracing::debug!("command channel closed, shutting down");
        }

        fn watch(&mut self, path: impl AsRef<Path>, tx: Sender<notify::Result<()>>) {
            let path = path.as_ref();
            if let Err(err) = self
                .watcher
                .watcher()
                .watch(path, notify::RecursiveMode::Recursive)
            {
                if let Err(err) = tx.send(Err(err)) {
                    tracing::error!(?err);
                }

                return;
            }

            self.watcher
                .cache()
                .add_root(path, notify::RecursiveMode::Recursive);

            tracing::debug!("watching {path:?}");
            if let Err(err) = tx.send(Ok(())) {
                tracing::error!(?err);
            }
        }

        fn unwatch(&mut self, path: impl AsRef<Path>, tx: Sender<notify::Result<()>>) {
            let path = path.as_ref();
            if let Err(err) = self.watcher.watcher().unwatch(path) {
                if let Err(err) = tx.send(Err(err)) {
                    tracing::error!(?err);
                }

                return;
            }

            self.watcher.cache().remove_root(path);
            tracing::debug!("unwatching {path:?}");
            if let Err(err) = tx.send(Ok(())) {
                tracing::error!(?err);
            }
        }

        /// Gets the final path of a file.
        ///
        /// # Returns
        /// + `None` if the path is not in the watcher's cache.
        ///
        /// # Errors
        /// + If the final path could not be obtained.
        fn final_path(
            &mut self,
            path: impl AsRef<Path>,
            tx: Sender<Result<Option<PathBuf>, file_path_from_id::Error>>,
        ) {
            let path = path.as_ref();
            let cache = self.watcher.cache();
            let Some(id) = cache.cached_file_id(path) else {
                match tx.send(Ok(None)) {
                    Ok(_) => {}
                    Err(err) => tracing::debug!(?err),
                };
                return;
            };

            let path_res = match file_path_from_id::path_from_id(id) {
                Ok(path) => Ok(Some(path)),
                Err(err) => Err(err),
            };

            match tx.send(path_res) {
                Ok(_) => {}
                Err(err) => tracing::debug!(?err),
            }
        }
    }
}
