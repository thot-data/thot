use super::{super::ConversionError, config, FsWatcher};
use crate::{error, event as app, server::event as fs_event, Event, EventKind};
use rayon::{iter::Either, prelude::*};
use std::{path::PathBuf, result::Result as StdResult, time::Instant};
use syre_local as local;
use uuid::Uuid;

impl FsWatcher {
    /// Convert [file system events](fs_event::Event) to [app events](Event).
    ///
    /// # Returns
    /// Tuple of (events, errors).
    pub fn process_events_fs_to_app<'a>(
        &self,
        events: Vec<fs_event::Event<'a>>,
    ) -> (Vec<Event>, Vec<ConversionError<'a>>) {
        let (converted, errors): (Vec<_>, Vec<_>) =
            events.into_par_iter().partition_map(|fs_event| {
                match self.process_event_fs_to_apps(&fs_event) {
                    Ok(events) => Either::Left(events),
                    Err(err) => Either::Right(ConversionError {
                        events: fs_event.parents(),
                        kind: err.into(),
                    }),
                }
            });

        let converted = converted.into_iter().flatten().collect();
        (converted, errors)
    }

    fn process_event_fs_to_apps(
        &self,
        event: &fs_event::Event,
    ) -> StdResult<Vec<Event>, error::processing::Error> {
        let events = match &event.kind {
            fs_event::EventKind::File(fs_event::File::Created(path)) => {
                let event = match self.handle_file_created(&path, &self.app_config) {
                    Ok(kind) => Event::with_time(kind, event.time, event.id().clone())
                        .add_path(path.clone()),
                    Err(err) => Event::with_time(
                        EventKind::File(app::ResourceEvent::Created),
                        event.time,
                        event.id().clone(),
                    )
                    .add_path(path.clone()),
                };

                vec![event]
            }

            fs_event::EventKind::File(fs_event::File::Removed(path)) => {
                let event = match self.handle_file_removed(&path, &self.app_config) {
                    Ok(kind) => Event::with_time(kind, event.time, event.id().clone())
                        .add_path(path.clone()),
                    Err(err) => Event::with_time(
                        EventKind::File(app::ResourceEvent::Removed),
                        event.time,
                        event.id().clone(),
                    )
                    .add_path(path.clone()),
                };

                vec![event]
            }

            fs_event::EventKind::File(fs_event::File::Moved { from, to }) => {
                Self::handle_file_moved(
                    from.clone(),
                    to.clone(),
                    event.time,
                    event.id().clone(),
                    event.parents(),
                    &self.app_config,
                )
            }

            fs_event::EventKind::File(fs_event::File::Renamed { from, to }) => {
                Self::handle_file_renamed(
                    from.clone(),
                    to.clone(),
                    event.time,
                    event.id().clone(),
                    &self.app_config,
                )?
            }

            fs_event::EventKind::File(fs_event::File::DataModified(path)) => {
                let event = match Self::handle_file_data_modified(&path, &self.app_config) {
                    Ok(kind) => Event::with_time(kind, event.time, event.id().clone())
                        .add_path(path.clone()),
                    Err(err) => Event::with_time(
                        EventKind::File(app::ResourceEvent::Removed),
                        event.time,
                        event.id().clone(),
                    )
                    .add_path(path.clone()),
                };

                vec![event]
            }

            fs_event::EventKind::File(fs_event::File::Other(path)) => {
                let event = match Self::handle_file_other(&path, &self.app_config) {
                    Ok(kind) => Event::with_time(kind, event.time, event.id().clone())
                        .add_path(path.clone()),
                    Err(err) => Event::with_time(
                        EventKind::File(app::ResourceEvent::Modified(app::ModifiedKind::Other)),
                        event.time,
                        event.id().clone(),
                    )
                    .add_path(path.clone()),
                };

                vec![event]
            }

            fs_event::EventKind::Folder(fs_event::Folder::Created(path)) => {
                let event = match self.handle_folder_created(&path) {
                    Ok(kind) => Event::with_time(kind, event.time, event.id().clone())
                        .add_path(path.clone()),
                    Err(err) => {
                        tracing::error!(?err);
                        Event::with_time(
                            EventKind::Folder(app::ResourceEvent::Created),
                            event.time,
                            event.id().clone(),
                        )
                        .add_path(path.clone())
                    }
                };

                vec![event]
            }

            fs_event::EventKind::Folder(fs_event::Folder::Removed(path)) => {
                let event = match self.handle_folder_removed(&path) {
                    Ok(kind) => Event::with_time(kind, event.time, event.id().clone())
                        .add_path(path.clone()),
                    Err(err) => {
                        if matches!(err.kind(), resources::ErrorKind::NotInProject) {
                            if let Ok(manifest) = self.app_config.load_project_manifest() {
                                if manifest.contains(&path) {
                                    return Ok(vec![Event::with_time(
                                        app::Project::FolderRemoved.into(),
                                        event.time,
                                        event.id().clone(),
                                    )
                                    .add_path(path.clone())]);
                                }

                                if let Some(parent) = path.parent() {
                                    let parent = parent.to_path_buf();
                                    if manifest.contains(&parent) {
                                        if let Some(file_name) = path.file_name() {
                                            if file_name == local::common::app_dir() {
                                                return Ok(vec![Event::with_time(
                                                    app::Project::ConfigDir(
                                                        app::StaticResourceEvent::Removed,
                                                    )
                                                    .into(),
                                                    event.time,
                                                    event.id().clone(),
                                                )
                                                .add_path(path.clone())]);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        tracing::error!(?err);
                        Event::with_time(
                            app::EventKind::Folder(app::ResourceEvent::Removed),
                            event.time,
                            event.id().clone(),
                        )
                    }
                };

                vec![event]
            }

            fs_event::EventKind::Folder(fs_event::Folder::Moved { from, to }) => {
                self.handle_folder_moved(from.clone(), to.clone(), event.time, event.id().clone())
            }

            fs_event::EventKind::Folder(fs_event::Folder::Renamed { from, to }) => {
                assert!(
                    from.parent() == to.parent(),
                    "renamed paths should have same parent"
                );

                self.handle_folder_renamed(
                    from.clone(),
                    to.clone(),
                    event.time,
                    event.id().clone(),
                )?
            }

            fs_event::EventKind::Folder(fs_event::Folder::Other(path)) => vec![Event::with_time(
                app::EventKind::Folder(app::ResourceEvent::Modified(app::ModifiedKind::Other)),
                event.time,
                event.id().clone(),
            )
            .add_path(path.clone())],

            fs_event::EventKind::Any(fs_event::Any::Removed(path)) => {
                assert!(!path.exists());
                let maybe_file_kind = resources::resource_kind(path, &self.app_config);
                let maybe_folder_kind = resources::dir_kind(path);

                let event = match (maybe_file_kind, maybe_folder_kind) {
                    (
                        Ok(Some(resources::ResourceEvent::Analysis {
                            project: project_file,
                        })),
                        Ok(resources::DirKind::None {
                            project: project_dir,
                        }),
                    ) => {
                        assert_eq!(project_file, project_dir);

                        Event::with_time(
                            app::EventKind::AnalysisFile(app::ResourceEvent::Removed),
                            event.time,
                            event.id().clone(),
                        )
                        .add_path(path.clone())
                    }

                    (Ok(_), Ok(_)) => Event::with_time(
                        app::GraphResource::Removed.into(),
                        event.time,
                        event.id().clone(),
                    )
                    .add_path(path.clone()),

                    (Ok(file_kind), Err(_)) => {
                        if let Ok(kind) = self.convert_file_removed(path, Ok(file_kind)) {
                            Event::with_time(kind, event.time, event.id().clone())
                                .add_path(path.clone())
                        } else {
                            Event::with_time(
                                app::Any::Removed.into(),
                                event.time,
                                event.id().clone(),
                            )
                            .add_path(path.clone())
                        }
                    }

                    (Err(_), Ok(kind)) => Event::with_time(
                        self.convert_folder_removed(kind),
                        event.time,
                        event.id().clone(),
                    )
                    .add_path(path.clone()),

                    (Err(_), Err(_)) => {
                        Event::with_time(app::Any::Removed.into(), event.time, event.id().clone())
                            .add_path(path.clone())
                    }
                };

                vec![event]
            }
        };

        Ok(events)
    }

    fn handle_file_created(
        &self,
        path: &PathBuf,
        config: &config::Config,
    ) -> StdResult<EventKind, resources::Error> {
        let kind = match resources::resource_kind(path, config) {
            Ok(Some(kind)) => Self::convert_resource_to_event_kind_created(kind),
            Ok(None) => EventKind::File(app::ResourceEvent::Created),
            Err(err) => match err.kind() {
                resources::error::ErrorKind::NotInProject => {
                    let roots = self.roots.lock().unwrap();
                    let project = roots
                        .iter()
                        .find(|project| path.starts_with(project))
                        .expect("event should not be triggered if not in a root");

                    assert_ne!(
                        *path,
                        local::common::project_file_of(&project),
                        "NotInProject error indicates project file does not exist"
                    );
                    if *path == local::common::project_settings_file_of(&project) {
                        app::Project::Settings(app::StaticResourceEvent::Created).into()
                    } else if *path == local::common::analyses_file_of(&project) {
                        app::Project::Analyses(app::StaticResourceEvent::Created).into()
                    } else {
                        return Err(err);
                    }
                }
                resources::error::ErrorKind::LoadProject(_) => {
                    let project = syre_local::project::project::project_root_path(path)
                        .expect("LoadProject error indicates we are in a project");

                    if *path == local::common::project_file_of(&project) {
                        app::Project::Properties(app::StaticResourceEvent::Created).into()
                    } else if *path == local::common::project_settings_file_of(&project) {
                        app::Project::Settings(app::StaticResourceEvent::Created).into()
                    } else if *path == local::common::analyses_file_of(&project) {
                        app::Project::Analyses(app::StaticResourceEvent::Created).into()
                    } else {
                        return Err(err);
                    }
                }
                _ => return Err(err),
            },
        };

        Ok(kind)
    }

    fn handle_file_removed(
        &self,
        path: &PathBuf,
        app_config: &config::Config,
    ) -> StdResult<EventKind, resources::Error> {
        self.convert_file_removed(path, resources::resource_kind(path, app_config))
    }

    fn convert_file_removed(
        &self,
        path: &PathBuf,
        kind: Result<Option<resources::ResourceEvent>, resources::Error>,
    ) -> StdResult<EventKind, resources::Error> {
        let kind = match kind {
            Ok(Some(kind)) => Self::convert_resource_to_event_kind_removed(kind),
            Ok(None) => EventKind::File(app::ResourceEvent::Removed),
            Err(err) => match err.kind() {
                resources::error::ErrorKind::NotInProject => {
                    let roots = self.roots.lock().unwrap();
                    let project = roots
                        .iter()
                        .find(|project| path.starts_with(project))
                        .expect("event should not be triggered if not in a root");

                    if *path == local::common::project_file_of(&project) {
                        app::Project::Properties(app::StaticResourceEvent::Removed).into()
                    } else if *path == local::common::project_settings_file_of(&project) {
                        app::Project::Settings(app::StaticResourceEvent::Removed).into()
                    } else if *path == local::common::analyses_file_of(&project) {
                        app::Project::Analyses(app::StaticResourceEvent::Removed).into()
                    } else {
                        return Err(err);
                    }
                }
                resources::error::ErrorKind::LoadProject(_) => {
                    let project = syre_local::project::project::project_root_path(path)
                        .expect("LoadProject error indicates the path is in a project");

                    assert_ne!(
                        *path,
                        local::common::project_file_of(&project),
                        "LoadProject error indicates the path is in a project, requiring a project file to be present."
                    );
                    if *path == local::common::project_settings_file_of(&project) {
                        app::Project::Settings(app::StaticResourceEvent::Removed).into()
                    } else if *path == local::common::analyses_file_of(&project) {
                        app::Project::Analyses(app::StaticResourceEvent::Removed).into()
                    } else {
                        return Err(err);
                    }
                }
                _ => return Err(err),
            },
        };

        Ok(kind)
    }

    fn handle_file_moved(
        from: PathBuf,
        to: PathBuf,
        time: Instant,
        parent: Uuid,
        parent_events: Vec<&notify_debouncer_full::DebouncedEvent>,
        app_config: &config::Config,
    ) -> Vec<Event> {
        let from_kind = resources::resource_kind(&from, app_config);
        let to_kind = resources::resource_kind(&to, app_config);
        match (from_kind, to_kind) {
            (Err(from_err), Err(to_err)) => {
                vec![
                    Event::with_time(EventKind::File(app::ResourceEvent::Moved), time, parent)
                        .add_path(from.clone())
                        .add_path(to.clone()),
                ]
            }

            (Ok(from_kind), Err(to_err)) => {
                if let Some(from_kind) = from_kind {
                    let kind = Self::convert_resource_to_event_kind_moved_from(from_kind);
                    vec![Event::with_time(kind, time, parent)
                        .add_path(from)
                        .add_path(to)]
                } else {
                    vec![
                        Event::with_time(EventKind::File(app::ResourceEvent::Moved), time, parent)
                            .add_path(from.clone())
                            .add_path(to.clone()),
                    ]
                }
            }

            (Err(from_err), Ok(to_kind)) => {
                if let Some(to_kind) = to_kind {
                    let kind = Self::convert_resource_to_event_kind_moved_to(to_kind);
                    vec![Event::with_time(kind, time, parent)
                        .add_path(from)
                        .add_path(to)]
                } else {
                    vec![
                        Event::with_time(EventKind::File(app::ResourceEvent::Moved), time, parent)
                            .add_path(from.clone())
                            .add_path(to.clone()),
                    ]
                }
            }

            (Ok(from_kind), Ok(to_kind)) => match (from_kind, to_kind) {
                (None, None) => {
                    vec![
                        Event::with_time(EventKind::File(app::ResourceEvent::Moved), time, parent)
                            .add_path(from.clone())
                            .add_path(to.clone()),
                    ]
                }

                (Some(from_kind), None) => {
                    let kind = Self::convert_resource_to_event_kind_moved_from(from_kind);
                    vec![Event::with_time(kind, time, parent)
                        .add_path(from)
                        .add_path(to)]
                }

                (None, Some(to_kind)) => {
                    let kind = if matches!(parent_events[1].kind, notify::EventKind::Create(_)) {
                        Self::convert_resource_to_event_kind_created(to_kind)
                    } else {
                        Self::convert_resource_to_event_kind_moved_to(to_kind)
                    };

                    vec![Event::with_time(kind, time, parent)
                        .add_path(from)
                        .add_path(to)]
                }

                (Some(from_kind), Some(to_kind)) => Self::convert_resource_to_event_kind_moved(
                    from_kind, to_kind, from, to, time, parent,
                ),
            },
        }
    }

    fn handle_file_renamed(
        from: PathBuf,
        to: PathBuf,
        time: Instant,
        parent: Uuid,
        app_config: &config::Config,
    ) -> StdResult<Vec<Event>, error::processing::Error> {
        let from_kind = resources::resource_kind(&from, app_config);
        let to_kind = resources::resource_kind(&to, app_config);
        match (from_kind, to_kind) {
            (Err(from_err), Err(to_err)) => {
                if to_err.kind() != from_err.kind() {
                    return Err(error::processing::Error::InvalidState(format!(
                        "rename errors differ. from: {from_err:?}. to: {to_err:?}."
                    )));
                }

                let event =
                    Event::with_time(EventKind::File(app::ResourceEvent::Renamed), time, parent)
                        .add_path(from.clone())
                        .add_path(to.clone());

                Ok(vec![event])
            }

            (Ok(_from_kind), Err(to_err)) => {
                return Err(error::processing::Error::InvalidState(format!(
                    "rename errors differ. from: Ok. to: {to_err:?}."
                )));
            }

            (Err(from_err), Ok(_to_kind)) => {
                return Err(error::processing::Error::InvalidState(format!(
                    "rename errors differ. from: {from_err:?}. to: Ok."
                )));
            }

            (Ok(from_kind), Ok(to_kind)) => match (from_kind, to_kind) {
                (None, None) => Ok(vec![Event::with_time(
                    EventKind::File(app::ResourceEvent::Renamed),
                    time,
                    parent,
                )
                .add_path(from.clone())
                .add_path(to.clone())]),

                (Some(from_kind), None) => {
                    let kind = Self::convert_resource_to_event_kind_renamed_from(from_kind);
                    Ok(vec![Event::with_time(kind, time, parent)
                        .add_path(from)
                        .add_path(to)])
                }

                (None, Some(to_kind)) => {
                    let kind = Self::convert_resource_to_event_kind_renamed_to(to_kind);
                    Ok(vec![Event::with_time(kind, time, parent)
                        .add_path(from)
                        .add_path(to)])
                }

                (Some(from_kind), Some(to_kind)) => Self::convert_resource_to_event_kind_renamed(
                    from_kind, to_kind, from, to, time, parent,
                ),
            },
        }
    }

    fn handle_file_data_modified(
        path: &PathBuf,
        app_config: &config::Config,
    ) -> StdResult<EventKind, resources::Error> {
        let kind = match resources::resource_kind(path, app_config)? {
            Some(kind) => Self::convert_resource_to_event_kind_data_modified(kind),
            None => app::EventKind::File(app::ResourceEvent::Modified(app::ModifiedKind::Data)),
        };

        Ok(kind)
    }

    fn handle_file_other(
        path: &PathBuf,
        app_config: &config::Config,
    ) -> StdResult<EventKind, resources::Error> {
        let kind = match resources::resource_kind(path, app_config)? {
            Some(kind) => Self::convert_resource_to_event_kind_other(kind),
            None => app::EventKind::File(app::ResourceEvent::Modified(app::ModifiedKind::Other)),
        };

        Ok(kind)
    }

    fn handle_folder_created(&self, path: &PathBuf) -> StdResult<EventKind, resources::Error> {
        use resources::error::ErrorKind;

        assert!(path.exists());
        let kind = match resources::dir_kind(path) {
            Ok(resources::DirKind::None { .. }) => {
                app::EventKind::Folder(app::ResourceEvent::Created)
            }
            Ok(resources::DirKind::ContainerLike { .. }) => {
                if local::common::container_file_of(path).exists() {
                    app::Graph::Created.into()
                } else {
                    app::EventKind::Folder(app::ResourceEvent::Created)
                }
            }
            Ok(kind) => Self::convert_dir_to_event_kind_created(&kind),
            Err(err) if matches!(err.kind(), resources::error::ErrorKind::NotInProject) => {
                let projects = match self.app_config.load_project_manifest() {
                    Ok(projects) => projects,
                    Err(err) => {
                        return Err(resources::error::Error::new(
                            path.clone(),
                            ErrorKind::LoadProjectManifest(err),
                        ));
                    }
                };

                let Some(project) = projects.iter().find(|project| path.starts_with(project))
                else {
                    return Err(resources::error::Error::new(
                        path.clone(),
                        ErrorKind::NotInProject,
                    ));
                };

                if path == project {
                    app::Project::Created.into()
                } else if path == &syre_local::common::app_dir_of(project) {
                    app::Project::ConfigDir(app::StaticResourceEvent::Created).into()
                } else {
                    return Err(resources::error::Error::new(
                        path.clone(),
                        ErrorKind::NotInProject,
                    ));
                }
            }
            Err(err) => return Err(err),
        };

        Ok(kind)
    }

    fn handle_folder_removed(&self, path: &PathBuf) -> StdResult<EventKind, resources::Error> {
        assert!(!path.exists());
        let kind = resources::dir_kind(path)?;
        Ok(self.convert_folder_removed(kind))
    }

    fn convert_folder_removed(&self, kind: resources::DirKind) -> EventKind {
        match kind {
            resources::DirKind::None { .. } => app::EventKind::Folder(app::ResourceEvent::Removed),
            kind => Self::convert_dir_to_event_kind_removed(&kind),
        }
    }

    /// Handles a moved folder
    fn handle_folder_moved(
        &self,
        from: PathBuf,
        to: PathBuf,
        time: Instant,
        parent: Uuid,
    ) -> Vec<Event> {
        assert!(
            from.parent() != to.parent(),
            "moved paths should have different parent"
        );

        let from_kind = resources::dir_kind(&from);
        let to_kind = resources::dir_kind(&to);
        match (from_kind, to_kind) {
            (Err(from_err), Err(to_err)) => {
                vec![
                    Event::with_time(EventKind::Folder(app::ResourceEvent::Moved), time, parent)
                        .add_path(from.clone())
                        .add_path(to.clone()),
                ]
            }

            (Ok(from_kind), Err(to_err)) => {
                assert!(!matches!(from_kind, resources::DirKind::Container { .. }));
                if matches!(
                    from_kind,
                    resources::DirKind::Project {
                        kind: resources::ProjectDir::Root,
                        ..
                    }
                ) {
                    tracing::warn!("UNUSUAL SITUATION: project moved and replaced");
                    vec![
                        Event::with_time(app::Project::Moved.into(), time, parent)
                            .add_path(from.clone())
                            .add_path(to.clone()),
                        Event::with_time(app::Project::Modified.into(), time, parent)
                            .add_path(from),
                    ]
                } else if matches!(to_err.kind(), resources::ErrorKind::NotInProject) {
                    let kind = Self::convert_dir_to_event_kind_moved_from_project(&from_kind);
                    vec![Event::with_time(kind, time, parent)
                        .add_path(from)
                        .add_path(to)]
                } else {
                    vec![Event::with_time(
                        EventKind::Folder(app::ResourceEvent::Moved),
                        time,
                        parent,
                    )
                    .add_path(from)
                    .add_path(to)]
                }
            }

            (Err(from_err), Ok(to_kind)) => {
                if matches!(
                    to_kind,
                    resources::DirKind::Project {
                        kind: resources::ProjectDir::Root,
                        ..
                    },
                ) {
                    vec![Event::with_time(app::Project::Moved.into(), time, parent)
                        .add_path(from)
                        .add_path(to)]
                } else if matches!(from_err.kind(), resources::ErrorKind::NotInProject) {
                    let kind = Self::convert_dir_to_event_kind_moved_to_project(&to_kind);
                    vec![Event::with_time(kind, time, parent)
                        .add_path(from)
                        .add_path(to)]
                } else {
                    vec![Event::with_time(
                        EventKind::Folder(app::ResourceEvent::Moved),
                        time,
                        parent,
                    )
                    .add_path(from)
                    .add_path(to)]
                }
            }

            (Ok(from_kind), Ok(to_kind)) => {
                assert!(!matches!(from_kind, resources::DirKind::Container { .. }));
                match (from_kind, to_kind) {
                    (resources::DirKind::None { .. }, resources::DirKind::None { .. }) => {
                        vec![Event::with_time(
                            EventKind::Folder(app::ResourceEvent::Moved),
                            time,
                            parent,
                        )
                        .add_path(from.clone())
                        .add_path(to.clone())]
                    }

                    (from_kind, resources::DirKind::None { .. }) => {
                        let kind = Self::convert_dir_to_event_kind_moved_from(&from_kind);
                        vec![Event::with_time(kind, time, parent)
                            .add_path(from)
                            .add_path(to)]
                    }

                    (
                        resources::DirKind::None {
                            project: from_project,
                        },
                        to_kind,
                    ) => match to_kind {
                        resources::DirKind::ContainerLike {
                            project: to_project,
                        } => {
                            if from_project == to_project {
                                vec![Event::with_time(
                                    EventKind::Folder(app::ResourceEvent::Moved),
                                    time,
                                    parent,
                                )
                                .add_path(from.clone())
                                .add_path(to.clone())]
                            } else {
                                vec![
                                    Event::with_time(
                                        EventKind::Folder(app::ResourceEvent::Removed),
                                        time,
                                        parent,
                                    )
                                    .add_path(from.clone()),
                                    Event::with_time(
                                        EventKind::Folder(app::ResourceEvent::Created),
                                        time,
                                        parent,
                                    )
                                    .add_path(to.clone()),
                                ]
                            }
                        }
                        _ => {
                            let kind = Self::convert_dir_to_event_kind_moved_to(&to_kind);
                            vec![Event::with_time(kind, time, parent)
                                .add_path(from)
                                .add_path(to)]
                        }
                    },

                    (from_kind, to_kind) => Self::convert_dir_to_event_kind_moved(
                        from_kind, to_kind, from, to, time, parent,
                    ),
                }
            }
        }
    }

    fn handle_folder_renamed(
        &self,
        from: PathBuf,
        to: PathBuf,
        time: Instant,
        parent: Uuid,
    ) -> StdResult<Vec<Event>, error::processing::Error> {
        assert!(
            from.parent() == to.parent(),
            "renamed paths should have same parent"
        );

        let from_kind = resources::dir_kind(&from);
        let to_kind = resources::dir_kind(&to);
        tracing::debug!(?from_kind, ?to_kind);
        match (from_kind, to_kind) {
            (Err(from_err), Err(to_err)) => {
                if to_err.kind() != from_err.kind() {
                    return Err(error::processing::Error::InvalidState(format!(
                        "rename errors differ. from: {from_err:?}. to: {to_err:?}."
                    )));
                }

                let event =
                    Event::with_time(EventKind::Folder(app::ResourceEvent::Renamed), time, parent)
                        .add_path(from)
                        .add_path(to);

                Ok(vec![event])
            }

            (Ok(from_kind), Err(to_err)) => {
                assert!(!matches!(from_kind, resources::DirKind::Container { .. }));
                if matches!(
                    (from_kind, to_err.kind()),
                    (
                        resources::DirKind::Project {
                            kind: resources::ProjectDir::Root,
                            ..
                        },
                        resources::ErrorKind::NotInProject
                    )
                ) {
                    Ok(vec![
                        Event::with_time(app::Project::Modified.into(), time, parent)
                            .add_path(from),
                        Event::with_time(app::Project::Modified.into(), time, parent).add_path(to),
                    ])
                } else {
                    return Err(error::processing::Error::InvalidState(format!(
                        "rename errors differ. from: Ok. to: {to_err:?}."
                    )));
                }
            }

            (Err(from_err), Ok(to_kind)) => {
                if matches!(
                    (from_err.kind(), to_kind),
                    (
                        resources::ErrorKind::NotInProject,
                        resources::DirKind::Project {
                            kind: resources::ProjectDir::Root,
                            ..
                        }
                    )
                ) {
                    Ok(vec![Event::with_time(
                        app::Project::Moved.into(),
                        time,
                        parent,
                    )
                    .add_path(from)
                    .add_path(to)])
                } else {
                    return Err(error::processing::Error::InvalidState(format!(
                        "rename errors differ. from: {from_err:?}. to: Ok."
                    )));
                }
            }

            (Ok(from_kind), Ok(to_kind)) => {
                assert!(!matches!(from_kind, resources::DirKind::Container { .. }));
                match (from_kind, to_kind) {
                    (resources::DirKind::None { .. }, resources::DirKind::None { .. }) => {
                        Ok(vec![Event::with_time(
                            EventKind::Folder(app::ResourceEvent::Renamed),
                            time,
                            parent,
                        )
                        .add_path(from.clone())
                        .add_path(to.clone())])
                    }

                    (from_kind, resources::DirKind::None { .. }) => {
                        assert!(
                            !matches!(
                                from_kind,
                                resources::DirKind::Project {
                                    kind: resources::ProjectDir::Root,
                                    ..
                                }
                            ),
                            "renaming project should result in destination being a project"
                        );
                        assert!(
                            !matches!(from_kind, resources::DirKind::ContainerLike {  .. }),
                            "renaming a container like should not result in it not beign a resource");

                        match from_kind {
                            resources::DirKind::Project {
                                kind: resources::ProjectDir::Analysis,
                                ..
                            } => Ok(vec![Event::with_time(
                                app::Project::AnalysisDir(app::ResourceEvent::Renamed).into(),
                                time,
                                parent,
                            )
                            .add_path(from)
                            .add_path(to)]),

                            resources::DirKind::Project {
                                kind: resources::ProjectDir::Data,
                                ..
                            } => Ok(vec![Event::with_time(
                                app::Project::DataDir(app::ResourceEvent::Renamed).into(),
                                time,
                                parent,
                            )
                            .add_path(from)
                            .add_path(to)]),
                            _ => {
                                let kind =
                                    Self::convert_dir_to_event_kind_renamed_from(&from_kind)?;
                                Ok(vec![Event::with_time(kind, time, parent)
                                    .add_path(from)
                                    .add_path(to)])
                            }
                        }
                    }

                    (resources::DirKind::None { .. }, to_kind) => {
                        assert!(
                            !matches!(
                                to_kind,
                                resources::DirKind::Project {
                                    kind: resources::ProjectDir::Root,
                                    ..
                                }
                            ),
                            "renaming project should result in error at original location"
                        );

                        let kind = Self::convert_dir_to_event_kind_renamed_to(&to_kind);
                        Ok(vec![Event::with_time(kind, time, parent)
                            .add_path(from)
                            .add_path(to)])
                    }

                    (from_kind, to_kind) => Self::convert_dir_to_event_kind_renamed(
                        from_kind, to_kind, from, to, time, parent,
                    ),
                }
            }
        }
    }
}

impl FsWatcher {
    fn convert_resource_to_event_kind_created(kind: resources::ResourceEvent) -> EventKind {
        match kind {
            resources::ResourceEvent::Config(kind) => match kind {
                resources::Config::ProjectManifest => EventKind::Config(
                    app::Config::ProjectManifest(app::StaticResourceEvent::Created),
                ),
                resources::Config::UserManifest => {
                    EventKind::Config(app::Config::UserManifest(app::StaticResourceEvent::Created))
                }
                resources::Config::LocalConfig => {
                    EventKind::Config(app::Config::LocalConfig(app::StaticResourceEvent::Created))
                }
            },

            resources::ResourceEvent::Project { kind, .. } => match kind {
                resources::Project::Properties => {
                    app::Project::Properties(app::StaticResourceEvent::Created).into()
                }

                resources::Project::Settings => {
                    app::Project::Settings(app::StaticResourceEvent::Created).into()
                }

                resources::Project::Analyses => {
                    app::Project::Analyses(app::StaticResourceEvent::Created).into()
                }
            },

            resources::ResourceEvent::Container { kind, .. } => match kind {
                resources::Container::Properties => {
                    app::Container::Properties(app::StaticResourceEvent::Created).into()
                }

                resources::Container::Settings => {
                    app::Container::Settings(app::StaticResourceEvent::Created).into()
                }

                resources::Container::Assets => {
                    app::Container::Assets(app::StaticResourceEvent::Created).into()
                }
            },

            resources::ResourceEvent::Analysis { .. } => {
                app::EventKind::AnalysisFile(app::ResourceEvent::Created)
            }

            resources::ResourceEvent::Asset { .. } => {
                app::EventKind::AssetFile(app::ResourceEvent::Created)
            }
        }
    }

    fn convert_resource_to_event_kind_removed(kind: resources::ResourceEvent) -> EventKind {
        match kind {
            resources::ResourceEvent::Config(kind) => match kind {
                resources::Config::ProjectManifest => EventKind::Config(
                    app::Config::ProjectManifest(app::StaticResourceEvent::Removed),
                ),
                resources::Config::UserManifest => {
                    EventKind::Config(app::Config::UserManifest(app::StaticResourceEvent::Removed))
                }
                resources::Config::LocalConfig => {
                    EventKind::Config(app::Config::LocalConfig(app::StaticResourceEvent::Removed))
                }
            },

            resources::ResourceEvent::Project { kind, .. } => match kind {
                resources::Project::Properties => {
                    app::Project::Properties(app::StaticResourceEvent::Removed).into()
                }

                resources::Project::Settings => {
                    app::Project::Settings(app::StaticResourceEvent::Removed).into()
                }

                resources::Project::Analyses => {
                    app::Project::Analyses(app::StaticResourceEvent::Removed).into()
                }
            },

            resources::ResourceEvent::Container { kind, .. } => match kind {
                resources::Container::Properties => {
                    app::Container::Properties(app::StaticResourceEvent::Removed).into()
                }

                resources::Container::Settings => {
                    app::Container::Settings(app::StaticResourceEvent::Removed).into()
                }

                resources::Container::Assets => {
                    app::Container::Assets(app::StaticResourceEvent::Removed).into()
                }
            },

            resources::ResourceEvent::Analysis { .. } => {
                app::EventKind::AnalysisFile(app::ResourceEvent::Removed)
            }

            resources::ResourceEvent::Asset { .. } => {
                app::EventKind::AssetFile(app::ResourceEvent::Removed)
            }
        }
    }

    fn convert_resource_to_event_kind_moved_from(kind: resources::ResourceEvent) -> EventKind {
        match kind {
            resources::ResourceEvent::Config(kind) => match kind {
                resources::Config::ProjectManifest => EventKind::Config(
                    app::Config::ProjectManifest(app::StaticResourceEvent::Removed),
                ),
                resources::Config::UserManifest => {
                    EventKind::Config(app::Config::UserManifest(app::StaticResourceEvent::Removed))
                }
                resources::Config::LocalConfig => {
                    EventKind::Config(app::Config::LocalConfig(app::StaticResourceEvent::Removed))
                }
            },

            resources::ResourceEvent::Project { kind, .. } => match kind {
                resources::Project::Properties => {
                    app::Project::Properties(app::StaticResourceEvent::Removed).into()
                }

                resources::Project::Settings => {
                    app::Project::Settings(app::StaticResourceEvent::Removed).into()
                }

                resources::Project::Analyses => {
                    app::Project::Analyses(app::StaticResourceEvent::Removed).into()
                }
            },

            resources::ResourceEvent::Container { kind, .. } => match kind {
                resources::Container::Properties => {
                    app::Container::Properties(app::StaticResourceEvent::Removed).into()
                }

                resources::Container::Settings => {
                    app::Container::Settings(app::StaticResourceEvent::Removed).into()
                }

                resources::Container::Assets => {
                    app::Container::Assets(app::StaticResourceEvent::Removed).into()
                }
            },

            resources::ResourceEvent::Analysis { .. } => {
                app::EventKind::AnalysisFile(app::ResourceEvent::Removed)
            }

            resources::ResourceEvent::Asset { .. } => {
                app::EventKind::AssetFile(app::ResourceEvent::Removed)
            }
        }
    }

    /// Non-resource moved into a resource.
    fn convert_resource_to_event_kind_moved_to(kind: resources::ResourceEvent) -> EventKind {
        match kind {
            resources::ResourceEvent::Config(kind) => match kind {
                resources::Config::ProjectManifest => {
                    EventKind::Config(app::Config::ProjectManifest(
                        app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                    ))
                }
                resources::Config::UserManifest => EventKind::Config(app::Config::UserManifest(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )),
                resources::Config::LocalConfig => EventKind::Config(app::Config::LocalConfig(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )),
            },

            resources::ResourceEvent::Project { kind, .. } => match kind {
                resources::Project::Properties => app::Project::Properties(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),

                resources::Project::Settings => app::Project::Settings(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),

                resources::Project::Analyses => app::Project::Analyses(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),
            },

            resources::ResourceEvent::Container { kind, .. } => match kind {
                resources::Container::Properties => app::Container::Properties(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),

                resources::Container::Settings => app::Container::Settings(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),

                resources::Container::Assets => app::Container::Assets(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),
            },

            resources::ResourceEvent::Analysis { .. } => {
                app::EventKind::AnalysisFile(app::ResourceEvent::Modified(app::ModifiedKind::Other))
            }

            resources::ResourceEvent::Asset { .. } => {
                app::EventKind::AssetFile(app::ResourceEvent::Modified(app::ModifiedKind::Other))
            }
        }
    }

    fn convert_resource_to_event_kind_moved(
        from_kind: resources::ResourceEvent,
        to_kind: resources::ResourceEvent,
        from: PathBuf,
        to: PathBuf,
        time: Instant,
        parent: Uuid,
    ) -> Vec<Event> {
        assert!(
            from.parent() != to.parent(),
            "moved paths should have different parent"
        );

        match (from_kind, to_kind) {
            (
                resources::ResourceEvent::Asset {
                    project: from_project,
                },
                resources::ResourceEvent::Asset {
                    project: to_project,
                },
            ) => {
                let kind = if from_project == to_project {
                    EventKind::AssetFile(app::ResourceEvent::Moved)
                } else {
                    EventKind::AssetFile(app::ResourceEvent::MovedProject)
                };

                vec![Event::with_time(kind, time, parent)
                    .add_path(from)
                    .add_path(to)]
            }

            (
                resources::ResourceEvent::Analysis {
                    project: from_project,
                },
                resources::ResourceEvent::Analysis {
                    project: to_project,
                },
            ) => {
                let kind = if from_project == to_project {
                    EventKind::AnalysisFile(app::ResourceEvent::Moved)
                } else {
                    EventKind::AnalysisFile(app::ResourceEvent::MovedProject)
                };

                vec![Event::with_time(kind, time, parent)
                    .add_path(from)
                    .add_path(to)]
            }

            (from_kind, to_kind) => {
                let from_kind = Self::convert_resource_to_event_kind_moved_from(from_kind);
                let to_kind = Self::convert_resource_to_event_kind_moved_to(to_kind);
                let from_event = Event::with_time(from_kind, time, parent).add_path(from);
                let to_event = Event::with_time(to_kind, time, parent).add_path(to);
                vec![from_event, to_event]
            }
        }
    }

    fn convert_resource_to_event_kind_renamed_from(kind: resources::ResourceEvent) -> EventKind {
        match kind {
            resources::ResourceEvent::Config(kind) => match kind {
                resources::Config::ProjectManifest => EventKind::Config(
                    app::Config::ProjectManifest(app::StaticResourceEvent::Removed),
                ),
                resources::Config::UserManifest => {
                    EventKind::Config(app::Config::UserManifest(app::StaticResourceEvent::Removed))
                }
                resources::Config::LocalConfig => {
                    EventKind::Config(app::Config::LocalConfig(app::StaticResourceEvent::Removed))
                }
            },

            resources::ResourceEvent::Project { kind, .. } => match kind {
                resources::Project::Properties => {
                    app::Project::Properties(app::StaticResourceEvent::Removed).into()
                }

                resources::Project::Settings => {
                    app::Project::Settings(app::StaticResourceEvent::Removed).into()
                }

                resources::Project::Analyses => {
                    app::Project::Analyses(app::StaticResourceEvent::Removed).into()
                }
            },

            resources::ResourceEvent::Container { kind, .. } => match kind {
                resources::Container::Properties => {
                    app::Container::Properties(app::StaticResourceEvent::Removed).into()
                }

                resources::Container::Settings => {
                    app::Container::Settings(app::StaticResourceEvent::Removed).into()
                }

                resources::Container::Assets => {
                    app::Container::Assets(app::StaticResourceEvent::Removed).into()
                }
            },

            resources::ResourceEvent::Analysis { .. } => {
                app::EventKind::AnalysisFile(app::ResourceEvent::Removed)
            }

            resources::ResourceEvent::Asset { .. } => {
                panic!("asset file renaming should not affect resource kind");
            }
        }
    }

    fn convert_resource_to_event_kind_renamed_to(kind: resources::ResourceEvent) -> EventKind {
        match kind {
            resources::ResourceEvent::Config(kind) => match kind {
                resources::Config::ProjectManifest => EventKind::Config(
                    app::Config::ProjectManifest(app::StaticResourceEvent::Created),
                ),

                resources::Config::UserManifest => {
                    EventKind::Config(app::Config::UserManifest(app::StaticResourceEvent::Created))
                }

                resources::Config::LocalConfig => {
                    EventKind::Config(app::Config::LocalConfig(app::StaticResourceEvent::Created))
                }
            },

            resources::ResourceEvent::Project { kind, .. } => match kind {
                resources::Project::Properties => {
                    app::Project::Properties(app::StaticResourceEvent::Created).into()
                }

                resources::Project::Settings => {
                    app::Project::Settings(app::StaticResourceEvent::Created).into()
                }

                resources::Project::Analyses => {
                    app::Project::Analyses(app::StaticResourceEvent::Created).into()
                }
            },

            resources::ResourceEvent::Container { kind, .. } => match kind {
                resources::Container::Properties => {
                    app::Container::Properties(app::StaticResourceEvent::Created).into()
                }

                resources::Container::Settings => {
                    app::Container::Settings(app::StaticResourceEvent::Created).into()
                }

                resources::Container::Assets => {
                    app::Container::Assets(app::StaticResourceEvent::Created).into()
                }
            },

            resources::ResourceEvent::Analysis { .. } => {
                app::EventKind::AnalysisFile(app::ResourceEvent::Created)
            }

            resources::ResourceEvent::Asset { .. } => {
                panic!("asset file renaming should not affect resource kind");
            }
        }
    }

    fn convert_resource_to_event_kind_renamed(
        from_kind: resources::ResourceEvent,
        to_kind: resources::ResourceEvent,
        from: PathBuf,
        to: PathBuf,
        time: Instant,
        parent: Uuid,
    ) -> StdResult<Vec<Event>, error::processing::Error> {
        match (from_kind, to_kind) {
            (
                resources::ResourceEvent::Asset {
                    project: from_project,
                },
                resources::ResourceEvent::Asset {
                    project: to_project,
                },
            ) => {
                if from_project != to_project {
                    return Err(error::processing::Error::InvalidState(
                        "asset rename should not change project".to_string(),
                    ));
                }

                Ok(vec![Event::with_time(
                    EventKind::AssetFile(app::ResourceEvent::Renamed),
                    time,
                    parent,
                )
                .add_path(from)
                .add_path(to)])
            }

            (
                resources::ResourceEvent::Analysis {
                    project: from_project,
                },
                resources::ResourceEvent::Analysis {
                    project: to_project,
                },
            ) => {
                if from_project != to_project {
                    return Err(error::processing::Error::InvalidState(
                        "analysis rename should not change project".to_string(),
                    ));
                }

                Ok(vec![Event::with_time(
                    EventKind::AnalysisFile(app::ResourceEvent::Renamed),
                    time,
                    parent,
                )
                .add_path(from)
                .add_path(to)])
            }

            (from_kind, to_kind) => {
                let from_kind = Self::convert_resource_to_event_kind_moved_from(from_kind);
                let to_kind = Self::convert_resource_to_event_kind_moved_to(to_kind);
                let from_event = Event::with_time(from_kind, time, parent).add_path(from);
                let to_event = Event::with_time(to_kind, time, parent).add_path(to);
                Ok(vec![from_event, to_event])
            }
        }
    }

    fn convert_resource_to_event_kind_data_modified(kind: resources::ResourceEvent) -> EventKind {
        match kind {
            resources::ResourceEvent::Config(kind) => match kind {
                resources::Config::ProjectManifest => app::Config::ProjectManifest(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Data),
                )
                .into(),

                resources::Config::UserManifest => app::Config::UserManifest(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Data),
                )
                .into(),

                resources::Config::LocalConfig => app::Config::LocalConfig(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Data),
                )
                .into(),
            },

            resources::ResourceEvent::Project { kind, .. } => match kind {
                resources::Project::Properties => app::Project::Properties(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Data),
                )
                .into(),

                resources::Project::Settings => app::Project::Settings(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Data),
                )
                .into(),

                resources::Project::Analyses => app::Project::Analyses(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Data),
                )
                .into(),
            },

            resources::ResourceEvent::Container { kind, .. } => match kind {
                resources::Container::Properties => app::Container::Properties(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Data),
                )
                .into(),

                resources::Container::Settings => app::Container::Settings(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Data),
                )
                .into(),

                resources::Container::Assets => app::Container::Assets(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Data),
                )
                .into(),
            },

            resources::ResourceEvent::Analysis { .. } => {
                app::EventKind::AnalysisFile(app::ResourceEvent::Modified(app::ModifiedKind::Data))
            }

            resources::ResourceEvent::Asset { .. } => {
                app::EventKind::AssetFile(app::ResourceEvent::Modified(app::ModifiedKind::Data))
            }
        }
    }

    fn convert_resource_to_event_kind_other(kind: resources::ResourceEvent) -> EventKind {
        match kind {
            resources::ResourceEvent::Config(kind) => match kind {
                resources::Config::ProjectManifest => app::Config::ProjectManifest(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),

                resources::Config::UserManifest => app::Config::UserManifest(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),

                resources::Config::LocalConfig => app::Config::LocalConfig(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),
            },

            resources::ResourceEvent::Project { kind, .. } => match kind {
                resources::Project::Properties => app::Project::Properties(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),

                resources::Project::Settings => app::Project::Settings(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),

                resources::Project::Analyses => app::Project::Analyses(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),
            },

            resources::ResourceEvent::Container { kind, .. } => match kind {
                resources::Container::Properties => app::Container::Properties(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),

                resources::Container::Settings => app::Container::Settings(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),

                resources::Container::Assets => app::Container::Assets(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),
            },

            resources::ResourceEvent::Analysis { .. } => {
                app::EventKind::AnalysisFile(app::ResourceEvent::Modified(app::ModifiedKind::Other))
            }

            resources::ResourceEvent::Asset { .. } => {
                app::EventKind::AssetFile(app::ResourceEvent::Modified(app::ModifiedKind::Other))
            }
        }
    }
}

