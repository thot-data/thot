use crate::{
    common,
    event::{self as update, Update},
    server::{self, state::project::graph},
    state, Database,
};
use std::{
    assert_matches::assert_matches,
    io,
    path::{Path, PathBuf},
};
use syre_core::{self as core, types::ResourceId};
use syre_fs_watcher::{event, EventKind};
use syre_local::{self as local, TryReducible};

impl Database {
    pub(super) fn handle_fs_event_graph(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        let EventKind::Graph(kind) = event.kind() else {
            panic!("invalid event kind");
        };

        match kind {
            event::Graph::Created => self.handle_fs_event_graph_created(event),
            event::Graph::Removed => todo!(),
            event::Graph::Moved => self.handle_fs_event_graph_moved(event),
            event::Graph::Modified(_) => todo!(),
        }
    }

    pub(super) fn handle_fs_event_graph_resource(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::GraphResource(kind) = event.kind() else {
            panic!("invalid event kind");
        };

        match kind {
            event::GraphResource::Removed => self.handle_fs_event_graph_resource_removed(event),
        }
    }
}

impl Database {
    #[cfg(target_os = "windows")]
    fn handle_fs_event_graph_created(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        use std::fs;

        use local::{loader::container, project::resources};

        assert_matches!(event.kind(), EventKind::Graph(event::Graph::Created));
        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        assert!(project_state.graph().is_present());
        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let data_root_path = project.path().join(&project_properties.data_root);
        let parent_path =
            common::container_graph_path(&data_root_path, path.parent().unwrap()).unwrap();

        if self.config.handle_fs_resource_changes() {
            if let Err(errors) = graph_reassign_ids(path) {
                let create_containers = errors.into_iter().map(|(path, err)| {
                    match (&err.properties, &err.assets) {
                        (Some(error::LoadSave::Load(local::error::IoSerde::Io(io::ErrorKind::NotFound))), Some(error::LoadSave::Load(local::error::IoSerde::Io(io::ErrorKind::NotFound)))) => {
                            if local::common::app_dir_of(&path).exists() {
                                tracing::warn!("{path:?}: `.syre` folder exists without `container.json` and `assets.json` files");
                                Err((path, error::GraphCreate::Reassign(err)))
                            } else {
                                let mut container = resources::Container::new(&path);
                                let assets = fs::read_dir(&path).map(|entries| entries.filter_map(|entry| {
                                    entry.ok()
                                }).filter(|entry| {
                                    entry.file_type().map(|kind| kind.is_file()).unwrap_or(false)
                                })
                                .map(|entry| {
                                    let asset_path = entry.path();
                                    let asset_path = asset_path.strip_prefix(&path).unwrap();
                                    core::project::Asset::new(asset_path)
                                })
                                .collect::<Vec<_>>());

                                if let Ok(assets) = assets {
                                container.assets.extend(assets);
                                } else {
                                    todo!("{assets:?}");
                                }

                                container.save().map_err(|err| (path, error::GraphCreate::CreateContainer(err)))
                            }
                        }

                        _=> Err((path, error::GraphCreate::Reassign(err))),
                    }
                }).collect::<Vec<_>>();

                let errors = create_containers
                    .into_iter()
                    .filter_map(|result| result.err())
                    .collect::<Vec<_>>();
                if !errors.is_empty() {
                    todo!("{errors:?}");
                }
            }

            let mut root = match container::Loader::load_from_only_properties(&path) {
                Ok(root) => root,
                Err(err) => todo!("{err:?}"),
            };

            let dir_name = path.file_name().unwrap().to_string_lossy();
            if root.properties.name != dir_name {
                root.properties.name = dir_name.to_string();
                if let Err(err) = root.save(path) {
                    tracing::error!("{err:?}");
                }
            }
        }

        let subgraph = graph::State::load(path).unwrap();
        let subgraph_state = subgraph.as_graph();

        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();

        let state::FolderResource::Present(graph) = project_state.graph() else {
            unreachable!();
        };

        let root = subgraph.root().lock().unwrap();
        let root_path = parent_path.join(root.name());
        drop(root);
        if graph.find(&root_path).unwrap().is_some() {
            tracing::trace!("{root_path:?} already exists");
            self.state
                .try_reduce(server::state::Action::Project {
                    path: project_path.clone(),
                    action: server::state::project::action::Graph::Remove(root_path).into(),
                })
                .unwrap();

            self.state
                .try_reduce(server::state::Action::Project {
                    path: project_path.clone(),
                    action: server::state::project::action::Graph::Insert {
                        parent: parent_path.clone(),
                        graph: subgraph,
                    }
                    .into(),
                })
                .unwrap();
        } else {
            self.state
                .try_reduce(server::state::Action::Project {
                    path: project_path.clone(),
                    action: server::state::project::action::Graph::Insert {
                        parent: parent_path.clone(),
                        graph: subgraph,
                    }
                    .into(),
                })
                .unwrap();
        }

        vec![Update::project_with_id(
            project_id,
            project_path,
            update::Graph::Inserted {
                parent: parent_path,
                graph: subgraph_state,
            }
            .into(),
            event.id().clone(),
        )]
    }

