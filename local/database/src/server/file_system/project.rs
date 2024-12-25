use crate::{
    common,
    event::{self as update, Update},
    server, state, Database,
};
use std::{assert_matches::assert_matches, io, path};
use syre_fs_watcher::{event, EventKind};
use syre_local::{
    self as local,
    error::IoSerde,
    project::resources::{project::LoadError, Analyses, Project},
    types::AnalysisKind,
    TryReducible,
};

impl Database {
    pub(super) fn handle_fs_event_project(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        let EventKind::Project(kind) = event.kind() else {
            panic!("invalid event kind");
        };

        match kind {
            event::Project::Created => todo!(),
            event::Project::FolderRemoved => self.handle_fs_event_project_folder_removed(event),
            event::Project::Moved => todo!(),
            event::Project::ConfigDir(_) => self.handle_fs_event_project_config_dir(event),
            event::Project::AnalysisDir(_) => self.handle_fs_event_project_analysis_dir(event),
            event::Project::DataDir(_) => self.handle_fs_event_project_data_dir(event),
            event::Project::Properties(_) => self.handle_fs_event_project_properties(event),
            event::Project::Settings(_) => self.handle_fs_event_project_settings(event),
            event::Project::Analyses(_) => self.handle_fs_event_project_analyses(event),
            event::Project::Modified => todo!(),
        }
    }
}

impl Database {
    fn handle_fs_event_project_folder_removed(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Project(event::Project::FolderRemoved)
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let project_state = self.state.find_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project_state.fs_resource() else {
            panic!("invalid state");
        };

        let project_id = project_state
            .properties()
            .map(|properties| properties.rid().clone())
            .ok();

        if self.config.handle_fs_resource_changes() {
            local::system::project_manifest::deregister_project(path).unwrap();
            vec![]
        } else {
            self.state
                .try_reduce(server::state::Action::Project {
                    path: path.clone(),
                    action: server::state::project::Action::RemoveFolder.into(),
                })
                .unwrap();

            vec![Update::project(
                project_id,
                path.clone(),
                update::Project::FolderRemoved,
                event.id().clone(),
            )]
        }
    }
}

impl Database {
    fn handle_fs_event_project_config_dir(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        let EventKind::Project(event::Project::ConfigDir(kind)) = event.kind() else {
            panic!("invalid event kind");
        };

        match kind {
            event::StaticResourceEvent::Created => {
                self.handle_fs_event_project_config_dir_created(event)
            }
            event::StaticResourceEvent::Removed => {
                self.handle_fs_event_project_config_dir_removed(event)
            }
            event::StaticResourceEvent::Modified(_) => unreachable!(),
        }
    }

    fn handle_fs_event_project_config_dir_created(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Project(event::Project::ConfigDir(
                event::StaticResourceEvent::Created
            ))
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap();
        let project_state = self.state.find_project_by_path(base_path).unwrap();
        let state::FolderResource::Present(project_state) = project_state.fs_resource() else {
            panic!("invalid state");
        };

        assert_matches!(
            project_state.properties(),
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        );
        assert_matches!(
            project_state.settings(),
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        );
        assert_matches!(
            project_state.analyses(),
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        );

        let mut updates = vec![];
        let project = Project::load_from(base_path);
        match project {
            Ok(project) => {
                let (properties, settings, project_path) = project.into_parts();
                assert_eq!(base_path, project_path);
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: base_path.to_path_buf(),
                        action: server::state::project::Action::SetProperties(
                            state::DataResource::Ok(properties.clone()),
                        ),
                    })
                    .unwrap();

                self.state
                    .try_reduce(server::state::Action::Project {
                        path: base_path.to_path_buf(),
                        action: server::state::project::Action::SetSettings(
                            state::DataResource::Ok(settings.clone()),
                        )
                        .into(),
                    })
                    .unwrap();

                let project_id = properties.rid().clone();
                updates.extend([
                    Update::project_with_id(
                        project_id.clone(),
                        base_path.to_path_buf(),
                        update::Project::Properties(update::DataResource::Created(Ok(properties))),
                        event.id().clone(),
                    ),
                    Update::project_with_id(
                        project_id,
                        base_path.to_path_buf(),
                        update::Project::Settings(update::DataResource::Created(Ok(settings))),
                        event.id().clone(),
                    ),
                ]);
            }

