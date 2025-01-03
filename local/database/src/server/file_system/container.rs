use crate::{
    common,
    event::{self as update, Update},
    server, state, Database,
};
use std::{assert_matches::assert_matches, io};
use syre_fs_watcher::{event, EventKind};
use syre_local::{error::IoSerde, loader, TryReducible};

impl Database {
    pub(super) fn handle_fs_event_container(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Container(kind) = event.kind() else {
            panic!("invalid event kind");
        };

        match kind {
            event::Container::Renamed => self.handle_fs_event_container_renamed(event),
            event::Container::ConfigDir(_) => self.handle_fs_event_container_config_dir(event),
            event::Container::Properties(_) => self.handle_fs_event_container_properties(event),
            event::Container::Settings(_) => self.handle_fs_event_container_settings(event),
            event::Container::Assets(_) => self.handle_fs_event_container_assets(event),
            event::Container::Flags(_) => self.handle_fs_event_container_flags(event),
        }
    }
}

impl Database {
    fn handle_fs_event_container_renamed(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Container(event::Container::Renamed)
        );

        let [from, to] = &event.paths()[..] else {
            panic!("invalid paths");
        };
        assert_eq!(from.parent(), to.parent());

        let project = self.state.find_resource_project_by_path(from).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        // let state::FolderResource::Present(graph) = project_state.graph() else {
        //     panic!("invalid state");
        // };

        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let container_graph_path =
            common::container_graph_path(project.path().join(&project_properties.data_root), &from)
                .unwrap();
        // let container_state = graph.find(&container_graph_path).unwrap().unwrap();
        // let container_state = container_state.lock().unwrap();
        let to_name = to.file_name().unwrap().to_os_string();
        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        // drop(container_state);
        self.state
            .try_reduce(server::state::Action::Project {
                path: project_path.clone(),
                action: server::state::project::Action::Container {
                    path: container_graph_path.clone(),
                    action: server::state::project::action::Container::SetName(to_name.clone()),
                },
            })
            .unwrap();

        if self.config.handle_fs_resource_changes() {
            if let Err(err) = self.handle_fs_event_container_renamed_changes(&event) {
                tracing::error!(?err);
            }
        }

        vec![Update::project_with_id(
            project_id.clone(),
            project_path.clone(),
            update::Graph::Renamed {
                from: container_graph_path,
                to: to_name,
            }
            .into(),
            event.id().clone(),
        )]
    }

    fn handle_fs_event_container_renamed_changes(
        &mut self,
        event: &syre_fs_watcher::Event,
    ) -> Result<(), IoSerde> {
        use syre_local::loader::container::Loader;

        let [from, to] = &event.paths()[..] else {
            panic!("invalid paths");
        };
        assert_eq!(from.parent(), to.parent());

        let mut properties = Loader::load_from_only_properties(to)?;
        properties.properties.name = to.file_name().unwrap().to_string_lossy().to_string();
        properties.save(to)?;
        Ok(())
    }
}

impl Database {
    fn handle_fs_event_container_config_dir(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Container(event::Container::ConfigDir(kind)) = event.kind() else {
            panic!("invalid event kind");
        };

        match kind {
            event::StaticResourceEvent::Created => {
                self.handle_fs_event_container_config_dir_created(event)
            }
            event::StaticResourceEvent::Removed => {
                self.handle_fs_event_container_config_dir_removed(event)
            }
            event::StaticResourceEvent::Modified(_) => {
                self.handle_fs_event_container_config_dir_modified(event)
            }
        }
    }