impl FsWatcher {
    fn convert_dir_to_event_kind_created(kind: &resources::DirKind) -> EventKind {
        match kind {
            resources::DirKind::AppConfig => app::Config::Created.into(),
            resources::DirKind::Project { kind, .. } => match kind {
                resources::ProjectDir::Root => app::Project::Modified.into(),
                resources::ProjectDir::Config => {
                    app::Project::ConfigDir(app::StaticResourceEvent::Created).into()
                }
                resources::ProjectDir::Analysis => {
                    app::Project::AnalysisDir(app::ResourceEvent::Created).into()
                }
                resources::ProjectDir::Data => {
                    app::Project::DataDir(app::ResourceEvent::Created).into()
                }
            },
            resources::DirKind::ContainerLike { .. } => unreachable!("should be handled elsewhere"),
            resources::DirKind::Container { .. } => EventKind::Graph(app::Graph::Created),
            resources::DirKind::ContainerConfig { .. } => {
                EventKind::Container(app::Container::ConfigDir(app::StaticResourceEvent::Created))
            }
            resources::DirKind::None { .. } => EventKind::Folder(app::ResourceEvent::Created),
        }
    }

    fn convert_dir_to_event_kind_removed(kind: &resources::DirKind) -> EventKind {
        match kind {
            resources::DirKind::AppConfig => app::Config::Removed.into(),
            resources::DirKind::Project { kind, .. } => match kind {
                resources::ProjectDir::Root => app::Project::FolderRemoved.into(),
                resources::ProjectDir::Config => {
                    app::Project::ConfigDir(app::StaticResourceEvent::Removed).into()
                }

                resources::ProjectDir::Analysis => {
                    app::Project::AnalysisDir(app::ResourceEvent::Removed).into()
                }

                resources::ProjectDir::Data => {
                    app::Project::DataDir(app::ResourceEvent::Removed).into()
                }
            },
            resources::DirKind::ContainerLike { .. } => {
                EventKind::Folder(app::ResourceEvent::Removed)
            }
            resources::DirKind::Container { .. } => EventKind::Graph(app::Graph::Removed),
            resources::DirKind::ContainerConfig { .. } => {
                app::Container::ConfigDir(app::StaticResourceEvent::Removed).into()
            }
            resources::DirKind::None { .. } => EventKind::Folder(app::ResourceEvent::Removed),
        }
    }