            Err(LoadError {
                properties,
                settings,
            }) => {
                let mut project_id = None;
                if !matches!(
                    properties,
                    state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
                ) {
                    if let Ok(properties) = properties.as_ref() {
                        project_id = Some(properties.rid().clone());
                    }

                    self.state
                        .try_reduce(server::state::Action::Project {
                            path: base_path.to_path_buf(),
                            action: server::state::project::Action::SetProperties(
                                properties.clone(),
                            ),
                        })
                        .unwrap();

                    let update = match properties {
                        Ok(properties) => Update::project_with_id(
                            properties.rid().clone(),
                            base_path.to_path_buf(),
                            update::Project::Properties(update::DataResource::Created(Ok(
                                properties,
                            ))),
                            event.id().clone(),
                        ),

                        Err(err) => Update::project_no_id(
                            base_path.to_path_buf(),
                            update::Project::Properties(update::DataResource::Created(Err(err))),
                            event.id().clone(),
                        ),
                    };

                    updates.push(update);
                }

                if !matches!(
                    settings,
                    state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
                ) {
                    self.state
                        .try_reduce(server::state::Action::Project {
                            path: base_path.to_path_buf(),
                            action: server::state::project::Action::SetSettings(settings.clone()),
                        })
                        .unwrap();

                    let update = match settings {
                        Ok(settings) => Update::project(
                            project_id,
                            base_path.to_path_buf(),
                            update::Project::Settings(update::DataResource::Created(Ok(settings))),
                            event.id().clone(),
                        ),
                        Err(err) => Update::project(
                            project_id,
                            base_path.to_path_buf(),
                            update::Project::Settings(update::DataResource::Created(Err(err))),
                            event.id().clone(),
                        ),
                    };

                    updates.push(update);
                }
            }
        }

        match Analyses::load_from(&base_path) {
            Ok(manifest) => todo!(),
            Err(IoSerde::Io(io::ErrorKind::NotFound)) => {}
            Err(err) => todo!(),
        }

        updates
    }

    fn handle_fs_event_project_config_dir_removed(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Project(event::Project::ConfigDir(
                event::StaticResourceEvent::Removed
            ))
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap();
        let project_state = self.state.find_project_by_path(base_path).unwrap();
        let state::FolderResource::Present(project_state) = project_state.fs_resource() else {
            panic!("invalid state");
        };

        let mut updates = vec![];
        let mut project_id = None;
        match project_state.properties().as_ref() {
            Ok(properties) => {
                project_id = Some(properties.rid().clone());
                updates.push(Update::project_with_id(
                    properties.rid().clone(),
                    base_path,
                    update::Project::Properties(update::DataResource::Removed),
                    event.id().clone(),
                ));
            }
            Err(IoSerde::Io(err)) if *err == io::ErrorKind::NotFound => {}
            Err(_) => {
                updates.push(Update::project_no_id(
                    base_path,
                    update::Project::Properties(update::DataResource::Removed),
                    event.id().clone(),
                ));
            }
        }

        if !matches!( project_state.settings().as_ref(),
            Err(IoSerde::Io(err)) if *err == io::ErrorKind::NotFound)
        {
            if let Some(project_id) = project_id {
                updates.push(Update::project_with_id(
                    project_id,
                    base_path,
                    update::Project::Properties(update::DataResource::Removed),
                    event.id().clone(),
                ));
            } else {
                updates.push(Update::project_no_id(
                    base_path,
                    update::Project::Settings(update::DataResource::Removed),
                    event.id().clone(),
                ));
            }
        }

        self.state
            .try_reduce(server::state::Action::Project {
                path: base_path.to_path_buf(),
                action: server::state::project::Action::RemoveConfig,
            })
            .unwrap();

        updates
    }
}

impl Database {
    fn handle_fs_event_project_analysis_dir(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Project(event::Project::AnalysisDir(kind)) = event.kind() else {
            panic!("invalid event kind");
        };

        match kind {
            event::ResourceEvent::Created => {
                self.handle_fs_event_project_analysis_dir_created(event)
            }
            event::ResourceEvent::Removed => todo!(),
            event::ResourceEvent::Renamed => todo!(),
            event::ResourceEvent::Moved => todo!(),
            event::ResourceEvent::MovedProject => todo!(),
            event::ResourceEvent::Modified(_) => todo!(),
        }
    }

