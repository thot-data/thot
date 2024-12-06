use super::FsWatcher;
use crate::WatcherCommand;
use notify_debouncer_full::DebouncedEvent;

impl FsWatcher {
    /// Some text editors remove then replace a file when saving it.
    /// We must manually check it the file exists to determine if this occured.
    ///
    /// If a config file is removed, it is added to the path watcher.
    ///
    /// # Returns
    /// Events modified to account for false removals.
    ///
    /// # Notes
    /// See https://docs.rs/notify/latest/notify/#editor-behaviour.
    fn handle_remove_events(&self, mut events: Vec<DebouncedEvent>) -> Vec<DebouncedEvent> {
        let mut remove_events = vec![];
        for (index, event) in events.iter().enumerate() {
            match &event.kind {
                notify::EventKind::Remove(_) => {
                    let [path] = &event.paths[..] else {
                        panic!("invalid paths");
                    };

                    if !remove_events.iter().any(|&index| {
                        let event: &DebouncedEvent = &events[index];
                        if let Some(p) = event.paths.get(0) {
                            p == path
                        } else {
                            false
                        }
                    }) {
                        remove_events.push(index);
                    }
                }
                notify::EventKind::Create(_) => {
                    let [path] = &event.paths[..] else {
                        panic!("invalid paths");
                    };

                    if let Some(index) = remove_events.iter().position(|&index| {
                        let event = &events[index];
                        &event.paths[0] == path
                    }) {
                        remove_events.swap_remove(index);
                    }
                }
                notify::EventKind::Any
                | notify::EventKind::Access(_)
                | notify::EventKind::Modify(_)
                | notify::EventKind::Other => {}
            }
        }

        for index in remove_events {
            let event = &events[index];
            let [path] = &event.paths[..] else {
                panic!("invalid paths");
            };

            if path == self.app_config.user_manifest()
                || path == self.app_config.project_manifest()
                || path == self.app_config.local_config()
            {
                if path.exists() {
                    let (tx, rx) = crossbeam::channel::bounded(1);
                    tracing::debug!("rewatching {path:?}");
                    self.command_tx
                        .send(WatcherCommand::Watch {
                            path: path.clone(),
                            tx,
                        })
                        .unwrap();

                    match rx.recv().unwrap() {
                        Ok(()) => {
                            let event = event.event.clone().set_kind(notify::EventKind::Modify(
                                notify::event::ModifyKind::Data(notify::event::DataChange::Any),
                            ));

                            *events[index] = event;
                        }
                        Err(err) => {
                            panic!("UNUSUAL SITUATION: watching manifest modified with {event:?} resulted in {err:?}");
                        }
                    }
                } else {
                    self.path_watcher_command_tx
                        .send(path_watcher::Command::Watch(path.clone()))
                        .unwrap();
                }
            } else {
                tracing::debug!("UNUSUAL REMOVE EVENT: {event:?}");
            }
        }

        events
    }
}