    /// Convert a directory event as if it moved from a resource to not.
    fn convert_dir_to_event_kind_moved_from(kind: &resources::DirKind) -> EventKind {
        match kind {
            resources::DirKind::AppConfig => app::Config::Removed.into(),
            resources::DirKind::Project { kind, .. } => match kind {
                resources::ProjectDir::Root => app::Project::Modified.into(),
                resources::ProjectDir::Config => {
                    app::Project::ConfigDir(app::StaticResourceEvent::Removed).into()
                }
                resources::ProjectDir::Analysis => {
                    app::Project::AnalysisDir(app::ResourceEvent::Moved).into()
                }
                resources::ProjectDir::Data => {
                    app::Project::DataDir(app::ResourceEvent::Moved).into()
                }
            },
            resources::DirKind::ContainerLike { .. } => unreachable!("should be handled elsewhere"),
            resources::DirKind::Container { .. } => EventKind::Graph(app::Graph::Removed),
            resources::DirKind::ContainerConfig { .. } => {
                EventKind::Folder(app::ResourceEvent::Removed)
            }
            resources::DirKind::None { .. } => unreachable!("should be handled elsewhere"),
        }
    }

    /// Convert a directory event as if the directory went from not being a resource to being one.
    fn convert_dir_to_event_kind_moved_to(kind: &resources::DirKind) -> EventKind {
        match kind {
            resources::DirKind::AppConfig => app::Config::Modified(app::ModifiedKind::Other).into(),
            resources::DirKind::Project { kind, .. } => match kind {
                resources::ProjectDir::Root => app::Project::Modified.into(),
                resources::ProjectDir::Config => app::Project::ConfigDir(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),
                resources::ProjectDir::Analysis => app::Project::AnalysisDir(
                    app::ResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),
                resources::ProjectDir::Data => {
                    app::Project::DataDir(app::ResourceEvent::Modified(app::ModifiedKind::Other))
                        .into()
                }
            },
            resources::DirKind::ContainerLike { .. } => unreachable!("should be handled elsewhere"),
            resources::DirKind::Container { .. } => {
                app::Graph::Modified(app::ModifiedKind::Other).into()
            }
            resources::DirKind::ContainerConfig { .. } => {
                EventKind::Folder(app::ResourceEvent::Modified(app::ModifiedKind::Other))
            }
            resources::DirKind::None { .. } => unreachable!("should be handled elsewhere"),
        }
    }