    fn handle_fs_event_container_config_dir_created(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Container(event::Container::ConfigDir(
                event::StaticResourceEvent::Created
            ))
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap();
        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::FolderResource::Present(graph) = project_state.graph() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let container_graph_path = common::container_graph_path(
            project.path().join(&project_properties.data_root),
            &base_path,
        )
        .unwrap();

        let container_state = graph.find(&container_graph_path).unwrap().unwrap();
        let container_state = container_state.lock().unwrap();
        if cfg!(target_os = "windows") {
            if !matches!(
                container_state.properties(),
                state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
            ) {
                tracing::warn!("container properties already exists");
            }
            if !matches!(
                container_state.settings(),
                state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
            ) {
                tracing::warn!("container settings already exists");
            }
            if !matches!(
                container_state.assets(),
                state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
            ) {
                tracing::warn!("container assets already exists");
            }
        } else {
            assert_matches!(
                container_state.properties(),
                state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
            );
            assert_matches!(
                container_state.settings(),
                state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
            );
            assert_matches!(
                container_state.assets(),
                state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
            );
        }
        drop(container_state);

        let loader::container::State {
            properties,
            settings,
            assets,
        } = loader::container::Loader::load_resources(base_path);

        let mut updates = vec![];
        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        if !matches!(properties, Err(IoSerde::Io(io::ErrorKind::NotFound))) {
            self.state
                .try_reduce(server::state::Action::Project {
                    path: project_path.clone(),
                    action: server::state::project::Action::Container {
                        path: container_graph_path.clone(),
                        action: server::state::project::action::Container::SetProperties(
                            properties.clone(),
                        ),
                    },
                })
                .unwrap();

            updates.push(Update::project_with_id(
                project_id.clone(),
                project_path.clone(),
                update::Project::Container {
                    path: container_graph_path.clone(),
                    update: update::Container::Properties(update::DataResource::Created(
                        properties,
                    )),
                },
                event.id().clone(),
            ));
        }

        if !matches!(settings, Err(IoSerde::Io(io::ErrorKind::NotFound))) {
            self.state
                .try_reduce(server::state::Action::Project {
                    path: project_path.clone(),
                    action: server::state::project::Action::Container {
                        path: container_graph_path.clone(),
                        action: server::state::project::action::Container::SetSettings(
                            settings.clone(),
                        ),
                    },
                })
                .unwrap();

            updates.push(Update::project_with_id(
                project_id.clone(),
                project_path.clone(),
                update::Project::Container {
                    path: container_graph_path.clone(),
                    update: update::Container::Settings(update::DataResource::Created(settings)),
                },
                event.id().clone(),
            ));
        }

        match assets {
            Ok(assets) => {
                let assets = assets::from_assets(base_path, assets);
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: project_path.clone(),
                        action: server::state::project::Action::Container {
                            path: container_graph_path.clone(),
                            action: server::state::project::action::Container::SetAssets(
                                state::DataResource::Ok(assets.clone()),
                            ),
                        },
                    })
                    .unwrap();

                updates.push(Update::project_with_id(
                    project_id.clone(),
                    project_path.clone(),
                    update::Project::Container {
                        path: container_graph_path.clone(),
                        update: update::Container::Assets(update::DataResource::Created(
                            state::DataResource::Ok(assets),
                        )),
                    },
                    event.id().clone(),
                ));
            }
            Err(IoSerde::Io(io::ErrorKind::NotFound)) => {}
            Err(err) => {
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: project_path.clone(),
                        action: server::state::project::Action::Container {
                            path: container_graph_path.clone(),
                            action: server::state::project::action::Container::SetAssets(
                                state::DataResource::Err(err.clone()),
                            ),
                        },
                    })
                    .unwrap();

                updates.push(Update::project_with_id(
                    project_id.clone(),
                    path,
                    update::Project::Container {
                        path: container_graph_path.clone(),
                        update: update::Container::Assets(update::DataResource::Created(
                            state::DataResource::Err(err),
                        )),
                    },
                    event.id().clone(),
                ));
            }
        }

        updates
    }

    fn handle_fs_event_container_config_dir_removed(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Container(event::Container::ConfigDir(
                event::StaticResourceEvent::Removed
            ))
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap();
        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::FolderResource::Present(graph) = project_state.graph() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let container_graph_path = base_path
            .strip_prefix(project.path().join(&project_properties.data_root))
            .unwrap();

        let container_graph_path = common::prepend_root_dir(container_graph_path);
        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        assert!(graph.find(&container_graph_path).unwrap().is_some());
        self.state
            .try_reduce(server::state::Action::Project {
                path: project_path.clone(),
                action: server::state::project::Action::Container {
                    path: container_graph_path.clone(),
                    action: server::state::project::action::Container::RemoveConfig,
                },
            })
            .unwrap();

        vec![
            Update::project_with_id(
                project_id.clone(),
                project_path.clone(),
                update::Project::Container {
                    path: container_graph_path.clone(),
                    update: update::Container::Properties(update::DataResource::Removed),
                },
                event.id().clone(),
            ),
            Update::project_with_id(
                project_id.clone(),
                project_path.clone(),
                update::Project::Container {
                    path: container_graph_path.clone(),
                    update: update::Container::Settings(update::DataResource::Removed),
                },
                event.id().clone(),
            ),
            Update::project_with_id(
                project_id,
                project_path,
                update::Project::Container {
                    path: container_graph_path,
                    update: update::Container::Assets(update::DataResource::Removed),
                },
                event.id().clone(),
            ),
        ]
    }

    fn handle_fs_event_container_config_dir_modified(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Container(event::Container::ConfigDir(
            event::StaticResourceEvent::Modified(kind),
        )) = event.kind()
        else {
            panic!("invalid event kind");
        };

        todo!();
    }
}

impl Database {
    fn handle_fs_event_container_properties(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Container(event::Container::Properties(kind)) = event.kind() else {
            panic!("invalid event kind");
        };

        match kind {
            event::StaticResourceEvent::Created => {
                self.handle_fs_event_container_properties_created(event)
            }
            event::StaticResourceEvent::Removed => {
                self.handle_fs_event_container_properties_removed(event)
            }
            event::StaticResourceEvent::Modified(_) => {
                self.handle_fs_event_container_properties_modified(event)
            }
        }
    }

