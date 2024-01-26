//! Database for storing resources.
#[path = "./command/mod.rs"]
pub(super) mod command;

#[path = "./file_system/mod.rs"]
mod file_system;

use self::command::CommandActor;
use self::file_system::actor::{FileSystemActor, FileSystemActorCommand};
use self::file_system::file_system_event_processor::FileSystemEventProcessor;
use super::store::Datastore;
use super::Event;
use crate::command::Command;
use crate::event::Update;
use crate::{common, constants, Result};
use notify_debouncer_full::{DebounceEventResult, DebouncedEvent};
use serde_json::Value as JsValue;
use std::path::PathBuf;
use std::result::Result as StdResult;
use std::sync::mpsc;
use std::thread;

/// Database.
pub struct Database {
    store: Datastore,
    event_rx: mpsc::Receiver<Event>,
    file_system_tx: mpsc::Sender<FileSystemActorCommand>,

    /// Publication socket to broadcast updates.
    update_tx: zmq::Socket,
}

impl Database {
    /// Creates a new Database.
    /// The database immediately begins listening for ZMQ and file system events.
    pub fn new() -> Self {
        let zmq_context = zmq::Context::new();
        let update_tx = zmq_context.socket(zmq::PUB).unwrap();
        update_tx.bind(&common::zmq_url(zmq::PUB).unwrap()).unwrap();

        let (event_tx, event_rx) = mpsc::channel();
        let (file_system_tx, file_system_rx) = mpsc::channel();
        let command_actor = CommandActor::new(event_tx.clone());
        let mut file_system_actor = FileSystemActor::new(event_tx, file_system_rx);

        thread::spawn(move || command_actor.run());
        thread::spawn(move || file_system_actor.run());

        Database {
            store: Datastore::new(),
            event_rx,
            file_system_tx,
            update_tx,
        }
    }

    /// Begin responding to events.
    pub fn start(&mut self) {
        self.listen_for_events();
    }

    /// Listen for events coming from child actors.
    fn listen_for_events(&mut self) {
        loop {
            match self.event_rx.recv().unwrap() {
                Event::Command { cmd, tx } => tx.send(self.handle_command(cmd)).unwrap(),
                Event::FileSystem(events) => self.handle_file_system_events(events).unwrap(),
            }
        }
    }

    /// Add a path to watch for file system changes.
    fn watch_path(&mut self, path: impl Into<PathBuf>) {
        self.file_system_tx
            .send(FileSystemActorCommand::Watch(path.into()))
            .unwrap();
    }

    /// Remove a path from watching file system changes.
    fn unwatch_path(&mut self, path: impl Into<PathBuf>) {
        self.file_system_tx
            .send(FileSystemActorCommand::Unwatch(path.into()))
            .unwrap();
    }

    /// Gets the final path of a file from the file system watcher.
    fn get_final_path(
        &self,
        path: impl Into<PathBuf>,
    ) -> StdResult<Option<PathBuf>, file_path_from_id::Error> {
        let (tx, rx) = mpsc::channel();
        self.file_system_tx
            .send(FileSystemActorCommand::FinalPath {
                path: path.into(),
                tx,
            })
            .unwrap();

        rx.recv().unwrap()
    }

    /// Publish an update to subscribers.
    /// Triggered by file system events.
    fn publish_update(&self, update: &Update) -> zmq::Result<()> {
        let mut topic = constants::PUB_SUB_TOPIC.to_string();
        match update {
            Update::Project {
                project,
                update: _update,
            } => {
                topic.push_str(&format!("/project/{project}"));
            }
        };

        self.update_tx.send(&topic, zmq::SNDMORE)?;
        self.update_tx
            .send(&serde_json::to_string(update).unwrap(), 0)
    }