    /// Convert event as if it was moved inside a project to outside a project.
    fn convert_dir_to_event_kind_moved_from_project(kind: &resources::DirKind) -> EventKind {
        match kind {
            resources::DirKind::AppConfig => app::Config::Removed.into(),
            resources::DirKind::Project { kind, .. } => match kind {
                resources::ProjectDir::Root => app::Project::FolderRemoved.into(), // TODO: Maybe unreachable?
                resources::ProjectDir::Config => {
                    app::Project::ConfigDir(app::StaticResourceEvent::Removed).into()
                }
                resources::ProjectDir::Analysis => {
                    app::Project::AnalysisDir(app::ResourceEvent::Removed).into()
                }
                resources::ProjectDir::Data => {
                    app::Project::DataDir(app::ResourceEvent::Removed).into()
                }
            },
            resources::DirKind::ContainerLike { .. } => unreachable!("should be handled elsewhere"),
            resources::DirKind::Container { .. } => EventKind::Graph(app::Graph::Removed),
            resources::DirKind::ContainerConfig { .. } => {
                EventKind::Folder(app::ResourceEvent::Removed)
            }
            resources::DirKind::None { .. } => EventKind::Folder(app::ResourceEvent::Removed),
        }
    }

    /// Convert event as if it was moved from outside a project into a project.
    fn convert_dir_to_event_kind_moved_to_project(kind: &resources::DirKind) -> EventKind {
        match kind {
            resources::DirKind::AppConfig => app::Config::Modified(app::ModifiedKind::Other).into(),
            resources::DirKind::Project { kind, .. } => match kind {
                resources::ProjectDir::Root => app::Project::Modified.into(),
                resources::ProjectDir::Config => app::Project::ConfigDir(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),
                resources::ProjectDir::Analysis => app::Project::AnalysisDir(
                    app::ResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),
                resources::ProjectDir::Data => {
                    app::Project::DataDir(app::ResourceEvent::Modified(app::ModifiedKind::Other))
                        .into()
                }
            },
            resources::DirKind::ContainerLike { .. } => {
                app::EventKind::Folder(app::ResourceEvent::Created)
            }
            resources::DirKind::Container { .. } => app::Graph::Created.into(),
            resources::DirKind::ContainerConfig { .. } => {
                EventKind::Folder(app::ResourceEvent::Modified(app::ModifiedKind::Other))
            }
            resources::DirKind::None { .. } => EventKind::Folder(app::ResourceEvent::Created),
        }
    }