    fn handle_fs_event_container_properties_created(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Container(event::Container::Properties(
                event::StaticResourceEvent::Created
            ))
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::FolderResource::Present(graph) = project_state.graph() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let container_graph_path = base_path
            .strip_prefix(project.path().join(&project_properties.data_root))
            .unwrap();
        let container_graph_path = common::prepend_root_dir(container_graph_path);
        let container_state = graph.find(&container_graph_path).unwrap().unwrap();
        let container_state = container_state.lock().unwrap();

        if cfg!(target_os = "windows") {
            if !matches!(
                container_state.properties(),
                state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
            ) {
                tracing::warn!("container properties already exists");
            }
        } else {
            assert_matches!(
                container_state.properties(),
                state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
            );
        }
        drop(container_state);

        let properties = loader::container::Loader::load_from_only_properties(base_path);
        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        if matches!(properties, Err(IoSerde::Io(io::ErrorKind::NotFound))) {
            vec![]
        } else {
            self.state
                .try_reduce(server::state::Action::Project {
                    path: project_path.clone(),
                    action: server::state::project::Action::Container {
                        path: container_graph_path.clone(),
                        action: server::state::project::action::Container::SetProperties(
                            properties.clone(),
                        ),
                    },
                })
                .unwrap();

            vec![Update::project_with_id(
                project_id,
                project_path,
                update::Project::Container {
                    path: container_graph_path.clone(),
                    update: update::Container::Properties(update::DataResource::Created(
                        properties,
                    )),
                },
                event.id().clone(),
            )]
        }
    }

    fn handle_fs_event_container_properties_removed(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Container(event::Container::Properties(
                event::StaticResourceEvent::Removed
            ))
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::FolderResource::Present(graph) = project_state.graph() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let container_graph_path = base_path
            .strip_prefix(project.path().join(&project_properties.data_root))
            .unwrap();
        let container_graph_path = common::prepend_root_dir(container_graph_path);
        let container_state = graph.find(&container_graph_path).unwrap().unwrap();
        let container_state = container_state.lock().unwrap();
        assert!(!matches!(
            container_state.properties(),
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        ));
        drop(container_state);

        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        self.state
            .try_reduce(server::state::Action::Project {
                path: project_path.clone(),
                action: server::state::project::Action::Container {
                    path: container_graph_path.clone(),
                    action: server::state::project::action::Container::SetProperties(
                        state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound)),
                    ),
                },
            })
            .unwrap();

        vec![Update::project_with_id(
            project_id,
            project_path,
            update::Project::Container {
                path: container_graph_path.clone(),
                update: update::Container::Properties(update::DataResource::Removed),
            },
            event.id().clone(),
        )]
    }

    fn handle_fs_event_container_properties_modified(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Container(event::Container::Properties(
            event::StaticResourceEvent::Modified(kind),
        )) = event.kind()
        else {
            panic!("invalid event kind");
        };

        match kind {
            event::ModifiedKind::Data => {
                self.handle_fs_event_container_properties_modified_data(event)
            }
            event::ModifiedKind::Other => {
                self.handle_fs_event_container_properties_modified_other(event)
            }
        }
    }

    fn handle_fs_event_container_properties_modified_data(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Container(event::Container::Properties(
                event::StaticResourceEvent::Modified(event::ModifiedKind::Data),
            )),
        );

        self.handle_container_properties_modified(event)
    }

    fn handle_fs_event_container_properties_modified_other(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Container(event::Container::Properties(
                event::StaticResourceEvent::Modified(event::ModifiedKind::Other),
            )),
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        #[cfg(target_os = "windows")]
        {
            self.handle_container_properties_modified(event)
        }

        #[cfg(target_os = "macos")]
        {
            todo!();
        }

        #[cfg(target_os = "linux")]
        {
            todo!();
        }
    }

    fn handle_container_properties_modified(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Container(event::Container::Properties(
            event::StaticResourceEvent::Modified(kind),
        )) = event.kind()
        else {
            panic!("invalid event kind");
        };

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::FolderResource::Present(graph) = project_state.graph() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let container_graph_path = base_path
            .strip_prefix(project.path().join(&project_properties.data_root))
            .unwrap();
        let container_graph_path = common::prepend_root_dir(container_graph_path);
        let container_state = graph.find(&container_graph_path).unwrap().unwrap();
        let container_state = container_state.lock().unwrap();
        assert!(!matches!(
            container_state.properties(),
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        ));

        let properties = loader::container::Loader::load_from_only_properties(base_path);
        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        let update = match (&container_state.properties, properties.clone()) {
            (Ok(state), Ok(properties)) => {
                if properties == *state {
                    return vec![];
                }

                Update::project_with_id(
                    project_id,
                    project_path.clone(),
                    update::Project::Container {
                        path: container_graph_path.clone(),
                        update: update::Container::Properties(update::DataResource::Modified(
                            properties,
                        )),
                    },
                    event.id().clone(),
                )
            }
            (Err(_state), Ok(properties)) => Update::project_with_id(
                project_id,
                project_path.clone(),
                update::Project::Container {
                    path: container_graph_path.clone(),
                    update: update::Container::Properties(update::DataResource::Repaired(
                        properties,
                    )),
                },
                event.id().clone(),
            ),
            (_, Err(IoSerde::Io(io::ErrorKind::NotFound))) => todo!(),
            (Ok(_), Err(err)) => Update::project_with_id(
                project_id,
                project_path.clone(),
                update::Project::Container {
                    path: container_graph_path.clone(),
                    update: update::Container::Properties(update::DataResource::Corrupted(err)),
                },
                event.id().clone(),
            ),
            (Err(state), Err(err)) => {
                if err == *state {
                    return vec![];
                }

                Update::project_with_id(
                    project_id,
                    project_path.clone(),
                    update::Project::Container {
                        path: container_graph_path.clone(),
                        update: update::Container::Properties(update::DataResource::Corrupted(err)),
                    },
                    event.id().clone(),
                )
            }
        };

        drop(container_state);
        self.state
            .try_reduce(server::state::Action::Project {
                path: project_path,
                action: server::state::project::Action::Container {
                    path: container_graph_path,
                    action: server::state::project::action::Container::SetProperties(properties),
                },
            })
            .unwrap();
        vec![update]
    }
}