    fn handle_fs_event_project_analysis_dir_created(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        use syre_local::types::AnalysisKind;

        assert_matches!(
            event.kind(),
            EventKind::Project(event::Project::AnalysisDir(event::ResourceEvent::Created))
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(properties) = project_state.properties() else {
            panic!("invalid state");
        };
        assert_eq!(
            properties.analysis_root.as_ref().unwrap(),
            path.strip_prefix(project.path()).unwrap()
        );

        let state::DataResource::Ok(analyses) = project_state.analyses() else {
            return vec![];
        };
        let mut analyses = analyses.clone();

        let mut modified = false;
        for analysis in analyses.iter_mut() {
            assert!(!analysis.is_present());
            let analysis_path = match analysis.properties() {
                AnalysisKind::Script(script) => path.join(&script.path),
                AnalysisKind::ExcelTemplate(template) => path.join(&template.template.path),
            };

            if analysis_path.is_file() {
                analysis.set_present();
                modified = true;
            }
        }

        if !modified {
            return vec![];
        }

        let project_path = project.path().clone();
        let project_id = properties.rid().clone();
        self.state
            .try_reduce(server::state::Action::Project {
                path: project_path.clone(),
                action: server::state::project::Action::SetAnalyses(state::DataResource::Ok(
                    analyses.clone(),
                )),
            })
            .unwrap();

        vec![Update::project_with_id(
            project_id,
            project_path,
            update::Project::Analyses(update::DataResource::Modified(analyses.clone())),
            event.id().clone(),
        )]
    }
}

impl Database {
    fn handle_fs_event_project_data_dir(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        let EventKind::Project(event::Project::DataDir(kind)) = event.kind() else {
            panic!("invalid event kind");
        };

        match kind {
            event::ResourceEvent::Created => self.handle_fs_event_project_data_dir_created(event),
            event::ResourceEvent::Removed => todo!(),
            event::ResourceEvent::Renamed => self.handle_fs_event_project_data_dir_renamed(event),
            event::ResourceEvent::Moved => todo!(),
            event::ResourceEvent::MovedProject => todo!(),
            event::ResourceEvent::Modified(_) => {
                self.handle_fs_event_project_data_dir_modified(event)
            }
        }
    }

    fn handle_fs_event_project_data_dir_created(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Project(event::Project::DataDir(event::ResourceEvent::Created)) =
            event.kind()
        else {
            panic!("invalid event kind");
        };

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let data_root_path = project.path().join(&properties.data_root);
        assert_eq!(*path, data_root_path);
        assert!(!project_state.graph().is_present());

        let project_path = project.path().clone();
        let project_id = properties.rid().clone();
        let graph = server::state::project::graph::State::load(path).unwrap();
        let graph_state = graph.as_graph();
        self.state
            .try_reduce(server::state::Action::Project {
                path: project_path.clone(),
                action: server::state::project::action::Graph::Set(state::FolderResource::Present(
                    graph,
                ))
                .into(),
            })
            .unwrap();

        vec![Update::project_with_id(
            project_id,
            project_path,
            update::Graph::Created(graph_state).into(),
            event.id().clone(),
        )]
    }

    fn handle_fs_event_project_data_dir_renamed(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Project(event::Project::DataDir(event::ResourceEvent::Renamed)) =
            event.kind()
        else {
            panic!("invalid event kind");
        };

        let [from, to] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let project = self.state.find_resource_project_by_path(from).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let data_root_path = project.path().join(&properties.data_root);
        assert_eq!(*from, data_root_path);
        assert!(project_state.graph().is_present());

        let project_path = project.path().clone();
        let project_id = properties.rid().clone();

        let from_path = common::container_graph_path(&data_root_path, from).unwrap();
        assert_eq!(from_path, local::common::root_path());
        let to_path = common::container_graph_path(&project_path, to).unwrap();
        let name = to_path.as_os_str().to_os_string();
        let mut properties = properties.clone();

        self.state
            .try_reduce(server::state::Action::Project {
                path: project_path.clone(),
                action: server::state::project::action::Action::Container {
                    path: from_path.clone(),
                    action: server::state::project::action::Container::SetName(name.clone()),
                }
                .into(),
            })
            .unwrap();

        if self.config.handle_fs_resource_changes() {
            assert_matches!(
                to_path.components().next().unwrap(),
                path::Component::RootDir
            );
            properties.data_root = to_path.clone().into_iter().skip(1).collect();
            if let Err(err) =
                local::project::resources::Project::save_properties_only(&project_path, &properties)
            {
                tracing::error!(?err);
                todo!();
            }

            let mut container =
                match local::loader::container::Loader::load_from_only_properties(to) {
                    Ok(container) => container,
                    Err(err) => {
                        tracing::error!(?err);
                        todo!();
                    }
                };
            container.properties.name = to.file_name().unwrap().to_string_lossy().to_string();
            if let Err(err) = container.save(to) {
                tracing::error!(?err);
                todo!();
            }
        }

        vec![Update::project_with_id(
            project_id,
            project_path,
            update::Project::Graph(update::Graph::Renamed {
                from: from_path,
                to: name,
            })
            .into(),
            event.id().clone(),
        )]
    }

