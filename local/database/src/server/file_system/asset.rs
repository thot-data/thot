use crate::{common, event as update, server, state, Database, Update};
use std::{assert_matches::assert_matches, path::PathBuf};
use syre_core as core;
use syre_fs_watcher::{event, EventKind};
use syre_local::{self as local, TryReducible};

impl Database {
    pub(super) fn handle_fs_event_asset_file(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::AssetFile(kind) = event.kind() else {
            panic!("invalid event kind");
        };

        match kind {
            syre_fs_watcher::event::ResourceEvent::Created => {
                self.handle_fs_event_asset_file_created(event)
            }
            syre_fs_watcher::event::ResourceEvent::Removed => {
                self.handle_fs_event_asset_file_removed(event)
            }
            syre_fs_watcher::event::ResourceEvent::Renamed => {
                self.handle_fs_event_asset_file_renamed(event)
            }
            syre_fs_watcher::event::ResourceEvent::Moved => todo!(),
            syre_fs_watcher::event::ResourceEvent::MovedProject => todo!(),
            syre_fs_watcher::event::ResourceEvent::Modified(_) => {
                self.handle_fs_event_asset_file_modified(event)
            }
        }
    }
}

impl Database {
    fn handle_fs_event_asset_file_created(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::AssetFile(event::ResourceEvent::Created)
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

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

        let container_path = path.parent().unwrap();
        let container_graph_path = common::container_graph_path(
            project.path().join(&project_properties.data_root),
            container_path,
        )
        .unwrap();

        let container_state = graph.find(&container_graph_path).unwrap().unwrap();
        let container_state = container_state.lock().unwrap();
        let state::DataResource::Ok(assets) = container_state.assets() else {
            return vec![];
        };

        let Some(asset_state) = assets
            .iter()
            .find(|asset| asset.path == path.file_name().unwrap())
        else {
            let path = common::container_graph_path(
                project.path().join(&project_properties.data_root),
                path,
            )
            .unwrap();

            if self.config.handle_fs_resource_changes() {
                // TODO: Set creator for all handled resources.
                let mut assets =
                    local::project::resources::Assets::load_from(&container_path).unwrap();
                let asset_path = path.strip_prefix(&container_graph_path).unwrap();
                assets.push(core::project::Asset::new(asset_path));
                assets.save().unwrap();

                return vec![];
            } else {
                return vec![Update::project_with_id(
                    project_properties.rid().clone(),
                    project.path().clone(),
                    update::Project::AssetFile(update::AssetFile::Created(path)),
                    event.id().clone(),
                )];
            }
        };

        if cfg!(target_os = "windows") {
            tracing::warn!("asset already present")
        } else {
            assert!(!asset_state.is_present());
        }
        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        let asset_id = asset_state.rid().clone();
        drop(container_state);
        self.state
            .try_reduce(server::state::Action::Project {
                path: project_path.clone(),
                action: server::state::project::Action::Container {
                    path: container_graph_path.clone(),
                    action: server::state::project::action::Container::Asset {
                        rid: asset_id.clone(),
                        action: server::state::project::action::Asset::SetPresent,
                    },
                },
            })
            .unwrap();

        vec![Update::project_with_id(
            project_id.clone(),
            project_path.clone(),
            update::Project::Asset {
                container: container_graph_path,
                asset: asset_id,
                update: update::Asset::FileCreated,
            },
            event.id().clone(),
        )]
    }

    fn handle_fs_event_asset_file_removed(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::AssetFile(event::ResourceEvent::Removed)
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
            base_path,
        )
        .unwrap();
        let container_state = graph.find(&container_graph_path).unwrap().unwrap();
        let container_state = container_state.lock().unwrap();
        let state::DataResource::Ok(assets) = container_state.assets() else {
            return vec![];
        };

        let asset_path = path
            .strip_prefix(
                project
                    .path()
                    .join(&project_properties.data_root)
                    .join(&container_graph_path),
            )
            .unwrap();

        let Some(asset_state) = assets.iter().find(|asset| asset.path == asset_path) else {
            let path = common::container_graph_path(
                project.path().join(&project_properties.data_root),
                path,
            )
            .unwrap();

            return vec![Update::project_with_id(
                project_properties.rid().clone(),
                project.path().clone(),
                update::Project::AssetFile(update::AssetFile::Removed(path)),
                event.id().clone(),
            )];
        };