impl Database {
    fn handle_fs_event_container_settings(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        let EventKind::Container(event::Container::Settings(kind)) = event.kind() else {
            panic!("invalid event kind");
        };

        match kind {
            event::StaticResourceEvent::Created => {
                self.handle_fs_event_container_settings_created(event)
            }
            event::StaticResourceEvent::Removed => {
                self.handle_fs_event_container_settings_removed(event)
            }
            event::StaticResourceEvent::Modified(_) => {
                self.handle_fs_event_container_settings_modified(event)
            }
        }
    }

    fn handle_fs_event_container_settings_created(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Container(event::Container::Settings(
                event::StaticResourceEvent::Created
            ))
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::FolderResource::Present(graph) = project_state.graph() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let container_graph_path = base_path
            .strip_prefix(project.path().join(&project_properties.data_root))
            .unwrap();
        let container_graph_path = common::prepend_root_dir(container_graph_path);
        let container_state = graph.find(&container_graph_path).unwrap().unwrap();
        let container_state = container_state.lock().unwrap();
        if cfg!(target_os = "windows") {
            if !matches!(
                container_state.settings(),
                state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
            ) {
                tracing::warn!("container settings already exists");
            }
        } else {
            assert_matches!(
                container_state.settings(),
                state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
            );
        }
        drop(container_state);

        let settings = loader::container::Loader::load_from_only_settings(base_path);
        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        if matches!(settings, Err(IoSerde::Io(io::ErrorKind::NotFound))) {
            vec![]
        } else {
            self.state
                .try_reduce(server::state::Action::Project {
                    path: project_path.clone(),
                    action: server::state::project::Action::Container {
                        path: container_graph_path.clone(),
                        action: server::state::project::action::Container::SetSettings(
                            settings.clone(),
                        ),
                    },
                })
                .unwrap();

            vec![Update::project_with_id(
                project_id,
                project_path,
                update::Project::Container {
                    path: container_graph_path.clone(),
                    update: update::Container::Settings(update::DataResource::Created(settings)),
                },
                event.id().clone(),
            )]
        }
    }