    fn handle_fs_event_project_data_dir_modified(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Project(event::Project::DataDir(event::ResourceEvent::Modified(kind))) =
            event.kind()
        else {
            panic!("invalid event kind");
        };

        match kind {
            event::ModifiedKind::Data => todo!(),
            event::ModifiedKind::Other => {
                self.handle_fs_event_project_data_dir_modified_other(event)
            }
        }
    }

    fn handle_fs_event_project_data_dir_modified_other(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Project(event::Project::DataDir(event::ResourceEvent::Modified(
                event::ModifiedKind::Other
            )))
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let data_root_path = project.path().join(&properties.data_root);
        #[cfg(target_os = "windows")]
        if *path == data_root_path {
            return vec![];
        } else {
            todo!();
        }

        #[cfg(not(target_os = "windows"))]
        if *path == data_root_path {
            todo!();
        } else {
            todo!();
        }
    }
}

impl Database {
    fn handle_fs_event_project_properties(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        let EventKind::Project(event::Project::Properties(kind)) = event.kind() else {
            panic!("invalid event kind");
        };

        match kind {
            event::StaticResourceEvent::Created => {
                self.handle_fs_event_project_properties_created(event)
            }
            event::StaticResourceEvent::Removed => {
                self.handle_fs_event_project_properties_removed(event)
            }
            event::StaticResourceEvent::Modified(_) => {
                self.handle_fs_event_project_properties_modified(event)
            }
        }
    }

    fn handle_fs_event_project_settings(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        let EventKind::Project(event::Project::Settings(kind)) = event.kind() else {
            panic!("invalid event kind");
        };

        match kind {
            event::StaticResourceEvent::Created => {
                self.handle_fs_event_project_settings_created(event)
            }
            event::StaticResourceEvent::Removed => {
                self.handle_fs_event_project_settings_removed(event)
            }
            event::StaticResourceEvent::Modified(_) => {
                self.handle_fs_event_project_settings_modified(event)
            }
        }
    }

    fn handle_fs_event_project_properties_created(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Project(event::Project::Properties(event::StaticResourceEvent::Created)) =
            event.kind()
        else {
            panic!("invalid event kind");
        };

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project_state = self.state.find_project_by_path(base_path).unwrap();
        let state::FolderResource::Present(project) = project_state.fs_resource() else {
            panic!("invalid state");
        };

        assert_matches!(
            project.properties(),
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        );

        match Project::load_from_properties_only(base_path) {
            Ok(properties) => {
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: base_path.to_path_buf(),
                        action: server::state::project::Action::SetProperties(
                            state::DataResource::Ok(properties.clone()),
                        ),
                    })
                    .unwrap();

                vec![Update::project_with_id(
                    properties.rid().clone(),
                    base_path,
                    update::Project::Properties(update::DataResource::Created(Ok(properties))),
                    event.id().clone(),
                )]
            }

            Err(IoSerde::Io(io::ErrorKind::NotFound)) => todo!(),
            Err(err) => {
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: base_path.to_path_buf(),
                        action: server::state::project::Action::SetProperties(
                            state::DataResource::Err(err.clone()),
                        ),
                    })
                    .unwrap();