    fn convert_dir_to_event_kind_moved(
        from_kind: resources::DirKind,
        to_kind: resources::DirKind,
        from: PathBuf,
        to: PathBuf,
        time: Instant,
        parent: Uuid,
    ) -> Vec<Event> {
        assert!(!matches!(from_kind, resources::DirKind::None { .. }));
        assert!(!matches!(to_kind, resources::DirKind::None { .. }));
        assert!(
            from.parent() != to.parent(),
            "move event should have different parents"
        );

        match (from_kind, to_kind) {
            (
                resources::DirKind::Container {
                    project: from_project,
                },
                resources::DirKind::Container {
                    project: to_project,
                },
            ) => {
                if from_project == to_project {
                    vec![
                        app::Event::with_time(app::Graph::Moved.into(), time, parent)
                            .add_path(from)
                            .add_path(to),
                    ]
                } else {
                    vec![
                        Event::with_time(app::Graph::Removed.into(), time, parent).add_path(from),
                        Event::with_time(app::Graph::Created.into(), time, parent).add_path(to),
                    ]
                }
            }

            (
                resources::DirKind::ContainerLike {
                    project: from_project,
                },
                resources::DirKind::ContainerLike {
                    project: to_project,
                },
            ) => {
                if from_project == to_project {
                    vec![Event::with_time(
                        app::EventKind::Folder(app::ResourceEvent::Moved),
                        time,
                        parent,
                    )
                    .add_path(from)
                    .add_path(to)]
                } else {
                    vec![
                        Event::with_time(
                            EventKind::Folder(app::ResourceEvent::Removed),
                            time,
                            parent,
                        )
                        .add_path(from),
                        Event::with_time(
                            EventKind::Folder(app::ResourceEvent::Created),
                            time,
                            parent,
                        )
                        .add_path(to),
                    ]
                }
            }

            (
                resources::DirKind::ContainerLike {
                    project: from_project,
                },
                resources::DirKind::Container {
                    project: to_project,
                },
            ) => {
                if from_project == to_project {
                    vec![Event::with_time(app::Graph::Moved.into(), time, parent)
                        .add_path(from)
                        .add_path(to)]
                } else {
                    vec![
                        Event::with_time(app::Graph::Removed.into(), time, parent).add_path(from),
                        Event::with_time(app::Graph::Created.into(), time, parent).add_path(to),
                    ]
                }
            }

            (
                resources::DirKind::Project {
                    kind: resources::ProjectDir::Root,
                    project: from_project,
                },
                resources::DirKind::Project {
                    kind: resources::ProjectDir::Root,
                    project: to_project,
                },
            ) => {
                assert!(
                    from_project != to_project,
                    "project exists in two locations: {from:?} -> {to:?}"
                );

                tracing::warn!("UNUSUAL SITUATION: Project moved and replaced");
                vec![Event::with_time(app::Project::Moved.into(), time, parent)
                    .add_path(from)
                    .add_path(to)]
            }

            (from_kind, to_kind) => {
                vec![
                    app::Event::with_time(
                        Self::convert_dir_to_event_kind_moved_from(&from_kind),
                        time,
                        parent,
                    )
                    .add_path(from),
                    app::Event::with_time(
                        Self::convert_dir_to_event_kind_moved_to(&to_kind),
                        time,
                        parent,
                    )
                    .add_path(to),
                ]
            }
        }
    }