    fn handle_fs_event_container_settings_removed(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Container(event::Container::Settings(
                event::StaticResourceEvent::Removed
            ))
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::FolderResource::Present(graph) = project_state.graph() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let container_graph_path = base_path
            .strip_prefix(project.path().join(&project_properties.data_root))
            .unwrap();
        let container_graph_path = common::prepend_root_dir(container_graph_path);
        let container_state = graph.find(&container_graph_path).unwrap().unwrap();
        let container_state = container_state.lock().unwrap();
        assert!(!matches!(
            container_state.settings(),
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        ));
        drop(container_state);

        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        self.state
            .try_reduce(server::state::Action::Project {
                path: project_path.clone(),
                action: server::state::project::Action::Container {
                    path: container_graph_path.clone(),
                    action: server::state::project::action::Container::SetSettings(
                        state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound)),
                    ),
                },
            })
            .unwrap();

        vec![Update::project_with_id(
            project_id,
            project_path,
            update::Project::Container {
                path: container_graph_path.clone(),
                update: update::Container::Settings(update::DataResource::Removed),
            },
            event.id().clone(),
        )]
    }

    fn handle_fs_event_container_settings_modified(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Container(event::Container::Settings(event::StaticResourceEvent::Modified(
            kind,
        ))) = event.kind()
        else {
            panic!("invalid event kind");
        };

        match kind {
            event::ModifiedKind::Data => {
                self.handle_fs_event_container_settings_modified_data(event)
            }
            event::ModifiedKind::Other => {
                self.handle_fs_event_container_settings_modified_other(event)
            }
        }
    }

    fn handle_fs_event_container_settings_modified_data(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Container(event::Container::Settings(
                event::StaticResourceEvent::Modified(event::ModifiedKind::Data)
            ))
        );

        self.handle_container_settings_modified(event)
    }

    fn handle_fs_event_container_settings_modified_other(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Container(event::Container::Settings(
                event::StaticResourceEvent::Modified(event::ModifiedKind::Other)
            ))
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        #[cfg(target_os = "windows")]
        {
            self.handle_container_settings_modified(event)
        }

        #[cfg(target_os = "macos")]
        {
            todo!();
        }

        #[cfg(target_os = "linux")]
        {
            todo!();
        }
    }

    fn handle_container_settings_modified(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::FolderResource::Present(graph) = project_state.graph() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let container_graph_path = base_path
            .strip_prefix(project.path().join(&project_properties.data_root))
            .unwrap();
        let container_graph_path = common::prepend_root_dir(container_graph_path);
        let container_state = graph.find(&container_graph_path).unwrap().unwrap();
        let container_state = container_state.lock().unwrap();
        assert!(!matches!(
            container_state.settings(),
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        ));

        let settings = loader::container::Loader::load_from_only_settings(base_path);
        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        let update = match (&container_state.settings, settings.clone()) {
            (Ok(state), Ok(settings)) => {
                if settings == *state {
                    return vec![];
                }

                Update::project_with_id(
                    project_id,
                    project_path.clone(),
                    update::Project::Container {
                        path: container_graph_path.clone(),
                        update: update::Container::Settings(update::DataResource::Modified(
                            settings,
                        )),
                    },
                    event.id().clone(),
                )
            }
            (Err(_state), Ok(settings)) => Update::project_with_id(
                project_id,
                project_path.clone(),
                update::Project::Container {
                    path: container_graph_path.clone(),
                    update: update::Container::Settings(update::DataResource::Repaired(settings)),
                },
                event.id().clone(),
            ),
            (_, Err(IoSerde::Io(io::ErrorKind::NotFound))) => todo!(),
            (Ok(_), Err(err)) => Update::project_with_id(
                project_id,
                project_path.clone(),
                update::Project::Container {
                    path: container_graph_path.clone(),
                    update: update::Container::Settings(update::DataResource::Corrupted(err)),
                },
                event.id().clone(),
            ),
            (Err(state), Err(err)) => {
                if err == *state {
                    return vec![];
                }

                Update::project_with_id(
                    project_id,
                    project_path.clone(),
                    update::Project::Container {
                        path: container_graph_path.clone(),
                        update: update::Container::Settings(update::DataResource::Corrupted(err)),
                    },
                    event.id().clone(),
                )
            }
        };

        drop(container_state);
        self.state
            .try_reduce(server::state::Action::Project {
                path: project_path,
                action: server::state::project::Action::Container {
                    path: container_graph_path,
                    action: server::state::project::action::Container::SetSettings(settings),
                },
            })
            .unwrap();
        vec![update]
    }
}

impl Database {
    fn handle_fs_event_container_assets(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        let EventKind::Container(event::Container::Assets(kind)) = event.kind() else {
            panic!("invalid event kind");
        };

        match kind {
            event::StaticResourceEvent::Created => {
                self.handle_fs_event_container_assets_created(event)
            }
            event::StaticResourceEvent::Removed => {
                self.handle_fs_event_container_assets_removed(event)
            }
            event::StaticResourceEvent::Modified(_) => {
                self.handle_fs_event_container_assets_modified(event)
            }
        }
    }