                vec![Update::project_no_id(
                    base_path,
                    update::Project::Properties(update::DataResource::Created(Err(err))),
                    event.id().clone(),
                )]
            }
        }
    }

    fn handle_fs_event_project_properties_removed(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Project(event::Project::Properties(event::StaticResourceEvent::Removed)) =
            event.kind()
        else {
            panic!("invalid event kind");
        };

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project_state = self.state.find_project_by_path(base_path).unwrap();
        let state::FolderResource::Present(project) = project_state.fs_resource() else {
            panic!("invalid state");
        };

        assert!(!matches!(
            project.properties(),
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        ));

        let project_id = if let state::DataResource::Ok(properties) = project.properties() {
            Some(properties.rid().clone())
        } else {
            None
        };

        self.state
            .try_reduce(server::state::Action::Project {
                path: base_path.to_path_buf(),
                action: server::state::project::Action::SetProperties(Err(IoSerde::Io(
                    io::ErrorKind::NotFound,
                ))),
            })
            .unwrap();

        vec![Update::project(
            project_id,
            base_path,
            update::Project::Properties(update::DataResource::Removed),
            event.id().clone(),
        )]
    }

    fn handle_fs_event_project_properties_modified(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Project(event::Project::Properties(event::StaticResourceEvent::Modified(
            kind,
        ))) = event.kind()
        else {
            panic!("invalid event kind");
        };

        match kind {
            event::ModifiedKind::Data => {
                self.handle_fs_event_project_properties_modified_data(event)
            }
            event::ModifiedKind::Other => {
                self.handle_fs_event_project_properties_modified_other(event)
            }
        }
    }

    fn handle_fs_event_project_properties_modified_data(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Project(event::Project::Properties(
                event::StaticResourceEvent::Modified(event::ModifiedKind::Data)
            ))
        );

        self.handle_project_properties_modified(event)
    }

    fn handle_fs_event_project_properties_modified_other(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Project(event::Project::Properties(
                event::StaticResourceEvent::Modified(event::ModifiedKind::Other)
            ))
        );

        #[cfg(target_os = "windows")]
        {
            self.handle_project_properties_modified(event)
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

    fn handle_project_properties_modified(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project_state = self.state.find_project_by_path(base_path).unwrap();
        let state::FolderResource::Present(project) = project_state.fs_resource() else {
            panic!("invalid state");
        };

        let state = project.properties();
        assert!(!matches!(
            state,
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        ));

        match (Project::load_from_properties_only(base_path), state) {
            (Ok(properties), Ok(state)) => {
                if properties == *state {
                    return vec![];
                }

                self.state
                    .try_reduce(server::state::Action::Project {
                        path: base_path.to_path_buf(),
                        action: server::state::project::Action::SetProperties(
                            state::DataResource::Ok(properties.clone()),
                        ),
                    })
                    .unwrap();

                vec![Update::project_with_id(
                    properties.rid().clone(),
                    base_path,
                    update::Project::Properties(update::DataResource::Modified(properties)),
                    event.id().clone(),
                )]
            }

            (Ok(properties), Err(_)) => {
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: base_path.to_path_buf(),
                        action: server::state::project::Action::SetProperties(
                            state::DataResource::Ok(properties.clone()),
                        ),
                    })
                    .unwrap();

                vec![Update::project_with_id(
                    properties.rid().clone(),
                    base_path,
                    update::Project::Properties(update::DataResource::Repaired(properties)),
                    event.id().clone(),
                )]
            }

            (Err(IoSerde::Io(io::ErrorKind::NotFound)), _) => todo!(),
            (Err(err), Ok(state)) => {
                let project_id = state.rid().clone();
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: base_path.to_path_buf(),
                        action: server::state::project::Action::SetProperties(
                            state::DataResource::Err(err.clone()),
                        ),
                    })
                    .unwrap();

                vec![Update::project_with_id(
                    project_id,
                    base_path,
                    update::Project::Properties(update::DataResource::Corrupted(err)),
                    event.id().clone(),
                )]
            }

            (Err(err), Err(_)) => {
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: base_path.to_path_buf(),
                        action: server::state::project::Action::SetProperties(
                            state::DataResource::Err(err.clone()),
                        ),
                    })
                    .unwrap();

                vec![Update::project_no_id(
                    base_path,
                    update::Project::Properties(update::DataResource::Corrupted(err)),
                    event.id().clone(),
                )]
            }
        }
    }

    fn handle_fs_event_project_settings_created(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Project(event::Project::Settings(event::StaticResourceEvent::Created)) =
            event.kind()
        else {
            panic!("invalid event kind");
        };

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project_state = self.state.find_project_by_path(base_path).unwrap();
        let state::FolderResource::Present(project) = project_state.fs_resource() else {
            panic!("invalid state");
        };

        assert_matches!(
            project.settings(),
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        );

        let project_id = if let state::DataResource::Ok(properties) = project.properties() {
            Some(properties.rid().clone())
        } else {
            None
        };

        match Project::load_from_settings_only(base_path) {
            Ok(settings) => {
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: base_path.to_path_buf(),
                        action: server::state::project::Action::SetSettings(
                            state::DataResource::Ok(settings.clone()),
                        ),
                    })
                    .unwrap();

                vec![Update::project(
                    project_id,
                    base_path,
                    update::Project::Settings(update::DataResource::Created(Ok(settings))),
                    event.id().clone(),
                )]
            }

            Err(IoSerde::Io(io::ErrorKind::NotFound)) => todo!(),
            Err(err) => {
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: base_path.to_path_buf(),
                        action: server::state::project::Action::SetSettings(
                            state::DataResource::Err(err.clone()),
                        ),
                    })
                    .unwrap();

                vec![Update::project(
                    project_id,
                    base_path,
                    update::Project::Settings(update::DataResource::Created(Err(err))),
                    event.id().clone(),
                )]
            }
        }
    }

    fn handle_fs_event_project_settings_removed(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Project(event::Project::Settings(event::StaticResourceEvent::Removed)) =
            event.kind()
        else {
            panic!("invalid event kind");
        };

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project_state = self.state.find_project_by_path(base_path).unwrap();
        let state::FolderResource::Present(project) = project_state.fs_resource() else {
            panic!("invalid state");
        };

        assert!(!matches!(
            project.settings(),
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        ));

        let project_id = if let state::DataResource::Ok(properties) = project.properties() {
            Some(properties.rid().clone())
        } else {
            None
        };

        self.state
            .try_reduce(server::state::Action::Project {
                path: base_path.to_path_buf(),
                action: server::state::project::Action::SetSettings(Err(IoSerde::Io(
                    io::ErrorKind::NotFound,
                ))),
            })
            .unwrap();

        vec![Update::project(
            project_id,
            base_path,
            update::Project::Settings(update::DataResource::Removed),
            event.id().clone(),
        )]
    }

    fn handle_fs_event_project_settings_modified(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Project(event::Project::Settings(event::StaticResourceEvent::Modified(
            kind,
        ))) = event.kind()
        else {
            panic!("invalid event kind");
        };

        if matches!(kind, event::ModifiedKind::Other) {
            todo!();
        }

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project_state = self.state.find_project_by_path(base_path).unwrap();
        let state::FolderResource::Present(project) = project_state.fs_resource() else {
            panic!("invalid state");
        };

        let project_id = if let state::DataResource::Ok(properties) = project.properties() {
            Some(properties.rid().clone())
        } else {
            None
        };

        let state = project.settings();
        assert!(!matches!(
            state,
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        ));

        match (Project::load_from_settings_only(base_path), state) {
            (Ok(settings), Ok(state)) => {
                if settings == *state {
                    return vec![];
                }

                self.state
                    .try_reduce(server::state::Action::Project {
                        path: base_path.to_path_buf(),
                        action: server::state::project::Action::SetSettings(
                            state::DataResource::Ok(settings.clone()),
                        ),
                    })
                    .unwrap();

                vec![Update::project(
                    project_id,
                    base_path,
                    update::Project::Settings(update::DataResource::Modified(settings)),
                    event.id().clone(),
                )]
            }

            (Ok(settings), Err(_)) => {
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: base_path.to_path_buf(),
                        action: server::state::project::Action::SetSettings(
                            state::DataResource::Ok(settings.clone()),
                        ),
                    })
                    .unwrap();

                vec![Update::project(
                    project_id,
                    base_path,
                    update::Project::Settings(update::DataResource::Repaired(settings)),
                    event.id().clone(),
                )]
            }

            (Err(IoSerde::Io(io::ErrorKind::NotFound)), _) => todo!(),
            (Err(err), _) => {
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: base_path.to_path_buf(),
                        action: server::state::project::Action::SetSettings(
                            state::DataResource::Err(err.clone()),
                        ),
                    })
                    .unwrap();

                vec![Update::project(
                    project_id,
                    base_path,
                    update::Project::Settings(update::DataResource::Corrupted(err)),
                    event.id().clone(),
                )]
            }
        }
    }
}