    #[cfg(not(target_os = "windows"))]
    fn handle_fs_event_graph_created(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        assert_matches!(event.kind(), EventKind::Graph(event::Graph::Created));
        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        assert!(project_state.graph().is_present());
        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let data_root_path = project.path().join(&project_properties.data_root);
        let parent_path =
            common::container_graph_path(&data_root_path, path.parent().unwrap()).unwrap();
        let subgraph = graph::State::load(path).unwrap();
        let subgraph_state = subgraph.as_graph();

        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();

        self.state
            .try_reduce(server::state::Action::Project {
                path: project_path.clone(),
                action: server::state::project::action::Graph::Insert {
                    parent: parent_path.clone(),
                    graph: subgraph,
                }
                .into(),
            })
            .unwrap();

        vec![Update::project_with_id(
            project_id,
            project_path,
            update::Graph::Inserted {
                parent: parent_path,
                graph: subgraph_state,
            }
            .into(),
            event.id().clone(),
        )]
    }

    fn handle_fs_event_graph_moved(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        let EventKind::Graph(event::Graph::Moved) = event.kind() else {
            panic!("invalid event kind");
        };

        let [from, to] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let project = self.state.find_resource_project_by_path(from).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        assert!(project_state.graph().is_present());
        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let data_root_path = project.path().join(&project_properties.data_root);
        let from_path = common::container_graph_path(&data_root_path, from).unwrap();
        let to_path = common::container_graph_path(&data_root_path, to).unwrap();

        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        self.state
            .try_reduce(server::state::Action::Project {
                path: project_path.clone(),
                action: server::state::project::action::Graph::Move {
                    from: from_path.clone(),
                    to: to_path.clone(),
                }
                .into(),
            })
            .unwrap();

        vec![Update::project_with_id(
            project_id.clone(),
            project_path.clone(),
            update::Graph::Moved {
                from: from_path,
                to: to_path,
            }
            .into(),
            event.id().clone(),
        )]
    }
}