        assert!(asset_state.is_present());
        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        let asset_id = asset_state.rid().clone();
        drop(container_state);
        self.state
            .try_reduce(server::state::Action::Project {
                path: project_path.clone(),
                action: server::state::project::Action::Container {
                    path: container_graph_path.clone(),
                    action: server::state::project::action::Container::Asset {
                        rid: asset_id.clone(),
                        action: server::state::project::action::Asset::SetAbsent,
                    },
                },
            })
            .unwrap();

        if self.config.handle_fs_resource_changes() {
            match local::project::resources::Assets::load_from(&base_path) {
                Ok(mut assets) => {
                    assets.retain(|asset| *asset.rid() != asset_id);
                    match assets.save() {
                        Ok(_) => return vec![],
                        Err(err) => {
                            tracing::error!(?err);
                        }
                    }
                }
                Err(err) => {
                    tracing::error!(?err);
                }
            }
        }

        vec![Update::project_with_id(
            project_id.clone(),
            project_path.clone(),
            update::Project::Asset {
                container: container_graph_path,
                asset: asset_id,
                update: update::Asset::FileRemoved,
            },
            event.id().clone(),
        )]
    }

    fn handle_fs_event_asset_file_renamed(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::AssetFile(event::ResourceEvent::Renamed)
        );

        let [from, to] = &event.paths()[..] else {
            panic!("invalid paths");
        };
        let from_path = PathBuf::from(from.file_name().unwrap());
        let to_path = PathBuf::from(to.file_name().unwrap());

        let project = self.state.find_resource_project_by_path(from).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::FolderResource::Present(graph) = project_state.graph() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let container_path = from.parent().unwrap();
        let container_graph_path = common::container_graph_path(
            project.path().join(&project_properties.data_root),
            container_path,
        )
        .unwrap();

        let container_state = graph.find(&container_graph_path).unwrap().unwrap();
        let container_state = container_state.lock().unwrap();
        let state::DataResource::Ok(assets) = container_state.assets() else {
            panic!("invalid state");
        };

        let mut asset_state = assets
            .iter()
            .find(|asset| asset.path == from_path)
            .unwrap()
            .clone();

        asset_state.properties.path = to_path.clone();
        if asset_state.path.is_file() {
            asset_state.fs_resource = state::FileResource::Present;
        } else {
            asset_state.fs_resource = state::FileResource::Absent;
        }

        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        let asset_id = asset_state.rid().clone();
        drop(container_state);
        self.state
            .try_reduce(server::state::Action::Project {
                path: project_path.clone(),
                action: server::state::project::Action::Container {
                    path: container_graph_path.clone(),
                    action: server::state::project::action::Container::Asset {
                        rid: asset_id.clone(),
                        action: server::state::project::action::Asset::SetState(
                            asset_state.clone(),
                        ),
                    },
                },
            })
            .unwrap();

        if self.config.handle_fs_resource_changes() {
            match local::project::resources::Assets::load_from(&container_path) {
                Ok(mut assets) => {
                    let asset = assets
                        .iter_mut()
                        .find(|asset| *asset.rid() == asset_id)
                        .unwrap();
                    asset.path = to_path;

                    match assets.save() {
                        Ok(_) => return vec![],
                        Err(err) => {
                            tracing::error!(?err);
                            todo!();
                        }
                    }
                }
                Err(err) => {
                    tracing::error!(?err);
                    todo!();
                }
            }
        } else {
            vec![Update::project_with_id(
                project_id.clone(),
                project_path.clone(),
                update::Project::Asset {
                    container: container_graph_path,
                    asset: asset_id,
                    update: update::Asset::Properties(asset_state),
                },
                event.id().clone(),
            )]
        }
    }

    fn handle_fs_event_asset_file_modified(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        use event::ModifiedKind;

        let EventKind::AssetFile(event::ResourceEvent::Modified(kind)) = event.kind() else {
            panic!("invalid event kind");
        };

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        match kind {
            event::ModifiedKind::Other => vec![],
            event::ModifiedKind::Data => todo!(),
        }
    }
}