    /// Convert directory event as if renaming it caused it to go from being a resource to not.
    fn convert_dir_to_event_kind_renamed_from(
        kind: &resources::DirKind,
    ) -> StdResult<EventKind, error::processing::Error> {
        let kind = match kind {
            resources::DirKind::AppConfig => app::Config::Removed.into(),
            resources::DirKind::Project { kind, .. } => match kind {
                resources::ProjectDir::Root => unreachable!("should be handled elsewhere"),
                resources::ProjectDir::Config => {
                    app::Project::ConfigDir(app::StaticResourceEvent::Removed).into()
                }
                resources::ProjectDir::Analysis => {
                    app::Project::AnalysisDir(app::ResourceEvent::Removed).into()
                }
                resources::ProjectDir::Data => {
                    app::Project::DataDir(app::ResourceEvent::Removed).into()
                }
            },
            resources::DirKind::ContainerLike { .. } => unreachable!("should be handled elsewhere"),
            resources::DirKind::Container { .. } => {
                return Err(error::processing::Error::InvalidState(
                    "renaming container should result in container".into(),
                ));
            }
            resources::DirKind::ContainerConfig { .. } => {
                EventKind::Folder(app::ResourceEvent::Removed)
            }
            resources::DirKind::None { .. } => unreachable!("should be handled elsewhere"),
        };

        Ok(kind)
    }