impl Database {
    fn handle_fs_event_graph_resource_removed(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::GraphResource(event::GraphResource::Removed)
        );
        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(project_properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let state::FolderResource::Present(graph) = project_state.graph() else {
            panic!("invalid state");
        };

        let data_root_path = project.path().join(&project_properties.data_root);
        let graph_path = common::container_graph_path(&data_root_path, path).unwrap();
        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();

        if let Some(_container) = graph.find(&graph_path).unwrap() {
            self.state
                .try_reduce(server::state::Action::Project {
                    path: project_path.clone(),
                    action: server::state::project::action::Graph::Remove(graph_path.clone())
                        .into(),
                })
                .unwrap();

            return vec![Update::project_with_id(
                project_id.clone(),
                project_path.clone(),
                update::Graph::Removed(graph_path).into(),
                event.id().clone(),
            )];
        }

        // TODO: When using buckets, must get the nearest container.
        // At the time this is written, buckets are not yet implemented though.
        let parent_container_path = path.parent().unwrap();
        let parent_container_graph_path =
            common::container_graph_path(&data_root_path, parent_container_path).unwrap();
        let rel_path = graph_path
            .strip_prefix(&parent_container_graph_path)
            .unwrap();

        let parent_node = graph.find(&parent_container_graph_path).unwrap().unwrap();
        let parent_state = parent_node.lock().unwrap();
        if let state::DataResource::Ok(assets) = parent_state.assets().clone() {
            if let Some(asset) = assets.iter().find(|asset| asset.path == rel_path) {
                let asset = asset.rid().clone();
                drop(parent_state);

                self.state
                    .try_reduce(server::state::Action::Project {
                        path: project_path.clone(),
                        action: server::state::project::action::Action::Container {
                            path: parent_container_graph_path.clone(),
                            action: server::state::project::action::Container::Asset {
                                rid: asset.clone(),
                                action: server::state::project::action::Asset::SetAbsent,
                            },
                        },
                    })
                    .unwrap();

                if self.config.handle_fs_resource_changes() {
                    tracing::debug!(?parent_container_path);
                    let mut local_assets =
                        local::project::resources::Assets::load_from(parent_container_path)
                            .unwrap();
                    local_assets.retain(|local_asset| *local_asset.rid() != asset);
                    local_assets.save().unwrap();
                    return vec![];
                } else {
                    return vec![Update::project_with_id(
                        project_id.clone(),
                        project_path.clone(),
                        update::Project::Asset {
                            container: parent_container_graph_path,
                            asset: asset.clone(),
                            update: update::Asset::FileRemoved,
                        },
                        event.id().clone(),
                    )];
                }
            }
        }

        // TODO: It could be that a file unassociated with a graph resource was
        // removed.
        // e.g. A file in a container folder that is not registered as an Asset.
        // Need to decide what to do in this case.
        panic!("expected a graph resource");
    }
}

/// Create new ids for the container and its assets.
fn graph_reassign_ids(
    path: impl AsRef<Path>,
) -> Result<(), Vec<(PathBuf, error::ContainerPropertiesAssets)>> {
    let walker = local::common::ignore::WalkBuilder::new(path)
        .filter_entry(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .build();

    let results = walker
        .into_iter()
        .filter_map(|entry| entry.ok())
        .map(|entry| {
            container_reassign_ids(entry.path()).map_err(|err| (entry.path().to_path_buf(), err))
        })
        .collect::<Vec<_>>();

    let errors = results
        .into_iter()
        .filter_map(|result| result.err())
        .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn container_reassign_ids(path: impl AsRef<Path>) -> Result<(), error::ContainerPropertiesAssets> {
    use local::loader::container;

    let properties = container::Loader::load_from_only_properties(&path)
        .map(|mut properties| {
            properties.rid = ResourceId::new();
            properties
                .save(&path)
                .map_err(|err| error::LoadSave::Save(err))
        })
        .map_err(|err| error::LoadSave::Load(err))
        .flatten();

    let assets = container::Loader::load_from_only_assets(&path)
        .map(|mut assets| {
            assets.iter_mut().for_each(|asset| {
                *asset = core::project::Asset::with_properties(
                    asset.path.clone(),
                    asset.properties.clone(),
                );
            });
            assets.save(&path).map_err(|err| error::LoadSave::Save(err))
        })
        .map_err(|err| error::LoadSave::Load(err))
        .flatten();

    if properties.is_err() || assets.is_err() {
        Err(error::ContainerPropertiesAssets {
            properties: properties.err(),
            assets: assets.err(),
        })
    } else {
        Ok(())
    }
}

mod error {
    use std::io;
    use syre_local::{self as local, project::resources};

    /// Error when handling `Graph::Create` events.
    #[derive(Debug)]
    pub enum GraphCreate {
        /// Error when reassigning a container's ids.
        Reassign(ContainerPropertiesAssets),

        /// Error when creating a new container.
        CreateContainer(resources::container::error::Save),
    }

    /// Error when updating a container's properties or assets file.
    #[derive(Debug)]
    pub struct ContainerPropertiesAssets {
        pub properties: Option<LoadSave>,
        pub assets: Option<LoadSave>,
    }

    #[derive(Debug)]
    pub enum LoadSave {
        Load(local::error::IoSerde),
        Save(io::Error),
    }
}