    // TODO Handle errors.
    /// Handles a given command, returning the correct data.
    fn handle_command(&mut self, command: Command) -> JsValue {
        tracing::debug!(?command);
        match command {
            Command::AssetCommand(cmd) => self.handle_command_asset(cmd),
            Command::ContainerCommand(cmd) => self.handle_command_container(cmd),
            Command::DatabaseCommand(cmd) => self.handle_command_database(cmd),
            Command::ProjectCommand(cmd) => self.handle_command_project(cmd),
            Command::GraphCommand(cmd) => self.handle_command_graph(cmd),
            Command::ScriptCommand(cmd) => self.handle_command_script(cmd),
            Command::UserCommand(cmd) => self.handle_command_user(cmd),
            Command::AnalysisCommand(cmd) => self.handle_command_analysis(cmd),
        }
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;

    impl Database {
        /// Handle file system events.
        /// To be used with [`notify::Watcher`]s.
        #[tracing::instrument(skip(self))]
        pub fn handle_file_system_events(&mut self, events: DebounceEventResult) -> Result {
            let events = match events {
                Ok(events) => events,
                Err(errs) => {
                    tracing::debug!("watch error: {errs:?}");
                    return Err(crate::Error::Database(format!("{errs:?}")));
                }
            };

            let events = self.rectify_event_paths(events);
            let mut events = FileSystemEventProcessor::process(events);
            events.sort_by(|a, b| a.time.cmp(&b.time));
            for event in events {
                if let Err(_err) = self.process_file_system_event(&event) {
                    tracing::debug!(?event);
                };
            }

            Ok(())
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use std::path::{Component, Path};
    use std::time::Instant;

    const TRASH_PATH: &str = ".Trash";

    impl Database {
        /// Handle file system events.
        /// To be used with [`notify::Watcher`]s.
        #[tracing::instrument(skip(self))]
        pub fn handle_file_system_events(&mut self, events: DebounceEventResult) -> Result {
            let events = match events {
                Ok(events) => events,
                Err(errs) => self.handle_file_system_watcher_errors(errs)?,
            };

            let mut events = FileSystemEventProcessor::process(events);
            events.sort_by(|a, b| a.time.cmp(&b.time));
            for event in events {
                if let Err(_err) = self.process_file_system_event(&event) {
                    tracing::debug!(?event);
                };
            }

            Ok(())
        }

        fn handle_file_system_watcher_errors(
            &self,
            errors: Vec<notify::Error>,
        ) -> Result<Vec<DebouncedEvent>> {
            const WATCH_ROOT_MOVED_PATTERN: &str =
                r"IO error for operation on (.+): No such file or directory \(os error 2\)";

            let (root_moved_errors, unhandled_errors): (Vec<_>, Vec<_>) =
                errors.into_iter().partition(|err| match &err.kind {
                    notify::ErrorKind::Generic(msg)
                        if msg.contains("No such file or directory (os error 2)") =>
                    {
                        true
                    }

                    _ => false,
                });

            let root_moved_pattern = regex::Regex::new(WATCH_ROOT_MOVED_PATTERN).unwrap();
            let moved_roots = root_moved_errors
                .into_iter()
                .map(|err| {
                    let notify::ErrorKind::Generic(msg) = err.kind else {
                        panic!("failed to partition errors correctly");
                    };

                    match root_moved_pattern.captures(&msg) {
                        None => panic!("unknown error message"),
                        Some(captures) => {
                            let path = captures.get(1).unwrap().as_str().to_string();
                            PathBuf::from(path)
                        }
                    }
                })
                .collect::<Vec<_>>();

            if moved_roots.len() == 0 && unhandled_errors.len() > 0 {
                tracing::debug!("watch error: {unhandled_errors:?}");
                return Err(crate::Error::Database(format!("{unhandled_errors:?}")));
            }

            let mut events = Vec::with_capacity(moved_roots.len() * 2);
            for path in moved_roots {
                let final_path = match self.get_final_path(&path) {
                    Ok(Some(final_path)) => Some(final_path),

                    Ok(None) => {
                        tracing::debug!("could not get final path of {path:?}");
                        continue;
                    }

                    Err(file_path_from_id::Error::NoFileInfo) => {
                        // path deleted
                        None
                    }

                    Err(err) => {
                        tracing::debug!("error retrieving final path of {path:?}: {err:?}");
                        continue;
                    }
                };

                tracing::debug!(?final_path);

                events.push(DebouncedEvent::new(
                    notify::Event {
                        kind: notify::EventKind::Remove(notify::event::RemoveKind::Folder),
                        paths: vec![path],
                        attrs: notify::event::EventAttributes::new(),
                    },
                    Instant::now(),
                ));

                if let Some(final_path) = final_path {
                    if !path_in_trash(&final_path) {
                        events.push(DebouncedEvent::new(
                            notify::Event {
                                kind: notify::EventKind::Create(notify::event::CreateKind::Folder),
                                paths: vec![final_path],
                                attrs: notify::event::EventAttributes::new(),
                            },
                            Instant::now(),
                        ));
                    }
                }
            }
            tracing::debug!(?events);

            Ok(events)
        }
    }

    fn path_in_trash(path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        match std::env::var_os("HOME") {
            None => {
                for component in path.components() {
                    match component {
                        Component::Normal(component) => {
                            if component == TRASH_PATH {
                                return true;
                            }
                        }

                        _ => {}
                    }
                }

                return false;
            }
            Some(home) => {
                let trash_path = PathBuf::from(home).join(TRASH_PATH);
                return path.starts_with(trash_path);
            }
        }
    }
}
