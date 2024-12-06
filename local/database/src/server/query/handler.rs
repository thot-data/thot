use crate::{common, constants, error, query, server, state, Database};
use serde_json::Value as JsValue;
use std::path::{Path, PathBuf};
use syre_core::{db::SearchFilter, system::User, types::ResourceId};
use syre_local as local;

impl Database {
    pub fn handle_query_config(&self, query: query::Config) -> JsValue {
        match query {
            query::Config::Id => constants::DATABASE_ID.into(),
        }
    }
}

impl Database {
    pub fn handle_query_state(&self, query: query::State) -> JsValue {
        match query {
            query::State::UserManifest => {
                serde_json::to_value(self.state.app().user_manifest()).unwrap()
            }
            query::State::ProjectManifest => {
                serde_json::to_value(self.state.app().project_manifest()).unwrap()
            }
            query::State::LocalConfig => {
                serde_json::to_value(self.state.app().local_config()).unwrap()
            }
            query::State::Projects => {
                let states = self.handle_query_state_projects();
                serde_json::to_value(states).unwrap()
            }
            query::State::Graph(base_path) => {
                let state = self.handle_query_state_graph(base_path);
                serde_json::to_value(state).unwrap()
            }
            query::State::Asset {
                project,
                container,
                asset,
            } => {
                let state = self.handle_query_state_asset(project, container, asset);
                serde_json::to_value(state).unwrap()
            }
        }
    }

    fn handle_query_state_projects(&self) -> Vec<state::Project> {
        self.state
            .projects()
            .iter()
            .map(|project| {
                let data = project.fs_resource().map(|data| data.project_data());
                state::Project {
                    path: project.path().clone(),
                    fs_resource: data,
                }
            })
            .collect()
    }

    fn handle_query_state_graph(&self, base_path: PathBuf) -> Option<state::Graph> {
        let Some(project) = self.state.find_project_by_path(&base_path) else {
            return None;
        };

        let state::FolderResource::Present(project) = project.fs_resource() else {
            return None;
        };

        let state::FolderResource::Present(graph) = project.graph() else {
            return None;
        };

        Some(graph.as_graph())
    }

    // NB: Asset is copied for code cleanliness.
    //      If this becomes a performance issue, this can be changed.
    /// # Arguments
    /// 1. `project`: Path to the project's base folder.
    /// 2. `container`: Absolute path to the container from the graph root.
    /// 3. `asset`: Relative path to the asset file from the container.
    fn handle_query_state_asset(
        &self,
        project: PathBuf,
        container: PathBuf,
        asset: PathBuf,
    ) -> Option<state::Asset> {
        let Some(project) = self.state.find_project_by_path(&project) else {
            return None;
        };

        let state::FolderResource::Present(project) = project.fs_resource() else {
            return None;
        };

        let state::FolderResource::Present(graph) = project.graph() else {
            return None;
        };

        let Some(container) = graph.find(container).unwrap() else {
            return None;
        };

        let container = container.lock().unwrap();
        let state::DataResource::Ok(ref assets) = container.assets else {
            return None;
        };

        assets.iter().find(|state| state.path == asset).cloned()
    }
}

impl Database {
    pub fn handle_query_user(&self, query: query::User) -> JsValue {
        match query {
            query::User::Info(id) => {
                let state::ManifestState::Ok(ref manifest) = self.state.app().user_manifest()
                else {
                    return serde_json::to_value(Option::<User>::None).unwrap();
                };

                let user = manifest.iter().find(|user| user.rid() == &id);
                serde_json::to_value(user).unwrap()
            }
            query::User::Projects(user) => serde_json::to_value(self.user_projects(&user)).unwrap(),
        }
    }