    fn handle_fs_event_container_assets_created(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Container(event::Container::Assets(
                event::StaticResourceEvent::Created
            ))
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::FolderResource::Present(graph) = project_state.graph() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let container_graph_path = base_path
            .strip_prefix(project.path().join(&project_properties.data_root))
            .unwrap();
        let container_graph_path = common::prepend_root_dir(container_graph_path);
        let container_state = graph.find(&container_graph_path).unwrap().unwrap();
        let container_state = container_state.lock().unwrap();
        if cfg!(target_os = "windows") {
            if !matches!(
                container_state.assets(),
                state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
            ) {
                tracing::warn!("asset created event occurred late");
            }
        } else {
            assert_matches!(
                container_state.assets(),
                state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
            );
        }
        drop(container_state);

        let assets = loader::container::Loader::load_from_only_assets(base_path);
        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        match assets {
            Ok(assets) => {
                let assets = assets::from_assets(base_path, assets.into_inner());
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: project_path.clone(),
                        action: server::state::project::Action::Container {
                            path: container_graph_path.clone(),
                            action: server::state::project::action::Container::SetAssets(
                                state::DataResource::Ok(assets.clone()),
                            ),
                        },
                    })
                    .unwrap();

                vec![Update::project_with_id(
                    project_id,
                    base_path,
                    update::Project::Container {
                        path: container_graph_path.clone(),
                        update: update::Container::Assets(update::DataResource::Created(
                            state::DataResource::Ok(assets),
                        )),
                    },
                    event.id().clone(),
                )]
            }
            Err(IoSerde::Io(io::ErrorKind::NotFound)) => {
                vec![]
            }
            Err(err) => {
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: project_path.clone(),
                        action: server::state::project::Action::Container {
                            path: container_graph_path.clone(),
                            action: server::state::project::action::Container::SetAssets(
                                state::DataResource::Err(err.clone()),
                            ),
                        },
                    })
                    .unwrap();

                vec![Update::project_with_id(
                    project_id,
                    project_path,
                    update::Project::Container {
                        path: container_graph_path.clone(),
                        update: update::Container::Assets(update::DataResource::Created(
                            state::DataResource::Err(err),
                        )),
                    },
                    event.id().clone(),
                )]
            }
        }
    }

    fn handle_fs_event_container_assets_removed(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Container(event::Container::Assets(
                event::StaticResourceEvent::Removed
            ))
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::FolderResource::Present(graph) = project_state.graph() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let container_graph_path = base_path
            .strip_prefix(project.path().join(&project_properties.data_root))
            .unwrap();
        let container_graph_path = common::prepend_root_dir(container_graph_path);
        let container_state = graph.find(&container_graph_path).unwrap().unwrap();
        let container_state = container_state.lock().unwrap();
        assert!(!matches!(
            container_state.assets(),
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        ));
        drop(container_state);

        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        self.state
            .try_reduce(server::state::Action::Project {
                path: project_path.clone(),
                action: server::state::project::Action::Container {
                    path: container_graph_path.clone(),
                    action: server::state::project::action::Container::SetAssets(
                        state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound)),
                    ),
                },
            })
            .unwrap();

        vec![Update::project_with_id(
            project_id,
            project_path,
            update::Project::Container {
                path: container_graph_path.clone(),
                update: update::Container::Assets(update::DataResource::Removed),
            },
            event.id().clone(),
        )]
    }

    fn handle_fs_event_container_assets_modified(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Container(event::Container::Assets(event::StaticResourceEvent::Modified(
            kind,
        ))) = event.kind()
        else {
            panic!("invalid event kind");
        };

        match kind {
            event::ModifiedKind::Data => self.handle_fs_event_container_assets_modified_data(event),
            event::ModifiedKind::Other => {
                self.handle_fs_event_container_assets_modified_other(event)
            }
        }
    }

    fn handle_fs_event_container_assets_modified_data(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Container(event::Container::Assets(
                event::StaticResourceEvent::Modified(event::ModifiedKind::Data),
            )),
        );

        self.handle_container_assets_modified(event)
    }

    fn handle_fs_event_container_assets_modified_other(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Container(event::Container::Assets(
                event::StaticResourceEvent::Modified(event::ModifiedKind::Other),
            )),
        );

        #[cfg(target_os = "windows")]
        {
            self.handle_container_assets_modified(event)
        }

        #[cfg(target_os = "macos")]
        {
            todo!();
        }

        #[cfg(target_os = "linux")]
        {
            todo!();
        }
    }

    fn handle_container_assets_modified(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::FolderResource::Present(graph) = project_state.graph() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let container_graph_path = base_path
            .strip_prefix(project.path().join(&project_properties.data_root))
            .unwrap();
        let container_graph_path = common::prepend_root_dir(container_graph_path);
        let container_state = graph.find(&container_graph_path).unwrap().unwrap();
        let container_state = container_state.lock().unwrap();
        assert!(!matches!(
            container_state.assets(),
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        ));

        let assets = loader::container::Loader::load_from_only_assets(base_path);
        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        let (assets, update) = match (&container_state.assets, assets.clone()) {
            (Ok(state), Ok(assets)) => {
                let assets = assets::from_assets(base_path, assets.into_inner());
                if assets == *state {
                    // TODO: Ignore order for comparison.
                    return vec![];
                }

                let update = Update::project_with_id(
                    project_id,
                    project_path.clone(),
                    update::Project::Container {
                        path: container_graph_path.clone(),
                        update: update::Container::Assets(update::DataResource::Modified(
                            assets.clone(),
                        )),
                    },
                    event.id().clone(),
                );

                (state::DataResource::Ok(assets), update)
            }
            (Err(_state), Ok(assets)) => {
                let assets = assets::from_assets(base_path, assets.into_inner());
                let update = Update::project_with_id(
                    project_id,
                    project_path.clone(),
                    update::Project::Container {
                        path: container_graph_path.clone(),
                        update: update::Container::Assets(update::DataResource::Repaired(
                            assets.clone(),
                        )),
                    },
                    event.id().clone(),
                );

                (state::DataResource::Ok(assets), update)
            }
            (_, Err(IoSerde::Io(io::ErrorKind::NotFound))) => todo!(),
            (Ok(_), Err(err)) => {
                let update = Update::project_with_id(
                    project_id,
                    project_path.clone(),
                    update::Project::Container {
                        path: container_graph_path.clone(),
                        update: update::Container::Assets(update::DataResource::Corrupted(
                            err.clone(),
                        )),
                    },
                    event.id().clone(),
                );

                (state::DataResource::Err(err), update)
            }
            (Err(state), Err(err)) => {
                if err == *state {
                    return vec![];
                }

                let update = Update::project_with_id(
                    project_id,
                    project_path.clone(),
                    update::Project::Container {
                        path: container_graph_path.clone(),
                        update: update::Container::Assets(update::DataResource::Corrupted(
                            err.clone(),
                        )),
                    },
                    event.id().clone(),
                );

                (state::DataResource::Err(err), update)
            }
        };

        drop(container_state);
        self.state
            .try_reduce(server::state::Action::Project {
                path: project_path,
                action: server::state::project::Action::Container {
                    path: container_graph_path,
                    action: server::state::project::action::Container::SetAssets(assets),
                },
            })
            .unwrap();
        vec![update]
    }
}

