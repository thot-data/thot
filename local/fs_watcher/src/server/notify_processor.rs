use super::{
    super::{event as fs_event, ConversionError, ConversionResult},
    FsWatcher,
};
use crate::{command::WatcherCommand, error};
use notify::event::{CreateKind, EventKind as NotifyEventKind, ModifyKind, RemoveKind, RenameMode};
use notify_debouncer_full::{DebouncedEvent, FileIdCache};
use std::{
    assert_matches::assert_matches, collections::HashMap, fs, io, path::PathBuf,
    result::Result as StdResult,
};
use syre_local::common as local_common;

impl FsWatcher {
    /// Process [`notify_debouncer_full::DebouncedEvent`]s into [`file_system::Event`](fs_event::Event)s.
    ///
    /// # Notes
    /// + Events are assumed to have already been preprocessed with paths rectified.
    ///
    /// # Returns
    /// Tuple of (events, errors).
    pub fn process_events_notify_to_fs<'a>(
        &'a self,
        events: &'a Vec<DebouncedEvent>,
    ) -> (Vec<fs_event::Event>, Vec<ConversionError>) {
        let events = events.iter().collect::<Vec<_>>();
        let filtered_events = Self::filter_events(events.clone())
            .into_iter()
            .filter(|event| {
                match &event.paths[..] {
                    // TODO: May want to handle events with multiple paths.
                    // Could do this by passing in ignore paths to `Self::filter_events` and deciding
                    // how to handle it based on event type. e.g. If moving from an ignored path treat as a creation event;
                    // if moving into an ignored path, treat as a removal event; if moving between ignore paths, filter event out.
                    [path] => !self
                        .app_config
                        .ignore_paths()
                        .iter()
                        .any(|pattern| pattern.matches_path(path)),
                    _ => true,
                }
            })
            .collect::<Vec<_>>();

        let (grouped, remaining) = self.group_events(filtered_events);
        let (mut converted, errors) = self.convert_events(remaining);
        converted.extend(grouped);

        self.update_file_ids(events);
        (converted, errors)
    }

    /// Filters out uninteresting events.
    fn filter_events(events: Vec<&DebouncedEvent>) -> Vec<&DebouncedEvent> {
        let events = events
            .into_iter()
            .filter(|event| match event.kind {
                NotifyEventKind::Create(_)
                | NotifyEventKind::Remove(_)
                | NotifyEventKind::Modify(ModifyKind::Data(_))
                | NotifyEventKind::Modify(ModifyKind::Name(_))
                | NotifyEventKind::Modify(ModifyKind::Any) => true,

                _ => false,
            })
            .filter(|event| {
                if let [path] = &event.paths[..] {
                    if let Some(file_name) = path.file_name() {
                        if file_name == ".DS_Store" {
                            return false;
                        }

                        if let Some(file_name) = file_name.to_str() {
                            if is_lock_file(file_name) {
                                return false;
                            }
                        }
                    }
                }

                true
            })
            .collect::<Vec<_>>();

        let events = Self::filter_nested_events(events);

        #[cfg(target_os = "windows")]
        let events = Self::windows_filter_modify_events(events);

        events
    }

    /// Remove nested events.
    ///
    /// i.e. If a folder was created/removed with children, and both the parent folder and children
    /// resources creation/removal events are present, the events of the children are filtered out.
    fn filter_nested_events<'a>(mut events: Vec<&'a DebouncedEvent>) -> Vec<&'a DebouncedEvent> {
        use notify::EventKind;

        enum EventRelation {
            ParentOf(usize),
            Descendant,
        }

        let len_orig = events.len();
        let create_remove_folder_events = events
            .clone()
            .into_iter()
            .filter(|event| match event.kind {
                EventKind::Create(notify::event::CreateKind::Folder)
                | EventKind::Create(notify::event::CreateKind::Any)
                | EventKind::Remove(notify::event::RemoveKind::Folder)
                | EventKind::Remove(notify::event::RemoveKind::Any) => true,
                _ => false,
            })
            .collect::<Vec<_>>();

        let mut parent_events: Vec<&DebouncedEvent> = vec![];
        for event in create_remove_folder_events {
            let [path] = &event.paths[..] else {
                panic!("invalid paths");
            };

            let relation = parent_events.iter().enumerate().find_map(|(idx, parent)| {
                let [parent_path] = &(*parent).paths[..] else {
                    panic!("invalid paths");
                };

                if parent_path.starts_with(path) {
                    Some(EventRelation::ParentOf(idx))
                } else if path.starts_with(parent_path) {
                    Some(EventRelation::Descendant)
                } else {
                    None
                }
            });

            match relation {
                None => parent_events.push(event),
                Some(EventRelation::ParentOf(idx)) => parent_events[idx] = event,
                Some(EventRelation::Descendant) => {}
            };
        }

        events.retain(|event| {
            match event.kind {
                EventKind::Create(_)
                | EventKind::Remove(_)
                | EventKind::Modify(notify::event::ModifyKind::Data(_))
                | EventKind::Modify(notify::event::ModifyKind::Any) => {
                    let [path] = &event.paths[..] else {
                        panic!("invalid paths");
                    };

                    !parent_events.iter().any(|parent| {
                        let [parent_path] = &parent.paths[..] else {
                            panic!("invalid paths");
                        };

                        if parent_path == path {
                            false
                        } else {
                            path.starts_with(parent_path)
                        }
                    })
                }
                EventKind::Modify(notify::event::ModifyKind::Name(_)) => {
                    // NB: Not clear how to handle.
                    // Let pass through.
                    true
                }
                _ => unreachable!("event kind previously filtered"),
            }
        });

        tracing::trace!("filtered {} nested events", len_orig - events.len());
        events
    }

    /// Tries to convert all events into a single one.
    ///
    /// # Returns
    /// Tuple of (<converted events>, <unconverted events>).
    fn group_events<'a>(
        &'a self,
        events: Vec<&'a DebouncedEvent>,
    ) -> (Vec<fs_event::Event>, Vec<&'a DebouncedEvent>) {
        let mut remaining = Vec::with_capacity(events.len());
        let mut grouped_id = HashMap::with_capacity(events.len());
        let mut grouped_path = HashMap::with_capacity(events.len());
        for event in events {
            match event.kind {
                NotifyEventKind::Remove(_) => {
                    let file_ids = self.file_ids.lock().unwrap();
                    if let Some(id) = file_ids.cached_file_id(&event.paths[0]).cloned() {
                        let entry = grouped_id.entry(id).or_insert(vec![]);
                        entry.push(event);
                        tracing::trace!("{event:?} added to grouped id");
                    }

                    let [path] = &event.paths[..] else {
                        panic!("invalid paths");
                    };

                    let entry = grouped_path.entry(path).or_insert(vec![]);
                    entry.push(event);
                    tracing::trace!("{event:?} added to grouped path");
                }

                NotifyEventKind::Create(_) => {
                    let id = match self.file_id_from_watcher(event.paths[0].clone()) {
                        Ok(id) => id,
                        Err(_err) => {
                            tracing::error!("could not retrieve id from watcher");
                            remaining.push(event);
                            tracing::trace!("{event:?} added to remaining");
                            continue;
                        }
                    };

                    let [path] = &event.paths[..] else {
                        panic!("invalid paths");
                    };
                    match grouped_path.get_mut(path) {
                        Some(path_events) => {
                            path_events.push(event);
                            tracing::trace!("{event:?} added to grouped path");
                        }

                        None => match id {
                            Some(id) => {
                                let entry = grouped_id.entry(id).or_insert(vec![]);
                                entry.push(event);
                                tracing::trace!("{event:?} added to grouped id");
                            }

                            None => {
                                remaining.push(event);
                                tracing::trace!("{event:?} added to remaining");
                            }
                        },
                    }
                }

                NotifyEventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                    let file_ids = self.file_ids.lock().unwrap();
                    let Some(id) = file_ids.cached_file_id(&event.paths[0]).cloned() else {
                        remaining.push(event);
                        tracing::trace!("{event:?} added to remaining");
                        continue;
                    };

                    let entry = grouped_id.entry(id).or_insert(vec![]);
                    entry.push(event);
                    tracing::trace!("{event:?} added to grouped id");
                }

                NotifyEventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                    let id = match self.file_id_from_watcher(event.paths[0].clone()) {
                        Ok(id) => id,
                        Err(_err) => {
                            tracing::error!("could not retrieve id from watcher");
                            remaining.push(event);
                            tracing::trace!("{event:?} added to remaining");
                            continue;
                        }
                    };

                    let Some(id) = id else {
                        remaining.push(event);
                        tracing::trace!("{event:?} added to remaining");
                        continue;
                    };

                    let entry = grouped_id.entry(id).or_insert(vec![]);
                    entry.push(event);
                    tracing::trace!("{event:?} added to grouped id");
                }

                NotifyEventKind::Modify(ModifyKind::Name(RenameMode::Any)) => {
                    let [path] = &event.paths[..] else {
                        panic!("invalid paths");
                    };

                    if path.exists() {
                        let id = match self.file_id_from_watcher(event.paths[0].clone()) {
                            Ok(id) => id,
                            Err(_err) => {
                                tracing::error!("could not retrieve id from watcher");
                                remaining.push(event);
                                tracing::trace!("{event:?} added to remaining");
                                continue;
                            }
                        };

                        let Some(id) = id else {
                            remaining.push(event);
                            tracing::trace!("{event:?} added to remaining");
                            continue;
                        };

                        let entry = grouped_id.entry(id).or_insert(vec![]);
                        entry.push(event);
                        tracing::trace!("{event:?} added to grouped id");
                    } else {
                        let file_ids = self.file_ids.lock().unwrap();
                        let Some(id) = file_ids.cached_file_id(&event.paths[0]).cloned() else {
                            remaining.push(event);
                            tracing::trace!("{event:?} added to remaining");
                            continue;
                        };

                        let entry = grouped_id.entry(id).or_insert(vec![]);
                        entry.push(event);
                        tracing::trace!("{event:?} added to grouped id");
                    }
                }

                _ => {
                    remaining.push(event);
                    tracing::trace!("{event:?} added to remaining");
                }
            }
        }

        tracing::trace!("converting grouped id");
        let mut converted = Vec::with_capacity(grouped_id.len() / 2);
        for mut events in grouped_id.into_values() {
            events.sort_unstable_by_key(|event| event.time);
            match &events[..] {
                [e] => {
                    tracing::trace!("{e:?} is alone");
                    match e.kind {
                        NotifyEventKind::Remove(_) | NotifyEventKind::Create(_) => {
                            let [path] = &e.paths[..] else {
                                panic!("invalid paths");
                            };

                            if !grouped_path.iter().any(|(p, _)| *p == path) {
                                tracing::trace!("event not paired by path added to remaining");
                                remaining.push(e);
                            }
                        }

                        _ => {
                            tracing::trace!("{e:?} added to remaining");
                            remaining.push(e);
                        }
                    }
                }

                [e1, e2] => match (e1.kind, e2.kind) {
                    (
                        NotifyEventKind::Modify(ModifyKind::Name(RenameMode::From)),
                        NotifyEventKind::Modify(ModifyKind::Name(RenameMode::To)),
                    )
                    | (
                        NotifyEventKind::Modify(ModifyKind::Name(RenameMode::Any)),
                        NotifyEventKind::Modify(ModifyKind::Name(RenameMode::Any)),
                    ) => {
                        let path_from = normalize_path_root(e1.paths[0].clone());
                        let path_to = normalize_path_root(e2.paths[0].clone());
                        if path_from.parent() == path_to.parent() {
                            if path_to.is_file() {
                                let event = fs_event::Event::new(
                                    fs_event::File::Renamed {
                                        from: path_from,
                                        to: path_to,
                                    },
                                    e2.time,
                                )
                                .add_parent(e1)
                                .add_parent(e2);

                                tracing::trace!("converted {e1:?} and {e2:?} to {event:?}");
                                converted.push(event);
                            } else if path_to.is_dir() {
                                let event = fs_event::Event::new(
                                    fs_event::Folder::Renamed {
                                        from: path_from,
                                        to: path_to,
                                    },
                                    e2.time,
                                )
                                .add_parent(e1)
                                .add_parent(e2);

                                tracing::trace!("converted {e1:?} and {e2:?} to {event:?}");
                                converted.push(event);
                            } else {
                                tracing::trace!("could not convert {events:?}");
                                remaining.extend(events);
                            }
                        } else {
                            if path_to.is_file() {
                                let event = fs_event::Event::new(
                                    fs_event::File::Moved {
                                        from: path_from,
                                        to: path_to,
                                    },
                                    e2.time,
                                )
                                .add_parent(e1)
                                .add_parent(e2);

                                tracing::trace!("converted {e1:?} and {e2:?} to {event:?}");
                                converted.push(event);
                            } else if path_to.is_dir() {
                                let event = fs_event::Event::new(
                                    fs_event::Folder::Moved {
                                        from: path_from,
                                        to: path_to,
                                    },
                                    e2.time,
                                )
                                .add_parent(e1)
                                .add_parent(e2);

                                tracing::trace!("converted {e1:?} and {e2:?} to {event:?}");
                                converted.push(event);
                            } else {
                                tracing::trace!("could not convert {e1:?} and {e2:?}");
                                remaining.extend(events);
                            }
                        }
                    }
                    (
                        NotifyEventKind::Remove(RemoveKind::File),
                        NotifyEventKind::Create(CreateKind::File),
                    ) => {
                        let path_from = normalize_path_root(e1.paths[0].clone());
                        let path_to = normalize_path_root(e2.paths[0].clone());
                        if path_from.parent() == path_to.parent() {
                            let event = fs_event::Event::new(
                                fs_event::File::Renamed {
                                    from: path_from,
                                    to: path_to,
                                },
                                e2.time,
                            )
                            .add_parent(e1)
                            .add_parent(e2);

                            tracing::trace!("converted {e1:?} and {e2:?} to {event:?}");
                            converted.push(event);
                        } else {
                            let event = fs_event::Event::new(
                                fs_event::File::Moved {
                                    from: path_from,
                                    to: path_to,
                                },
                                e2.time,
                            )
                            .add_parent(e1)
                            .add_parent(e2);

                            tracing::trace!("converted {e1:?} and {e2:?} to {event:?}");
                            converted.push(event);
                        }
                    }
                    (
                        NotifyEventKind::Remove(RemoveKind::Folder),
                        NotifyEventKind::Create(CreateKind::Folder),
                    ) => {
                        let path_from = normalize_path_root(e1.paths[0].clone());
                        let path_to = normalize_path_root(e2.paths[0].clone());
                        if path_from.parent() == path_to.parent() {
                            let event = fs_event::Event::new(
                                fs_event::Folder::Renamed {
                                    from: path_from,
                                    to: path_to,
                                },
                                e2.time,
                            )
                            .add_parent(e1)
                            .add_parent(e2);

                            tracing::trace!("converted {e1:?} and {e2:?} to {event:?}");
                            converted.push(event);
                        } else {
                            let event = fs_event::Event::new(
                                fs_event::Folder::Moved {
                                    from: path_from,
                                    to: path_to,
                                },
                                e2.time,
                            )
                            .add_parent(e1)
                            .add_parent(e2);

                            tracing::trace!("converted {e1:?} and {e2:?} to {event:?}");
                            converted.push(event);
                        }
                    }
                    (
                        NotifyEventKind::Remove(RemoveKind::Any),
                        NotifyEventKind::Create(CreateKind::Any),
                    ) => {
                        #[cfg(not(target_os = "windows"))]
                        todo!();

                        let path_from = normalize_path_root(e1.paths[0].clone());
                        let path_to = normalize_path_root(e2.paths[0].clone());
                        if path_from.parent() == path_to.parent() {
                            let event = if path_to.is_dir() {
                                fs_event::Event::new(
                                    fs_event::Folder::Renamed {
                                        from: path_from,
                                        to: path_to,
                                    },
                                    e2.time,
                                )
                                .add_parent(e1)
                                .add_parent(e2)
                            } else if path_to.is_file() {
                                fs_event::Event::new(
                                    fs_event::File::Renamed {
                                        from: path_from,
                                        to: path_to,
                                    },
                                    e2.time,
                                )
                                .add_parent(e1)
                                .add_parent(e2)
                            } else {
                                todo!();
                            };

                            tracing::trace!("converted {e1:?} and {e2:?} to {event:?}");
                            converted.push(event);
                        } else {
                            let event = if path_to.is_dir() {
                                fs_event::Event::new(
                                    fs_event::Folder::Moved {
                                        from: path_from,
                                        to: path_to,
                                    },
                                    e2.time,
                                )
                                .add_parent(e1)
                                .add_parent(e2)
                            } else if path_to.is_file() {
                                fs_event::Event::new(
                                    fs_event::File::Moved {
                                        from: path_from,
                                        to: path_to,
                                    },
                                    e2.time,
                                )
                                .add_parent(e1)
                                .add_parent(e2)
                            } else {
                                todo!();
                            };

                            tracing::trace!("converted {e1:?} and {e2:?} to {event:?}");
                            converted.push(event);
                        }
                    }

                    _ => {
                        tracing::trace!("could not convert {e1:?} and {e2:?} added to remaining");
                        remaining.extend(events);
                    }
                },

                _ => {
                    tracing::trace!("could not convert {events:?} added to remaining");
                    remaining.extend(events);
                }
            }
        }

        tracing::trace!("converting grouped path");
        let mut converted_parents = converted
            .iter()
            .flat_map(|event| event.parents())
            .collect::<Vec<_>>();
        for (path, events) in grouped_path.into_iter() {
            match &events[..] {
                [e] => {
                    tracing::trace!("{e:?} is alone");
                    if !(converted_parents.contains(e) || remaining.contains(e)) {
                        tracing::trace!("{e:?} add to remaining");
                        remaining.push(e);
                    }
                }

                [e1, e2] => {
                    assert_matches!(e1.kind, NotifyEventKind::Remove(_));
                    assert_matches!(e2.kind, NotifyEventKind::Create(_));
                    assert_eq!(e1.paths[0], *path);
                    assert_eq!(e2.paths[0], *path);

                    match e2.kind {
                        NotifyEventKind::Create(CreateKind::File) => {
                            let event = fs_event::Event::new(
                                fs_event::File::DataModified(path.clone()),
                                e2.time,
                            )
                            .add_parent(e1)
                            .add_parent(e2);

                            tracing::trace!("converted {e1:?} and {e2:?} to {event:?}");
                            converted.push(event);
                            converted_parents.extend(events);
                        }

                        NotifyEventKind::Create(CreateKind::Folder) => {
                            let event = fs_event::Event::new(
                                fs_event::Folder::Other(path.clone()),
                                e2.time,
                            )
                            .add_parent(e1)
                            .add_parent(e2);

                            tracing::trace!("converted {e1:?} and {e2:?} to {event:?}");
                            converted.push(event);
                            converted_parents.extend(events);
                        }

                        NotifyEventKind::Create(_) => {
                            tracing::trace!("could not convert {events:?}");
                            if !(converted_parents.contains(e1) || remaining.contains(e1)) {
                                tracing::trace!("{e1:?} added to remaining");
                                remaining.push(e1);
                            }
                            if !(converted_parents.contains(e2) || remaining.contains(e2)) {
                                tracing::trace!("{e2:?} added to remaining");
                                remaining.push(e2);
                            }
                        }

                        _ => unreachable!(),
                    }
                }

                _ => {
                    for event in events {
                        if !(converted_parents.contains(&event) || remaining.contains(&event)) {
                            tracing::trace!("added {event:?} to remaining");
                            remaining.push(event);
                        }
                    }
                }
            }
        }

        (converted, remaining)
    }

    fn convert_events<'a>(
        &'a self,
        events: Vec<&'a DebouncedEvent>,
    ) -> (Vec<fs_event::Event>, Vec<ConversionError>) {
        let (converted, errors): (Vec<_>, Vec<_>) = events
            .into_iter()
            .filter_map(|event| match self.convert_event(&event) {
                Ok(event) => event.map(|event| ConversionResult::Ok(event)),
                Err(kind) => Some(ConversionResult::Err(ConversionError {
                    events: vec![event],
                    kind,
                })),
            })
            .partition(|event| match event {
                ConversionResult::Ok(_) => true,
                ConversionResult::Err { .. } => false,
            });

        let converted = converted
            .into_iter()
            .map(|event| match event {
                ConversionResult::Ok(event) => event,
                _ => unreachable!("events are partitioned"),
            })
            .collect();

        let errors = errors
            .into_iter()
            .map(|event| match event {
                ConversionResult::Err(err) => err,
                _ => unreachable!("events are partitioned"),
            })
            .collect();

        (converted, errors)
    }

    fn convert_event(
        &self,
        event: &DebouncedEvent,
    ) -> Result<Option<fs_event::Event>, error::Process> {
        let time = event.time;
        let event = match event.kind {
            NotifyEventKind::Create(CreateKind::File) => {
                let [path] = &event.paths[..] else {
                    panic!("invalid paths");
                };

                let path = fs::canonicalize(path).unwrap();
                Some(fs_event::Event::new(fs_event::File::Created(path), time))
            }

            NotifyEventKind::Create(CreateKind::Folder) => {
                let [path] = &event.paths[..] else {
                    panic!("invalid paths");
                };

                let path = fs::canonicalize(path).unwrap();
                Some(fs_event::Event::new(fs_event::Folder::Created(path), time))
            }

            NotifyEventKind::Create(CreateKind::Any) => {
                let [path] = &event.paths[..] else {
                    panic!("invalid paths");
                };

                let path = fs::canonicalize(path).unwrap();
                if path.is_file() {
                    Some(fs_event::Event::new(fs_event::File::Created(path), time))
                } else if path.is_dir() {
                    Some(fs_event::Event::new(fs_event::Folder::Created(path), time))
                } else {
                    return Err(error::Process::UnknownFileType);
                }
            }

            NotifyEventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                // NB: Must check if paths exists due to operation of `notify` crate.
                // See https://github.com/notify-rs/notify/issues/554.
                let [path] = &event.paths[..] else {
                    panic!("invalid paths");
                };

                let path = normalize_path_root(path);
                if path.exists() {
                    if path.is_dir() {
                        Some(fs_event::Event::new(fs_event::Folder::Created(path), time))
                    } else if path.is_file() {
                        Some(fs_event::Event::new(fs_event::File::Created(path), time))
                    } else {
                        return Err(error::Process::UnknownFileType);
                    }
                } else {
                    Some(fs_event::Event::new(fs_event::Any::Removed(path), time))
                }
            }

            NotifyEventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                let [from, to] = &event.paths[..] else {
                    panic!("invalid paths");
                };

                let to = fs::canonicalize(to).unwrap();
                let from = normalize_path_root(from);
                if to.is_file() {
                    if to.parent() == from.parent() {
                        Some(fs_event::Event::new(
                            fs_event::File::Renamed { from, to },
                            time,
                        ))
                    } else {
                        Some(fs_event::Event::new(
                            fs_event::File::Moved { from, to },
                            time,
                        ))
                    }
                } else if to.is_dir() {
                    if to.parent() == from.parent() {
                        Some(fs_event::Event::new(
                            fs_event::Folder::Renamed { from, to },
                            time,
                        ))
                    } else {
                        Some(fs_event::Event::new(
                            fs_event::Folder::Moved { from, to },
                            time,
                        ))
                    }
                } else {
                    return Err(error::Process::UnknownFileType);
                }
            }

            NotifyEventKind::Modify(ModifyKind::Name(RenameMode::Any)) => {
                #[cfg(not(target_os = "macos"))]
                todo!();

                #[cfg(target_os = "macos")]
                /// Must check if paths exists due to operation of `notify` crate.
                /// See https://github.com/notify-rs/notify/issues/554.
                match &event.paths[..] {
                    [path] => {
                        if path.exists() {
                            if path.is_file() {
                                let path = fs::canonicalize(path).unwrap();
                                Some(fs_event::Event::new(fs_event::File::Other(path), time))
                            } else if path.is_dir() {
                                let path = fs::canonicalize(path).unwrap();
                                Some(fs_event::Event::new(fs_event::Folder::Other(path), time))
                            } else {
                                return Err(error::Process::UnknownFileType);
                            }
                        } else {
                            // TODO Could check file ids to get if file or folder.
                            Some(fs_event::Event::new(
                                fs_event::Any::Removed(path.clone()),
                                time,
                            ))
                        }
                    }

                    paths => todo!("unknown paths pattern: {paths:?}"),
                }
            }

            NotifyEventKind::Modify(ModifyKind::Data(_)) => {
                let [path] = &event.paths[..] else {
                    panic!("invalid paths");
                };

                let path = match fs::canonicalize(path) {
                    Ok(path) => path,
                    Err(err) => match err.kind() {
                        io::ErrorKind::NotFound => {
                            return Err(error::Process::NotFound);
                        }
                        _ => {
                            return Err(error::Process::Canonicalize);
                        }
                    },
                };

                if path.is_file() {
                    Some(fs_event::Event::new(
                        fs_event::File::DataModified(path),
                        time,
                    ))
                } else {
                    None
                }
            }

            NotifyEventKind::Modify(ModifyKind::Any) => {
                let [path] = &event.paths[..] else {
                    panic!("invalid paths");
                };

                let path = match fs::canonicalize(path) {
                    Ok(path) => path,
                    Err(err) => match err.kind() {
                        io::ErrorKind::NotFound => {
                            #[cfg(target_os = "windows")]
                            {
                                if path == self.app_config.user_manifest()
                                    || path == self.app_config.project_manifest()
                                    || path == self.app_config.local_config()
                                {
                                    let path = normalize_path_root(path);
                                    return Ok(Some(fs_event::Event::new(
                                        fs_event::File::Removed(path),
                                        event.time,
                                    )));
                                } else {
                                    return Err(error::Process::NotFound);
                                }
                            }

                            #[cfg(not(target_os = "windows"))]
                            return Err(error::Process::NotFound);
                        }

                        _ => {
                            return Err(error::Process::Canonicalize);
                        }
                    },
                };

                if path.is_file() {
                    Some(fs_event::Event::new(fs_event::File::Other(path), time))
                } else if path.is_dir() {
                    Some(fs_event::Event::new(fs_event::Folder::Other(path), time))
                } else {
                    return Err(error::Process::UnknownFileType);
                }
            }

            NotifyEventKind::Remove(RemoveKind::File) => {
                let [path] = &event.paths[..] else {
                    panic!("invalid paths");
                };

                let path = normalize_path_root(path);
                Some(fs_event::Event::new(fs_event::File::Removed(path), time))
            }

            NotifyEventKind::Remove(RemoveKind::Folder) => {
                let [path] = &event.paths[..] else {
                    panic!("invalid paths");
                };

                let path = normalize_path_root(path);
                Some(fs_event::Event::new(fs_event::Folder::Removed(path), time))
            }

            NotifyEventKind::Remove(RemoveKind::Any) => {
                let [path] = &event.paths[..] else {
                    panic!("invalid paths");
                };

                let path = normalize_path_root(path);
                Some(fs_event::Event::new(fs_event::Any::Removed(path), time))
            }

            event => unreachable!("unhandled event {event:?}"),
        };

        Ok(event)
    }

    /// Update file id cache based on events.
    /// See notify_debouncer_full::DebounceDataInner::add_event.
    fn update_file_ids(&self, events: Vec<&DebouncedEvent>) {
        let mut file_ids = self.file_ids.lock().unwrap();
        for event in events {
            let path = &event.paths[0];
            match event.kind {
                NotifyEventKind::Create(_) => file_ids.add_path(path),
                NotifyEventKind::Remove(_) => file_ids.remove_path(path),
                NotifyEventKind::Modify(ModifyKind::Name(rename_mode)) => match rename_mode {
                    RenameMode::Any => {
                        if path.exists() {
                            file_ids.add_path(path);
                        } else {
                            file_ids.remove_path(path);
                        }
                    }

                    RenameMode::Both => {
                        file_ids.remove_path(&event.paths[0]);
                        file_ids.add_path(&event.paths[1]);
                    }

                    RenameMode::From => {
                        file_ids.remove_path(path);
                    }

                    RenameMode::To => {
                        file_ids.add_path(path);
                    }

                    RenameMode::Other => {
                        // ignored
                    }
                },

                _ => {
                    if file_ids.cached_file_id(path).is_none() {
                        file_ids.add_path(path);
                    }
                }
            }
        }
    }

    fn file_id_from_watcher(&self, path: PathBuf) -> StdResult<Option<file_id::FileId>, ()> {
        let (tx, rx) = crossbeam::channel::bounded(1);
        if let Err(_err) = self.command_tx.send(WatcherCommand::FileId { path, tx }) {
            return Err(());
        }

        let id = match rx.recv() {
            Ok(id) => id,
            Err(_err) => {
                return Err(());
            }
        };

        Ok(id)
    }
}