    fn user_projects(&self, user: &ResourceId) -> Vec<(PathBuf, state::ProjectData)> {
        self.state
            .projects()
            .iter()
            .filter_map(|project| {
                let state::FolderResource::Present(project_state) = project.fs_resource() else {
                    return None;
                };

                let state::DataResource::Ok(settings) = project_state.settings() else {
                    return None;
                };

                let Some(permissions) =
                    settings.permissions.iter().find_map(|(uid, permissions)| {
                        if uid == user {
                            Some(permissions)
                        } else {
                            None
                        }
                    })
                else {
                    return None;
                };

                if permissions.any() {
                    Some((project.path().clone(), project_state.project_data()))
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Database {
    pub fn handle_query_project(&self, query: query::Project) -> JsValue {
        match query {
            query::Project::Get(project) => {
                let state = self.handle_query_project_get(&project);
                serde_json::to_value(state).unwrap()
            }
            query::Project::GetById(project) => {
                let state = self.handle_query_project_get_by_id(&project);
                serde_json::to_value(state).unwrap()
            }
            query::Project::Path(project) => {
                let state = self.handle_query_project_path(&project);
                serde_json::to_value(state).unwrap()
            }
            query::Project::GetMany(projects) => {
                let states = self.handle_query_project_get_many(&projects);
                serde_json::to_value(states).unwrap()
            }
            query::Project::Resources(project) => {
                let state = self.handle_query_state_project_resources(&project);
                serde_json::to_value(state).unwrap()
            }
        }
    }

    fn handle_query_project_get(&self, project: impl AsRef<Path>) -> Option<state::Project> {
        self.state.projects().iter().find_map(|state| {
            if state.path() == project.as_ref() {
                Some(state::Project {
                    path: state.path().clone(),
                    fs_resource: state.fs_resource().map(|data| data.project_data()),
                })
            } else {
                None
            }
        })
    }

    /// # Returns
    /// Project with the given id.
    /// `None` if a state is not associated with the project.
    fn handle_query_project_get_by_id(
        &self,
        project: &ResourceId,
    ) -> Option<(PathBuf, state::ProjectData)> {
        self.state.projects().iter().find_map(|state| {
            let state::FolderResource::Present(project_state) = state.fs_resource() else {
                return None;
            };

            let state::DataResource::Ok(properties) = project_state.properties() else {
                return None;
            };

            if properties.rid() == project {
                Some((state.path().clone(), project_state.project_data()))
            } else {
                None
            }
        })
    }

    /// # Returns
    /// Project's path.
    /// `None` if a state is not associated with the project.
    fn handle_query_project_path(&self, project: &ResourceId) -> Option<PathBuf> {
        self.state.projects().iter().find_map(|state| {
            let state::FolderResource::Present(project_state) = state.fs_resource() else {
                return None;
            };

            let state::DataResource::Ok(properties) = project_state.properties() else {
                return None;
            };

            if properties.rid() == project {
                Some(state.path().clone())
            } else {
                None
            }
        })
    }

    /// # Returns
    /// State of the projects at the given paths.
    /// Paths without an associated state are ommitted from the result.
    fn handle_query_project_get_many(&self, projects: &Vec<PathBuf>) -> Vec<state::Project> {
        projects
            .iter()
            .filter_map(|project| {
                self.state.projects().iter().find_map(|state| {
                    if state.path() == project {
                        Some(state::Project {
                            path: state.path().clone(),
                            fs_resource: state.fs_resource().map(|data| data.project_data()),
                        })
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    /// # Returns
    /// Project's path, data, and graph.
    /// `None` if a state is not associated with the project.
    fn handle_query_state_project_resources(
        &self,
        project: &ResourceId,
    ) -> Option<(
        PathBuf,
        state::ProjectData,
        state::FolderResource<state::Graph>,
    )> {
        self.state.projects().iter().find_map(|state| {
            let state::FolderResource::Present(project_data) = state.fs_resource() else {
                return None;
            };

            let state::DataResource::Ok(properties) = project_data.properties() else {
                return None;
            };

            if properties.rid() == project {
                Some((
                    state.path().clone(),
                    project_data.project_data(),
                    project_data.graph().map(|graph| graph.as_graph()),
                ))
            } else {
                None
            }
        })
    }
}

impl Database {
    pub fn handle_query_container(&self, query: query::Container) -> JsValue {
        match query {
            query::Container::Get { project, container } => {
                let state = self
                    .handle_query_container_get(&project, container)
                    .map(|state| {
                        state.map(|state| {
                            let container: std::sync::MutexGuard<'_, state::Container> =
                                state.lock().unwrap();
                            (*container).clone()
                        })
                    });

                serde_json::to_value(state).unwrap()
            }

            query::Container::GetById { project, container } => {
                let state = self
                    .handle_query_container_get_by_id(&project, &container)
                    .map(|state| {
                        let container = state.lock().unwrap();
                        (*container).clone()
                    });

                serde_json::to_value(state).unwrap()
            }

            query::Container::GetForAnalysis { project, container } => {
                let state = self.handle_query_container_get_for_analysis(&project, &container);
                serde_json::to_value(state).unwrap()
            }

            query::Container::GetByIdForAnalysis { project, container } => {
                let state =
                    self.handle_query_container_get_by_id_for_analysis(&project, &container);
                serde_json::to_value(state).unwrap()
            }

            query::Container::Search {
                project,
                root,
                query,
            } => {
                let results = self.handle_query_container_search(&project, &root, &query);
                serde_json::to_value(results).unwrap()
            }
        }
    }

    /// # Arguments
    /// 1. `project`: Path to the project's base folder.
    /// 2. `container`: Absolute path to the container from the graph root.
    /// The root container has the root path.
    fn handle_query_container_get(
        &self,
        project: &ResourceId,
        container: PathBuf,
    ) -> Result<Option<&server::state::project::graph::Node>, error::InvalidPath> {
        let Some(project) = self.state.find_project_by_id(&project) else {
            return Ok(None);
        };

        let state::FolderResource::Present(project) = project.fs_resource() else {
            return Ok(None);
        };

        let state::FolderResource::Present(graph) = project.graph() else {
            return Ok(None);
        };

        graph.find(container)
    }

    fn handle_query_container_get_by_id(
        &self,
        project: &ResourceId,
        container: &ResourceId,
    ) -> Option<&server::state::project::graph::Node> {
        let Some(project) = self.state.find_project_by_id(&project) else {
            return None;
        };

        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            return None;
        };

        let state::FolderResource::Present(graph) = project_state.graph() else {
            return None;
        };

        graph.nodes().iter().find(|node| {
            let container_state = node.lock().unwrap();
            let state::DataResource::Ok(rid) = container_state.rid() else {
                return false;
            };

            rid == container
        })
    }

    /// # Arguments
    /// 1. `project`: Project id.
    /// 2. `container`: Absolute path to the container from the graph root.
    /// The root container has the root path.
    ///
    /// # Returns
    /// Container shaped for use in an analysis script with folded metadata.
    /// Error is a tuple of (container, ancestors) where container is the state of the container,
    /// and ancestors is a list of ancestor property states.
    fn handle_query_container_get_for_analysis(
        &self,
        project: &ResourceId,
        container: impl AsRef<Path>,
    ) -> Result<
        Option<Result<ContainerForAnalysis, Vec<Option<local::error::IoSerde>>>>,
        error::InvalidPath,
    > {
        let Some(project) = self.state.find_project_by_id(&project) else {
            return Ok(None);
        };

        let state::FolderResource::Present(project) = project.fs_resource() else {
            return Ok(None);
        };

        let state::FolderResource::Present(graph) = project.graph() else {
            return Ok(None);
        };

        graph.find(container).map(|container| {
            container.map(|container| {
                self.container_for_analysis(&graph.ancestors(container))
                    .unwrap()
            })
        })
    }

    /// # Returns
    /// Container shaped for use in an analysis script with folded metadata.
    /// Error is a tuple of (container, ancestors) where container is the state of the container,
    /// and ancestors is a list of ancestor property states.
    fn handle_query_container_get_by_id_for_analysis(
        &self,
        project: &ResourceId,
        container: &ResourceId,
    ) -> Option<Result<ContainerForAnalysis, Vec<Option<local::error::IoSerde>>>> {
        let Some(project) = self.state.find_project_by_id(&project) else {
            return None;
        };

        let state::FolderResource::Present(project) = project.fs_resource() else {
            return None;
        };

        let state::FolderResource::Present(graph) = project.graph() else {
            return None;
        };

        graph.find_by_id(container).map(|container| {
            self.container_for_analysis(&graph.ancestors(container))
                .unwrap()
        })
    }

    /// # Returns
    /// Containers shaped for use in an analysis script with folded metadata.
    /// Error is a tuple of (container, ancestors) where container is the state of the container,
    /// and ancestors is a list of ancestor property states.
    fn handle_query_container_search(
        &self,
        project: &ResourceId,
        root: impl AsRef<Path>,
        query: &crate::query::ContainerQuery,
    ) -> Result<Vec<ContainerForAnalysis>, crate::query::error::Search> {
        let Some(project) = self.state.find_project_by_id(&project) else {
            return Err(crate::query::error::Search::ProjectDoesNotExist);
        };

        let state::FolderResource::Present(project) = project.fs_resource() else {
            return Err(crate::query::error::Search::ProjectDoesNotExist);
        };

        let state::FolderResource::Present(graph) = project.graph() else {
            return Err(crate::query::error::Search::RootDoesNotExist);
        };

        let Ok(root) = graph.find(root.as_ref()) else {
            return Err(crate::query::error::Search::InvalidPath);
        };
        let Some(root) = root else {
            return Err(crate::query::error::Search::RootDoesNotExist);
        };

        let descendants = graph.descendants(&root).unwrap();
        assert!(descendants.len() > 0);
        let matches = descendants
            .iter()
            .filter_map(|descendant| {
                let container = descendant.lock().unwrap();
                let matches = query.matches(&*container);
                drop(container);

                if matches {
                    let container = self
                        .container_for_analysis(&graph.ancestors(&descendant))
                        .unwrap();
                    Some(container)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if matches.iter().all(|state| state.is_ok()) {
            let matches = matches.into_iter().map(|state| state.unwrap()).collect();
            Ok(matches)
        } else {
            let mut errors = matches
                .into_iter()
                .enumerate()
                .filter_map(|(idx_root, state)| {
                    if let Err(errors) = state {
                        let node = &descendants[idx_root];
                        let errors = errors
                            .into_iter()
                            .enumerate()
                            .filter_map(|(idx_err, err)| {
                                if let Some(err) = err {
                                    let mut path = graph.path(&node).unwrap();
                                    (0..idx_err).for_each(|_| {
                                        // get path of node where error occurred
                                        path.pop();
                                    });
                                    Some((path, err))
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();

                        Some(errors)
                    } else {
                        None
                    }
                })
                .flatten()
                .map(|err| err.into())
                .collect::<Vec<query::error::CorruptState>>();

            errors.sort_by_key(|err| err.path.clone());
            errors.dedup();
            Err(crate::query::error::Search::Inheritance(errors))
        }
    }

    /// # Returns
    /// Container shaped for use in an analysis script with inherited metadata.
    /// `None` if `ancestors` is empty.
    /// Error is a list of error states for the ancestor at the corresponding index.
    fn container_for_analysis(
        &self,
        ancestors: &Vec<server::state::project::graph::Node>,
    ) -> Option<Result<ContainerForAnalysis, Vec<Option<local::error::IoSerde>>>> {
        if ancestors.is_empty() {
            return None;
        }

        let nodes = ancestors
            .iter()
            .map(|node| node.lock().unwrap())
            .collect::<Vec<_>>();

        let properties = nodes
            .iter()
            .map(|node| node.properties())
            .collect::<Vec<_>>();

        if properties.iter().any(|state| state.is_err()) {
            return Some(Err(properties
                .into_iter()
                .map(|state| state.err().clone())
                .collect()));
        }

        let metadata = properties
            .iter()
            .rev()
            .map(|state| state.as_ref().unwrap().metadata.clone())
            .reduce(|mut metadata, data| {
                metadata.extend(data);
                metadata
            })
            .unwrap();

        let container = &nodes[0];
        let mut properties = container.properties().unwrap().clone();
        properties.metadata = metadata;

        let assets = container
            .assets()
            .unwrap()
            .iter()
            .map(|asset| asset.properties.clone())
            .collect();

        Some(Ok(ContainerForAnalysis {
            rid: container.rid().unwrap().clone(),
            properties,
            assets,
        }))
    }
}

impl Database {
    pub fn handle_query_asset(&self, query: query::Asset) -> JsValue {
        match query {
            query::Asset::Search {
                project,
                root,
                query,
            } => {
                let results = self.handle_query_asset_search(&project, &root, &query);
                serde_json::to_value(results).unwrap()
            }
        }
    }

    /// # Returns
    /// Containers shaped for use in an analysis script with folded metadata.
    /// Error is a tuple of (assets, ancestors) where `assets`` is the state of the container's assets,
    /// and ancestors is a list of ancestor property states.
    fn handle_query_asset_search(
        &self,
        project: &ResourceId,
        root: impl AsRef<Path>,
        query: &crate::query::AssetQuery,
    ) -> Result<Vec<AssetForAnalysis>, crate::query::error::Search> {
        let Some(project) = self.state.find_project_by_id(&project) else {
            return Err(crate::query::error::Search::ProjectDoesNotExist);
        };

        let state::FolderResource::Present(project_data) = project.fs_resource() else {
            return Err(crate::query::error::Search::ProjectDoesNotExist);
        };

        let project_properties = match project_data.properties() {
            state::DataResource::Ok(properties) => properties,
            state::DataResource::Err(err) => {
                return Err(crate::query::error::Search::ProjectProperties(err))
            }
        };

        let state::FolderResource::Present(graph) = project_data.graph() else {
            return Err(crate::query::error::Search::RootDoesNotExist);
        };

        let Ok(root) = graph.find(root.as_ref()) else {
            return Err(crate::query::error::Search::InvalidPath);
        };
        let Some(root) = root else {
            return Err(crate::query::error::Search::RootDoesNotExist);
        };

        let data_root = project.path().join(&project_properties.data_root);
        let descendants = graph.descendants(&root).unwrap();
        assert!(descendants.len() > 0);
        let matches = descendants
            .iter()
            .filter_map(|descendant| {
                let container = descendant.lock().unwrap();
                let Ok(assets) = container.assets() else {
                    return None;
                };
                let matches = assets
                    .iter()
                    .filter_map(|asset| {
                        if query.matches(asset) {
                            Some(asset.properties.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                drop(container);

                let ancestors = graph.ancestors(descendant);
                assert!(ancestors.len() > 0);
                let container_path = graph.path(descendant).unwrap();
                let path = common::container_system_path(&data_root, container_path);
                Some(
                    matches
                        .into_iter()
                        .map(|asset| self.asset_for_analysis(asset, &ancestors, &path))
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();

        if matches.iter().flatten().all(|state| state.is_ok()) {
            let matches = matches
                .into_iter()
                .flatten()
                .map(|state| state.unwrap())
                .collect();

            Ok(matches)
        } else {
            let mut errors = matches
                .into_iter()
                .enumerate()
                .map(|(idx_root, states)| {
                    let node = &descendants[idx_root];
                    let path = graph.path(&node).unwrap();

                    states
                        .into_iter()
                        .filter_map(|state| {
                            if let Err(errors) = state {
                                let errors = errors
                                    .into_iter()
                                    .enumerate()
                                    .filter_map(|(idx_err, err)| {
                                        if let Some(err) = err {
                                            let mut path = path.clone();
                                            (0..idx_err).for_each(|_| {
                                                // get path of node where error occurred
                                                path.pop();
                                            });
                                            Some((path, err))
                                        } else {
                                            None
                                        }
                                    })
                                    .collect::<Vec<_>>();

                                Some(errors)
                            } else {
                                None
                            }
                        })
                        .flatten()
                        .collect::<Vec<_>>()
                })
                .flatten()
                .map(|err| err.into())
                .collect::<Vec<query::error::CorruptState>>();

            errors.sort_by_key(|err| err.path.clone());
            errors.dedup();
            Err(crate::query::error::Search::Inheritance(errors))
        }
    }

    /// # Returns
    /// Asset shaped for use in an analysis script with inherited metadata.
    /// Error is a list of the errors that occured for each ancestor in the corresponding index.
    fn asset_for_analysis(
        &self,
        mut asset: syre_core::project::Asset,
        ancestors: &Vec<server::state::project::graph::Node>,
        parent_path: impl AsRef<Path>,
    ) -> Result<AssetForAnalysis, Vec<Option<local::error::IoSerde>>> {
        assert!(ancestors.len() > 0);

        let nodes = ancestors
            .iter()
            .map(|node| node.lock().unwrap())
            .collect::<Vec<_>>();

        let properties = nodes
            .iter()
            .map(|node| node.properties())
            .collect::<Vec<_>>();

        if properties.iter().any(|state| state.is_err()) {
            return Err(properties
                .into_iter()
                .map(|state| state.err().clone())
                .collect());
        }

        let metadata = properties
            .iter()
            .rev()
            .map(|state| state.as_ref().unwrap().metadata.clone())
            .chain(std::iter::once(asset.properties.metadata.clone()))
            .reduce(|mut metadata, data| {
                metadata.extend(data);
                metadata
            })
            .unwrap();
        asset.properties.metadata = metadata;

        Ok(AssetForAnalysis {
            rid: asset.rid().clone(),
            properties: asset.properties,
            path: parent_path.as_ref().join(&asset.path),
        })
    }
}
#[derive(serde::Serialize, Debug)]
struct ContainerForAnalysis {
    rid: ResourceId,
    properties: syre_core::project::ContainerProperties,
    assets: Vec<syre_core::project::Asset>,
}

#[derive(serde::Serialize, Debug)]
struct AssetForAnalysis {
    rid: ResourceId,
    properties: syre_core::project::AssetProperties,
    path: PathBuf,
}