mod assets {
    use crate::state;
    use std::path::Path;
    use syre_core::project::Asset;

    /// Create asset states from list of assets by checking if paths
    /// are present in the file system.
    ///
    /// # Arguments
    /// 1. `container`: Absolute path to the container (from the file system root).
    /// 2. `assets`: List of assets.
    pub fn from_assets(container: impl AsRef<Path>, assets: Vec<Asset>) -> Vec<state::Asset> {
        let container = container.as_ref();
        assets
            .into_iter()
            .map(|asset| {
                if container.join(&asset.path).is_file() {
                    state::Asset::present(asset)
                } else {
                    state::Asset::absent(asset)
                }
            })
            .collect()
    }
}

impl Database {
    fn handle_fs_event_container_flags(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        let EventKind::Container(event::Container::Flags(kind)) = event.kind() else {
            panic!("invalid event kind");
        };

        match kind {
            event::StaticResourceEvent::Created => {
                self.handle_fs_event_container_flags_created(event)
            }
            event::StaticResourceEvent::Removed => {
                self.handle_fs_event_container_flags_removed(event)
            }
            event::StaticResourceEvent::Modified(_) => {
                self.handle_fs_event_container_flags_modified(event)
            }
        }
    }

    fn handle_fs_event_container_flags_created(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Container(event::Container::Flags(event::StaticResourceEvent::Created))
        );