impl Database {
    fn handle_fs_event_project_analyses(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        let EventKind::Project(event::Project::Analyses(kind)) = event.kind() else {
            panic!("invalid event kind");
        };

        match kind {
            event::StaticResourceEvent::Created => {
                self.handle_fs_event_project_analyses_created(event)
            }
            event::StaticResourceEvent::Removed => {
                self.handle_fs_event_project_analyses_removed(event)
            }
            event::StaticResourceEvent::Modified(_) => {
                self.handle_fs_event_project_analyses_modified(event)
            }
        }
    }

    fn handle_fs_event_project_analyses_created(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Project(event::Project::Analyses(event::StaticResourceEvent::Created)) =
            event.kind()
        else {
            panic!("invalid event kind");
        };

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project_state = self.state.find_project_by_path(base_path).unwrap();
        let state::FolderResource::Present(project) = project_state.fs_resource() else {
            panic!("invalid state");
        };

        assert_matches!(
            project.analyses(),
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        );

        let project_id = if let state::DataResource::Ok(properties) = project.properties() {
            Some(properties.rid().clone())
        } else {
            None
        };

        match Analyses::load_from(base_path) {
            Ok(analyses) => {
                let state::DataResource::Ok(properties) = project.properties() else {
                    todo!();
                };

                let analysis_root = project_state
                    .path()
                    .join(properties.analysis_root.clone().unwrap());

                let analyses = analyses::from_resources(analyses.to_vec(), analysis_root);
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: base_path.to_path_buf(),
                        action: server::state::project::Action::SetAnalyses(
                            state::DataResource::Ok(analyses.clone()),
                        ),
                    })
                    .unwrap();

                vec![Update::project(
                    project_id,
                    base_path,
                    update::Project::Analyses(update::DataResource::Created(Ok(analyses))),
                    event.id().clone(),
                )]
            }