    /// Convert directory event as if renaming it caused it to go from not being a resource to
    /// being one.
    fn convert_dir_to_event_kind_renamed_to(kind: &resources::DirKind) -> EventKind {
        match kind {
            resources::DirKind::AppConfig => app::Config::Modified(app::ModifiedKind::Other).into(),
            resources::DirKind::Project { kind, .. } => match kind {
                resources::ProjectDir::Root => app::Project::Modified.into(),
                resources::ProjectDir::Config => app::Project::ConfigDir(
                    app::StaticResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),
                resources::ProjectDir::Analysis => app::Project::AnalysisDir(
                    app::ResourceEvent::Modified(app::ModifiedKind::Other),
                )
                .into(),
                resources::ProjectDir::Data => {
                    app::Project::DataDir(app::ResourceEvent::Modified(app::ModifiedKind::Other))
                        .into()
                }
            },
            resources::DirKind::ContainerLike { .. } => {
                EventKind::Folder(app::ResourceEvent::Renamed)
            }
            resources::DirKind::Container { .. } => {
                panic!("renaming should not result in container");
            }
            resources::DirKind::ContainerConfig { .. } => {
                EventKind::Folder(app::ResourceEvent::Modified(app::ModifiedKind::Other))
            }
            resources::DirKind::None { .. } => unreachable!("should be handled elsewhere"),
        }
    }

    fn convert_dir_to_event_kind_renamed(
        from_kind: resources::DirKind,
        to_kind: resources::DirKind,
        from: PathBuf,
        to: PathBuf,
        time: Instant,
        parent: Uuid,
    ) -> StdResult<Vec<Event>, error::processing::Error> {
        assert!(
            from.parent() == to.parent(),
            "renamed paths should have same parent"
        );
        match (from_kind, to_kind) {
            (
                resources::DirKind::Container {
                    project: from_project,
                },
                resources::DirKind::Container {
                    project: to_project,
                },
            )
            | (
                resources::DirKind::ContainerLike {
                    project: from_project,
                },
                resources::DirKind::Container {
                    project: to_project,
                },
            ) => {
                assert!(
                    from_project == to_project,
                    "renaming container should not change project"
                );

                Ok(vec![app::Event::with_time(
                    app::Container::Renamed.into(),
                    time,
                    parent,
                )
                .add_path(from)
                .add_path(to)])
            }

            (
                resources::DirKind::ContainerLike {
                    project: from_project,
                },
                resources::DirKind::ContainerConfig {
                    project: to_project,
                },
            ) => {
                assert!(
                    from_project == to_project,
                    "renaming container should not change project"
                );

                Ok(vec![
                    app::Event::with_time(
                        app::EventKind::Folder(app::ResourceEvent::Removed).into(),
                        time,
                        parent,
                    )
                    .add_path(from),
                    app::Event::with_time(
                        app::Container::ConfigDir(app::StaticResourceEvent::Created).into(),
                        time,
                        parent,
                    )
                    .add_path(to),
                ])
            }

            (
                resources::DirKind::ContainerConfig {
                    project: from_project,
                },
                resources::DirKind::ContainerLike {
                    project: to_project,
                },
            ) => {
                assert!(
                    from_project == to_project,
                    "renaming container should not change project."
                );

                Ok(vec![
                    app::Event::with_time(
                        app::Container::ConfigDir(app::StaticResourceEvent::Removed).into(),
                        time,
                        parent,
                    )
                    .add_path(from),
                    app::Event::with_time(
                        app::EventKind::Folder(app::ResourceEvent::Created).into(),
                        time,
                        parent,
                    )
                    .add_path(to),
                ])
            }

            (
                resources::DirKind::ContainerConfig {
                    project: from_project,
                },
                resources::DirKind::Container {
                    project: to_project,
                },
            ) => {
                assert!(
                    from_project == to_project,
                    "renaming container should not change project."
                );

                Ok(vec![
                    app::Event::with_time(
                        app::Container::ConfigDir(app::StaticResourceEvent::Removed).into(),
                        time,
                        parent,
                    )
                    .add_path(from),
                    app::Event::with_time(app::Graph::Created.into(), time, parent).add_path(to),
                ])
            }

            (
                resources::DirKind::Project {
                    kind: resources::ProjectDir::Root,
                    ..
                },
                resources::DirKind::Project {
                    kind: resources::ProjectDir::Root,
                    ..
                },
            ) => Ok(vec![
                app::Event::with_time(app::Project::Moved.into(), time, parent),
                app::Event::with_time(app::Project::Modified.into(), time, parent),
            ]),

            (
                resources::DirKind::Project {
                    kind: resources::ProjectDir::Root,
                    ..
                },
                resources::DirKind::Project { kind: to_kind, .. },
            ) => {
                return Err(error::processing::Error::InvalidState(format!(
                    "renaming project resulted in {to_kind:?}. {from:?} -> {to:?}"
                )));
            }
            (
                resources::DirKind::Project {
                    kind: from_kind, ..
                },
                resources::DirKind::Project {
                    kind: resources::ProjectDir::Root,
                    ..
                },
            ) => {
                return Err(error::processing::Error::InvalidState(format!(
                    "renaming {from_kind:?} resulted in project. {from:?} -> {to:?}"
                )));
            }

            (from_kind, to_kind) => {
                let from_kind = Self::convert_dir_to_event_kind_renamed_from(&from_kind)?;
                let to_kind = Self::convert_dir_to_event_kind_renamed_to(&to_kind);
                Ok(vec![
                    app::Event::with_time(from_kind, time, parent).add_path(from),
                    app::Event::with_time(to_kind, time, parent).add_path(to),
                ])
            }
        }
    }
}

mod resources {
    use super::super::config;
    pub use error::{Error, ErrorKind};
    use std::path::{Component, Path, PathBuf};
    use syre_core::{project::ScriptLang, types::ResourceId};
    use syre_local::{
        common,
        project::{project, resources::Project as LocalProject},
    };

    /// Files of resources represented
    #[derive(Debug, derive_more::From)]
    pub(crate) enum ResourceEvent {
        #[from]
        Config(Config),
        Project {
            project: PathBuf,
            kind: Project,
        },

        Container {
            project: ResourceId,
            kind: Container,
        },

        Asset {
            project: ResourceId,
        },

        Analysis {
            project: ResourceId,
        },
    }

    #[derive(Debug)]
    pub(crate) enum Config {
        ProjectManifest,
        UserManifest,
        LocalConfig,
    }

    #[derive(Debug)]
    pub(crate) enum Project {
        Properties,
        Settings,
        Analyses,
    }

    #[derive(Debug)]
    pub(crate) enum Container {
        Properties,
        Settings,
        Assets,
    }

    #[derive(Debug)]
    pub(crate) enum DirKind {
        AppConfig,
        Project {
            project: ResourceId,
            kind: ProjectDir,
        },

        /// Folder is confirmed to be a container.
        Container {
            project: ResourceId,
        },

        /// Folder could potentially be a container.
        /// i.e. It is in the data folder of a project,
        /// and not within an app (.syre) folder.
        ContainerLike {
            project: ResourceId,
        },

        ContainerConfig {
            project: ResourceId,
        },

        None {
            project: ResourceId,
        },
    }

    #[derive(Debug)]
    pub(crate) enum ProjectDir {
        /// A project's base folder.
        Root,

        /// A project's config (.syre) folder.
        Config,

        /// A project's data root folder.
        Data,

        /// A project's analysis root folder.
        Analysis,
    }

    /// Represents potential of a path to be a config resource based on its location.
    #[derive(Debug)]
    pub(crate) enum ConfigLocationKind {
        /// Not a config resource.
        /// Path either does not contain any app folders (.syre) in it,
        /// or is nested in an app folder such that it could not be a resource.
        ///
        /// e.g. `/my/project/path/data`, `/my/project/path/.syre/nested/folder`
        Not,