        let path = match &event.paths()[..] {
            [path] => path,
            [_from, to] => to,
            _ => panic!("invalid paths"),
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::FolderResource::Present(graph) = project_state.graph() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let container_graph_path = base_path
            .strip_prefix(project.path().join(&project_properties.data_root))
            .unwrap();
        let container_graph_path = common::prepend_root_dir(container_graph_path);

        let flags = loader::container::flags::Loader::load(base_path);
        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        match flags {
            Ok(flags) => {
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: project_path.clone(),
                        action: server::state::project::Action::Container {
                            path: container_graph_path.clone(),
                            action: server::state::project::action::Container::SetFlags(
                                state::DataResource::Ok(flags.clone()),
                            ),
                        },
                    })
                    .unwrap();

                vec![Update::project_with_id(
                    project_id,
                    base_path,
                    update::Project::Container {
                        path: container_graph_path.clone(),
                        update: update::Container::Flags(update::DataResource::Created(
                            state::DataResource::Ok(flags),
                        )),
                    },
                    event.id().clone(),
                )]
            }
            Err(IoSerde::Io(io::ErrorKind::NotFound)) => {
                vec![]
            }
            Err(err) => {
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: project_path.clone(),
                        action: server::state::project::Action::Container {
                            path: container_graph_path.clone(),
                            action: server::state::project::action::Container::SetFlags(
                                state::DataResource::Err(err.clone()),
                            ),
                        },
                    })
                    .unwrap();

                vec![Update::project_with_id(
                    project_id,
                    project_path,
                    update::Project::Container {
                        path: container_graph_path.clone(),
                        update: update::Container::Flags(update::DataResource::Created(
                            state::DataResource::Err(err),
                        )),
                    },
                    event.id().clone(),
                )]
            }
        }
    }

    fn handle_fs_event_container_flags_removed(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Container(event::Container::Flags(event::StaticResourceEvent::Removed))
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::FolderResource::Present(graph) = project_state.graph() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let container_graph_path = base_path
            .strip_prefix(project.path().join(&project_properties.data_root))
            .unwrap();
        let container_graph_path = common::prepend_root_dir(container_graph_path);
        let container_state = graph.find(&container_graph_path).unwrap().unwrap();
        let container_state = container_state.lock().unwrap();
        assert!(!matches!(
            container_state.flags(),
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        ));
        drop(container_state);

        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        self.state
            .try_reduce(server::state::Action::Project {
                path: project_path.clone(),
                action: server::state::project::Action::Container {
                    path: container_graph_path.clone(),
                    action: server::state::project::action::Container::SetFlags(
                        state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound)),
                    ),
                },
            })
            .unwrap();

        vec![Update::project_with_id(
            project_id,
            project_path,
            update::Project::Container {
                path: container_graph_path.clone(),
                update: update::Container::Flags(update::DataResource::Removed),
            },
            event.id().clone(),
        )]
    }

    fn handle_fs_event_container_flags_modified(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Container(event::Container::Flags(event::StaticResourceEvent::Modified(
            kind,
        ))) = event.kind()
        else {
            panic!("invalid event kind");
        };

        match kind {
            event::ModifiedKind::Data => self.handle_fs_event_container_flags_modified_data(event),
            event::ModifiedKind::Other => {
                self.handle_fs_event_container_flags_modified_other(event)
            }
        }
    }

    fn handle_fs_event_container_flags_modified_data(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Container(event::Container::Flags(
                event::StaticResourceEvent::Modified(event::ModifiedKind::Data),
            )),
        );

        self.handle_container_flags_modified(event)
    }

    fn handle_fs_event_container_flags_modified_other(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Container(event::Container::Flags(
                event::StaticResourceEvent::Modified(event::ModifiedKind::Other),
            )),
        );

        #[cfg(target_os = "windows")]
        {
            self.handle_container_flags_modified(event)
        }

        #[cfg(target_os = "macos")]
        {
            todo!();
        }

        #[cfg(target_os = "linux")]
        {
            todo!();
        }
    }

    fn handle_container_flags_modified(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::FolderResource::Present(graph) = project_state.graph() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let container_graph_path = base_path
            .strip_prefix(project.path().join(&project_properties.data_root))
            .unwrap();
        let container_graph_path = common::prepend_root_dir(container_graph_path);
        let container_state = graph.find(&container_graph_path).unwrap().unwrap();
        let container_state = container_state.lock().unwrap();
        assert!(!matches!(
            container_state.flags(),
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        ));

        let flags = loader::container::flags::Loader::load(base_path);
        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        let (flags, update) = match (&container_state.flags, flags) {
            (Ok(state), Ok(flags)) => {
                if flags == *state {
                    // TODO: Ignore order for comparison.
                    return vec![];
                }

                let update = Update::project_with_id(
                    project_id,
                    project_path.clone(),
                    update::Project::Container {
                        path: container_graph_path.clone(),
                        update: update::Container::Flags(update::DataResource::Modified(
                            flags.clone(),
                        )),
                    },
                    event.id().clone(),
                );

                (state::DataResource::Ok(flags), update)
            }
            (Err(_state), Ok(flags)) => {
                let update = Update::project_with_id(
                    project_id,
                    project_path.clone(),
                    update::Project::Container {
                        path: container_graph_path.clone(),
                        update: update::Container::Flags(update::DataResource::Repaired(
                            flags.clone(),
                        )),
                    },
                    event.id().clone(),
                );

                (state::DataResource::Ok(flags), update)
            }
            (_, Err(IoSerde::Io(io::ErrorKind::NotFound))) => todo!(),
            (Ok(_), Err(err)) => {
                let update = Update::project_with_id(
                    project_id,
                    project_path.clone(),
                    update::Project::Container {
                        path: container_graph_path.clone(),
                        update: update::Container::Flags(update::DataResource::Corrupted(
                            err.clone(),
                        )),
                    },
                    event.id().clone(),
                );

                (state::DataResource::Err(err), update)
            }
            (Err(state), Err(err)) => {
                if err == *state {
                    return vec![];
                }

                let update = Update::project_with_id(
                    project_id,
                    project_path.clone(),
                    update::Project::Container {
                        path: container_graph_path.clone(),
                        update: update::Container::Flags(update::DataResource::Corrupted(
                            err.clone(),
                        )),
                    },
                    event.id().clone(),
                );

                (state::DataResource::Err(err), update)
            }
        };

        drop(container_state);
        self.state
            .try_reduce(server::state::Action::Project {
                path: project_path,
                action: server::state::project::Action::Container {
                    path: container_graph_path,
                    action: server::state::project::action::Container::SetFlags(flags),
                },
            })
            .unwrap();
        vec![update]
    }
}