            Err(IoSerde::Io(io::ErrorKind::NotFound)) => todo!(),
            Err(err) => {
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: base_path.to_path_buf(),
                        action: server::state::project::Action::SetAnalyses(
                            state::DataResource::Err(err.clone()),
                        ),
                    })
                    .unwrap();

                vec![Update::project(
                    project_id,
                    base_path,
                    update::Project::Analyses(update::DataResource::Created(Err(err))),
                    event.id().clone(),
                )]
            }
        }
    }

    fn handle_fs_event_project_analyses_removed(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Project(event::Project::Analyses(event::StaticResourceEvent::Removed)) =
            event.kind()
        else {
            panic!("invalid event kind");
        };

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project_state = self.state.find_project_by_path(base_path).unwrap();
        let state::FolderResource::Present(project) = project_state.fs_resource() else {
            panic!("invalid state");
        };

        assert!(!matches!(
            project.settings(),
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        ));

        let project_id = if let state::DataResource::Ok(properties) = project.properties() {
            Some(properties.rid().clone())
        } else {
            None
        };

        self.state
            .try_reduce(server::state::Action::Project {
                path: base_path.to_path_buf(),
                action: server::state::project::Action::SetAnalyses(Err(IoSerde::Io(
                    io::ErrorKind::NotFound,
                ))),
            })
            .unwrap();

        vec![Update::project(
            project_id,
            base_path,
            update::Project::Analyses(update::DataResource::Removed),
            event.id().clone(),
        )]
    }

    fn handle_fs_event_project_analyses_modified(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::Project(event::Project::Analyses(event::StaticResourceEvent::Modified(
            kind,
        ))) = event.kind()
        else {
            panic!("invalid event kind");
        };

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        if matches!(kind, event::ModifiedKind::Other) {
            todo!();
        }

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let base_path = path.parent().unwrap().parent().unwrap();
        let project_state = self.state.find_project_by_path(base_path).unwrap();
        let state::FolderResource::Present(project) = project_state.fs_resource() else {
            panic!("invalid state");
        };

        let project_id = if let state::DataResource::Ok(properties) = project.properties() {
            Some(properties.rid().clone())
        } else {
            None
        };

        let state = project.analyses();
        assert!(!matches!(
            state,
            state::DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound))
        ));

        match (state, Analyses::load_from(base_path)) {
            (Ok(state), Ok(analyses)) => {
                let state::DataResource::Ok(properties) = project.properties() else {
                    todo!();
                };

                let analysis_root = project_state
                    .path()
                    .join(properties.analysis_root.clone().unwrap());
                let analyses = analyses::from_resources(analyses.to_vec(), &analysis_root);
                if analyses.len() == state.len() {
                    let mut equal = true;
                    for analysis in analyses.iter() {
                        if !state.contains(analysis) {
                            equal = false;
                            break;
                        }
                    }

                    if equal {
                        return vec![];
                    }
                }

                'handle: {
                    if self.config.handle_fs_resource_changes() {
                        let crate::state::FolderResource::Present(graph) = project.graph() else {
                            break 'handle;
                        };

                        let crate::state::DataResource::Ok(project_properties) =
                            project.properties()
                        else {
                            break 'handle;
                        };

                        let data_root = project_state.path().join(&project_properties.data_root);
                        let removed = state
                            .iter()
                            .filter_map(|state_analysis| {
                                let rid = match state_analysis.properties() {
                                    AnalysisKind::Script(script) => script.rid(),
                                    AnalysisKind::ExcelTemplate(template) => template.rid(),
                                };

                                if !analyses.iter().any(|analysis| match analysis.properties() {
                                    AnalysisKind::Script(script) => rid == script.rid(),
                                    AnalysisKind::ExcelTemplate(template) => rid == template.rid(),
                                }) {
                                    Some(rid)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();

                        for node in graph.nodes().iter() {
                            let container = node.lock().unwrap();
                            let crate::state::DataResource::Ok(container_analyses) =
                                container.analyses()
                            else {
                                continue;
                            };

                            let mut container_removed = Vec::with_capacity(removed.len());
                            for association in container_analyses {
                                if removed.contains(&association.analysis()) {
                                    container_removed.push(association.analysis().clone());
                                }
                            }
                            drop(container); // prevent deadlock when getting graph path

                            if !container_removed.is_empty() {
                                let container_path = crate::common::container_system_path(
                                    &data_root,
                                    graph.path(node).unwrap(),
                                );

                                let mut container_fs =
                                    local::loader::container::Loader::load_from_only_properties(
                                        &container_path,
                                    )
                                    .unwrap();

                                container_fs.analyses.retain(|association| {
                                    !container_removed.contains(&association.analysis())
                                });

                                container_fs.save(&container_path).unwrap();
                            }
                        }
                    }
                }

                self.state
                    .try_reduce(server::state::Action::Project {
                        path: base_path.to_path_buf(),
                        action: server::state::project::Action::SetAnalyses(
                            state::DataResource::Ok(analyses.clone()),
                        ),
                    })
                    .unwrap();

                vec![Update::project(
                    project_id,
                    base_path,
                    update::Project::Analyses(update::DataResource::Modified(analyses)),
                    event.id().clone(),
                )]
            }

            (Err(_), Ok(analyses)) => {
                let state::DataResource::Ok(properties) = project.properties() else {
                    todo!();
                };

                let analysis_root = project_state
                    .path()
                    .join(properties.analysis_root.clone().unwrap());
                let analyses = analyses::from_resources(analyses.to_vec(), analysis_root);
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: base_path.to_path_buf(),
                        action: server::state::project::Action::SetAnalyses(
                            state::DataResource::Ok(analyses.clone()),
                        ),
                    })
                    .unwrap();

                let update = Update::project(
                    project_id,
                    base_path,
                    update::Project::Analyses(update::DataResource::Repaired(analyses)),
                    event.id().clone(),
                );

                if self.config.handle_fs_resource_changes() {
                    let mut updates = vec![update];

                    updates
                } else {
                    vec![update]
                }
            }

            (_, Err(IoSerde::Io(io::ErrorKind::NotFound))) => todo!(),
            (_, Err(err)) => {
                self.state
                    .try_reduce(server::state::Action::Project {
                        path: base_path.to_path_buf(),
                        action: server::state::project::Action::SetAnalyses(
                            state::DataResource::Err(err.clone()),
                        ),
                    })
                    .unwrap();

                vec![Update::project(
                    project_id,
                    base_path,
                    update::Project::Analyses(update::DataResource::Corrupted(err)),
                    event.id().clone(),
                )]
            }
        }
    }
}

mod analyses {
    use crate::state;
    use std::path::Path;
    use syre_local::types::AnalysisKind;

    /// Create analysis states from list of analyses by checking if paths
    /// are present in the file system.
    pub fn from_resources(
        analyses: Vec<AnalysisKind>,
        analysis_root: impl AsRef<Path>,
    ) -> Vec<state::Analysis> {
        let analysis_root = analysis_root.as_ref();
        analyses
            .into_iter()
            .map(|analysis| match analysis {
                AnalysisKind::Script(ref script) => {
                    if analysis_root.join(&script.path).is_file() {
                        state::Analysis::present(analysis)
                    } else {
                        state::Analysis::absent(analysis)
                    }
                }
                AnalysisKind::ExcelTemplate(ref template) => {
                    if analysis_root.join(&template.template.path).is_file() {
                        state::Analysis::present(analysis)
                    } else {
                        state::Analysis::absent(analysis)
                    }
                }
            })
            .collect()
    }
}