#[cfg(target_os = "windows")]
impl FsWatcher {
    /// Remove modify events that are paired to a `Create` event.
    fn windows_filter_modify_events<'a>(
        mut events: Vec<&'a DebouncedEvent>,
    ) -> Vec<&'a DebouncedEvent> {
        use notify::EventKind;

        let len_orig = events.len();

        let create_events = events
            .iter()
            .filter_map(|event| {
                if matches!(event.kind, EventKind::Create(_)) {
                    let [path] = &event.paths[..] else {
                        panic!("invalid paths");
                    };

                    Some(path.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        events.retain(|event| match event.kind {
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => true,
            EventKind::Modify(_) => {
                let [path] = &event.paths[..] else {
                    panic!("invalid paths");
                };

                !create_events.iter().any(|create_path| path == create_path)
            }

            _ => true,
        });
        tracing::trace!("filtered {} nested events", len_orig - events.len());

        events
    }
}

/// If on Windows, convert to UNC if needed.
/// Otherwise, returns the given path.
fn normalize_path_root(path: impl Into<PathBuf>) -> PathBuf {
    if cfg!(target_os = "windows") {
        local_common::ensure_windows_unc(path)
    } else {
        path.into()
    }
}

/// Whether the file name matches that of a lock file.
/// i.e. .~<file_name>#
fn is_lock_file(file_name: impl AsRef<str>) -> bool {
    const START_PATTERN: &str = ".~";
    const END_PATTERN: &str = "#";

    let name: &str = file_name.as_ref();
    name.starts_with(START_PATTERN) && name.ends_with(END_PATTERN)
}

#[cfg(test)]
#[path = "notify_processor_test.rs"]
mod notify_processor_test;