        /// A potential config directory.
        /// Path was not nested in any app folders (.syre), and the final
        /// path component is an app folder.
        ///
        /// e.g. `/my/project/path/data/child/.syre`
        Dir,

        /// A potential config resource.
        /// Path is a child of a potential config directory.
        ///
        /// e.g. `/my/project/path/.syre/file.json`
        Child,

        /// Path is a descnedant of an app folder (.syre) but is not a child.
        ///
        /// e.g. `/my/project/path/.syre/nested/file.json`
        Nested,
    }

    /// Gets the kind of resource the path represents.
    ///
    /// # Errors
    /// + If the path does not belong to a valid app location (config or project).
    /// + If the path is in a project that is corrupt.
    pub(crate) fn resource_kind(
        path: &PathBuf,
        app_config: &config::Config,
    ) -> Result<Option<ResourceEvent>, Error> {
        if path == app_config.project_manifest() {
            return Ok(Some(Config::ProjectManifest.into()));
        }

        if path == app_config.user_manifest() {
            return Ok(Some(Config::UserManifest.into()));
        }

        if path == app_config.local_config() {
            return Ok(Some(Config::LocalConfig.into()));
        }

        let project = match project_by_resource_path(&path) {
            Ok(project) => project,
            Err(err) => match err.kind() {
                ErrorKind::NotInProject | ErrorKind::LoadProjectManifest(_) => return Err(err),
                ErrorKind::LoadProject(_) => {
                    let kind = if *path == common::project_file_of(err.path()) {
                        Project::Properties
                    } else if *path == common::project_settings_file_of(err.path()) {
                        Project::Settings
                    } else if *path == common::analyses_file_of(err.path()) {
                        Project::Analyses
                    } else {
                        return Err(err);
                    };

                    return Ok(Some(ResourceEvent::Project {
                        project: err.path().clone(),
                        kind,
                    }));
                }
            },
        };

        if path.starts_with(common::app_dir_of(project.base_path())) {
            let kind =
                handle_file_project(path, project.base_path()).map(|kind| ResourceEvent::Project {
                    project: project.base_path().to_path_buf(),
                    kind,
                });

            return Ok(kind);
        }

        if let Some(analysis_root) = project.analysis_root_path().as_ref() {
            if path.starts_with(analysis_root) {
                return Ok(is_analysis(path).then_some(ResourceEvent::Analysis {
                    project: project.rid().clone(),
                }));
            }
        }

        if path.starts_with(project.data_root_path()) {
            let kind = handle_file_data(path, &project);
            return Ok(kind);
        }

        Ok(None)
    }

    pub(crate) fn dir_kind(path: &PathBuf) -> Result<DirKind, Error> {
        if let Ok(config_dir) = syre_local::system::common::config_dir_path() {
            if *path == config_dir {
                return Ok(DirKind::AppConfig);
            }
        }

        let project = project_by_resource_path(&path)?;
        if *path == project.base_path() {
            return Ok(DirKind::Project {
                project: project.rid().clone(),
                kind: ProjectDir::Root,
            });
        }

        if *path == project.data_root_path() {
            return Ok(DirKind::Project {
                project: project.rid().clone(),
                kind: ProjectDir::Data,
            });
        }

        if let Some(analysis_dir) = project.analysis_root_path() {
            if *path == analysis_dir {
                return Ok(DirKind::Project {
                    project: project.rid().clone(),
                    kind: ProjectDir::Analysis,
                });
            }
        }

        if *path == common::app_dir_of(project.base_path()) {
            return Ok(DirKind::Project {
                project: project.rid().clone(),
                kind: ProjectDir::Config,
            });
        }

        if path.starts_with(project.data_root_path()) {
            let kind = handle_folder_data(path, &project);
            return Ok(kind);
        }

        Ok(DirKind::None {
            project: project.rid().clone(),
        })
    }

    /// Returns the potential type of config directory the path represents.
    ///
    /// # Errors
    /// + If a path segment was required to determine the resource kind,
    /// but it could not be obtained.
    pub(crate) fn config_resource_location(
        path: impl AsRef<Path>,
    ) -> Result<ConfigLocationKind, ()> {
        let app_dir = common::app_dir().as_os_str();
        let path = path.as_ref();
        let kind = match path
            .components()
            .filter(
                |component| matches!(component, Component::Normal(segment) if *segment == app_dir),
            )
            .count()
        {
            0 => ConfigLocationKind::Not,
            1 => {
                let Some(file_name) = path.file_name() else {
                    return Err(());
                };

                if file_name == app_dir {
                    ConfigLocationKind::Dir
                } else {
                    let Some(parent) = path.parent() else {
                        return Err(());
                    };

                    let Some(parent) = parent.file_name() else {
                        return Err(());
                    };

                    if parent == app_dir {
                        ConfigLocationKind::Child
                    } else {
                        ConfigLocationKind::Nested
                    }
                }
            }

            _ => ConfigLocationKind::Nested,
        };

        Ok(kind)
    }

    /// Get a `Project` by a path within it.
    ///
    /// # Errors
    /// + The path is not in a project.
    /// + The path is in a project that can not be loaded.
    ///     The associated path is the base path of the project.
    fn project_by_resource_path(path: impl Into<PathBuf>) -> Result<LocalProject, Error> {
        let path = path.into();
        let Some(project_path) = project::project_root_path(&path) else {
            return Err(Error::new(path, ErrorKind::NotInProject));
        };

        LocalProject::load_from(&project_path)
            .map_err(|err| Error::new(project_path, ErrorKind::LoadProject(err).into()))
    }

    fn handle_file_project(path: &PathBuf, project: &Path) -> Option<Project> {
        if *path == common::project_file_of(project) {
            Some(Project::Properties)
        } else if *path == common::project_settings_file_of(project) {
            Some(Project::Settings)
        } else if *path == common::analyses_file_of(project) {
            Some(Project::Analyses)
        } else {
            None
        }
    }

    fn is_analysis(path: &PathBuf) -> bool {
        let Some(ext) = path.extension() else {
            return true;
        };

        let ext = ext.to_ascii_lowercase();
        let ext = ext.to_str().unwrap();
        if ScriptLang::supported_extensions().contains(&ext) {
            true
        } else {
            false
        }
    }

    fn handle_file_data(path: &PathBuf, project: &LocalProject) -> Option<ResourceEvent> {
        let Ok(rel_path) = path.strip_prefix(project.base_path()) else {
            return None;
        };

        let Ok(config_location) = config_resource_location(rel_path) else {
            return None;
        };

        match config_location {
            ConfigLocationKind::Not => Some(ResourceEvent::Asset {
                project: project.rid().clone(),
            }),

            ConfigLocationKind::Dir => {
                unreachable!("resource should not be a possible config folder")
            }

            ConfigLocationKind::Nested => None,

            ConfigLocationKind::Child => {
                if path.ends_with(common::container_file()) {
                    Some(ResourceEvent::Container {
                        project: project.rid().clone(),
                        kind: Container::Properties,
                    })
                } else if path.ends_with(common::container_settings_file()) {
                    Some(ResourceEvent::Container {
                        project: project.rid().clone(),
                        kind: Container::Settings,
                    })
                } else if path.ends_with(common::assets_file()) {
                    Some(ResourceEvent::Container {
                        project: project.rid().clone(),
                        kind: Container::Assets,
                    })
                } else {
                    None
                }
            }
        }
    }

    /// Obtain the type of resource a folder is that is within a project's data folder.
    fn handle_folder_data(path: &PathBuf, project: &LocalProject) -> DirKind {
        assert!(
            path.starts_with(project.data_root_path()),
            "data folders must begin with data root path"
        );

        let rel_path = path.strip_prefix(project.base_path()).unwrap();
        let Ok(config_location) = config_resource_location(rel_path) else {
            return DirKind::None {
                project: project.rid().clone(),
            };
        };

        match config_location {
            ConfigLocationKind::Not => {
                if common::container_file_of(path).exists() {
                    DirKind::Container {
                        project: project.rid().clone(),
                    }
                } else {
                    DirKind::ContainerLike {
                        project: project.rid().clone(),
                    }
                }
            }

            ConfigLocationKind::Dir => DirKind::ContainerConfig {
                project: project.rid().clone(),
            },

            ConfigLocationKind::Child => DirKind::None {
                project: project.rid().clone(),
            },

            ConfigLocationKind::Nested => DirKind::None {
                project: project.rid().clone(),
            },
        }
    }

    pub mod error {
        //! event errors meant to be reported with events that caused them.
        use std::path::PathBuf;
        use syre_local::{error::IoSerde, project::resources::project::LoadError as LoadProject};

        #[derive(Debug)]
        pub struct Error {
            path: PathBuf,
            kind: ErrorKind,
        }

        impl Error {
            pub fn new(path: PathBuf, kind: ErrorKind) -> Self {
                Self { path, kind }
            }

            pub fn path(&self) -> &PathBuf {
                &self.path
            }

            pub fn kind(&self) -> &ErrorKind {
                &self.kind
            }
        }

        #[derive(Debug, PartialEq)]
        pub enum ErrorKind {
            /// The path was not in a valid project.
            NotInProject,

            /// The project failed to load.
            LoadProject(LoadProject),

            /// The project manifest failed to load.
            LoadProjectManifest(IoSerde),
        }
    }
}

#[cfg(test)]
#[path = "fs_processor_test.rs"]
mod fs_processor_test;
