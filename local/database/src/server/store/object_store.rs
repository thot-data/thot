//! Database for storing resources.
use super::super::types::ProjectResources;
use crate::error::server::UpdateSubgraphPath as UpdateSubgraphPathError;
use crate::Result;
use has_id::HasId;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;
use std::{fs, io};
use syre_core::db::{SearchFilter, StandardSearchFilter as StdFilter};
use syre_core::error::{Error as CoreError, Resource as ResourceError};
use syre_core::graph::ResourceTree;
use syre_core::project::{Asset, Container as CoreContainer, ExcelTemplate, Metadata, Script};
use syre_core::types::{ResourceId, ResourceMap};
use syre_local::project::resources::{Analyses, Container as LocalContainer, Project};
use syre_local::types::analysis::AnalysisKind;

// *************
// *** Types ***
// *************

// TODO[l]: Types don't need to be `pub`.
#[derive(Debug)]
pub struct PathMap<T>(HashMap<PathBuf, T>);
impl<T> PathMap<T> {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get(&self, key: &Path) -> Option<&T> {
        self.0.get(key)
    }

    /// Gets an item.
    ///
    /// # Notes
    /// + Canonicalizes the `key` path.
    pub fn get_canonical(&self, key: &Path) -> StdResult<Option<&T>, io::Error> {
        let key = fs::canonicalize(&key)?;
        Ok(self.0.get(&key))
    }

    /// Gets an item.
    ///
    /// # Notes
    /// + Canonicalizes the `key` path.
    pub fn get_canonical_mut(&mut self, key: &Path) -> Option<&mut T> {
        let key = fs::canonicalize(key).unwrap();
        self.0.get_mut(&key)
    }

    /// Inserts an item.
    ///
    /// # Notes
    /// + Canonicalizes the `key` path.
    pub fn insert(&mut self, key: PathBuf, value: T) -> Option<T> {
        self.0.insert(key, value)
    }

    /// Inserts an item.
    ///
    /// # Notes
    /// + Canonicalizes the `key` path.
    pub fn insert_canonical(&mut self, key: PathBuf, value: T) -> StdResult<Option<T>, io::Error> {
        let key = fs::canonicalize(&key)?;
        Ok(self.0.insert(key, value))
    }

    pub fn remove(&mut self, key: &Path) -> Option<T> {
        self.0.remove(key)
    }

    /// Removes an item.
    ///
    /// # Notes
    /// + Canonicalizes the `key` path.
    pub fn remove_canonical(&mut self, key: &Path) -> StdResult<Option<T>, io::Error> {
        let key = fs::canonicalize(key)?;
        Ok(self.0.remove(&key))
    }
}

impl<T> Deref for PathMap<T> {
    type Target = HashMap<PathBuf, T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type ContainerTree = ResourceTree<LocalContainer>;

pub type IdMap = HashMap<ResourceId, ResourceId>;

/// Map of [`ResourceId`] to [`Project`].
pub type ProjectMap = ResourceMap<Project>;

/// Map from [`Project`]s to their [`Script`]s.
pub type AnalysesMap = HashMap<ResourceId, Analyses>;

/// Map from a `Project`'s id to its [`ContainerTree`]
pub type ProjectGraphMap = HashMap<ResourceId, ContainerTree>;

// *****************
// *** Datastore ***
// *****************

// TODO Paths should always be canonicalized.

/// A store for [`Container`](LocalContainer)s.
/// Assets can be referenced as well.
///
/// # Notes
/// + Because local Syre resources can only be controlled by a single process
/// a `Datastore` operates as a single repository for all processes requiring access
/// to these resources.
/// This means that the `Datastore` should control all resources it stores.
pub struct Objectstore {
    /// [`Project`] store.
    projects: ProjectMap,

    /// Map from a [`Project`]'s path to its [`ResourceId`].
    project_paths: PathMap<ResourceId>,

    /// Map from [`Project`] to its graph.
    graphs: ProjectGraphMap,

    /// Map from a [`Container`](LocalContainer)'s path to its [`ResourceId`].
    container_paths: PathMap<ResourceId>,

    /// Map from a `Container`'s id to its `Project`'s.
    container_projects: IdMap,

    /// Map from an [`Asset`]'s [`ResourceId`] to its `Container`'s.
    asset_containers: IdMap,

    /// Map from an [`Asset`]'s path to its [`ResourceId`].
    asset_paths: PathMap<ResourceId>,

    /// Map from an analysis' [`ResourceId`] to its `Project`.
    analysis_projects: IdMap,

    /// Holds a `Project`'s [`Analyses`].
    analyses: AnalysesMap,
}

impl Objectstore {
    pub fn new() -> Self {
        Objectstore {
            projects: ProjectMap::new(),
            project_paths: PathMap::new(),
            graphs: ProjectGraphMap::new(),
            container_paths: PathMap::new(),
            container_projects: IdMap::new(),
            asset_containers: IdMap::new(),
            asset_paths: PathMap::new(),
            analysis_projects: IdMap::new(),
            analyses: AnalysesMap::new(),
        }
    }

    // ***************
    // *** project ***
    // ***************

    /// Inserts a [`Project`] into the database.
    /// Canonicalizes the path.
    ///
    /// # Returns
    /// Reference to the inserted `Project`.
    pub fn insert_project(&mut self, project: Project) -> StdResult<(), io::Error> {
        let pid = project.rid.clone();
        let project_path = project.base_path().to_path_buf();

        self.projects.insert(pid.clone(), project);
        self.project_paths.insert_canonical(project_path, pid)?;

        Ok(())
    }

    /// Removes a `Project` and its resources.
    ///
    /// # Notes
    /// Removes the graph, and all mappings associated with the `Project``.
    pub fn remove_project(&mut self, project: &ResourceId) -> ProjectResources {
        let project = self.projects.remove(project);
        let graph = match project.as_ref() {
            None => None,
            Some(project) => self.remove_project_graph(&project.rid),
        };

        if let Some(project) = project.as_ref() {
            self.project_paths.remove(&project.base_path());
        }

        ProjectResources { project, graph }
    }

    /// Updates a `Project`s path amp and all it's resources' path maps.
    /// Canonicalizes paths.
    ///
    /// # Returns
    /// `Project`'s old path.
    pub fn update_project_path(
        &mut self,
        project: &ResourceId,
        to: impl Into<PathBuf>,
    ) -> StdResult<PathBuf, error::UpdateProjectPath> {
        let to: PathBuf = to.into();
        let Some(project) = self.projects.get_mut(project) else {
            return Err(error::UpdateProjectPath::NotFound);
        };

        let pid = project.rid.clone();
        let old_project_path = project.base_path().to_path_buf();
        project.set_base_path(to.clone());

        self.project_paths.remove(&old_project_path);
        if let Err(err) = self
            .project_paths
            .insert_canonical(project.base_path().to_path_buf(), pid.clone())
        {
            return Err(error::UpdateProjectPath::Canonicalize(err.kind()));
        }

        let mut container_paths_map = Vec::new();
        let mut asset_path_maps = Vec::new();
        if let Some(graph) = self.get_project_graph_mut(&pid) {
            for (_, container) in graph.iter_nodes_mut() {
                let old_container_path = container.base_path().to_path_buf();
                let container_relative_path =
                    old_container_path.strip_prefix(&old_project_path).unwrap();

                let container_path = to.join(container_relative_path);
                container.set_base_path(container_path.clone());
                container_paths_map.push((
                    container.rid.clone(),
                    old_container_path.clone(),
                    container_path.clone(),
                ));

                for (_, asset) in container.assets.iter() {
                    asset_path_maps.push((
                        asset.rid.clone(),
                        old_container_path.join(asset.path.as_path()),
                        container_path.join(asset.path.as_path()),
                    ));
                }
            }
        }

        for (cid, old_container_path, container_path) in container_paths_map {
            self.container_paths.remove(&old_container_path);
            self.container_paths
                .insert_canonical(container_path.clone(), cid.clone())
                .unwrap_or_else(|_| self.container_paths.insert(container_path, cid.clone()));
        }

        for (aid, old_asset_path, asset_path) in asset_path_maps {
            self.asset_paths.remove(&old_asset_path);
            self.asset_paths
                .insert_canonical(asset_path.clone(), aid.clone())
                .unwrap_or_else(|_| self.asset_paths.insert(asset_path, aid.clone()));
        }

        Ok(old_project_path.to_path_buf())
    }

    /// Gets a [`Project`] from the database if it exists,
    /// otherwise `None`.
    pub fn get_project(&self, rid: &ResourceId) -> Option<&Project> {
        self.projects.get(rid)
    }

    /// Gets a `mut`able [`Project`] from the database if it exists,
    /// otherwise `None`.
    pub fn get_project_mut(&mut self, rid: &ResourceId) -> Option<&mut Project> {
        self.projects.get_mut(rid)
    }

    /// Gets the `Project` associated to the given path.
    ///
    /// # Notes
    /// + Canonicalizes the path.
    pub fn get_path_project_canonical(
        &self,
        path: &Path,
    ) -> StdResult<Option<&ResourceId>, io::Error> {
        self.project_paths.get_canonical(path)
    }

    /// Gets the `Project` associated to the given path.
    pub fn get_path_project(&self, path: &Path) -> Option<&ResourceId> {
        self.project_paths.get(path)
    }

    /// Gets the `Project` the `Container` belongs to.
    pub fn get_container_project(&self, container: &ResourceId) -> Option<&ResourceId> {
        self.container_projects.get(container)
    }

    // *************
    // *** graph ***
    // *************

    pub fn graphs(&self) -> &ProjectGraphMap {
        &self.graphs
    }

    /// Gets a [`Project`]'s [`ContainerTree`].
    ///
    /// # Arguments
    /// 1. [`ResourceId`] of the [`Project`].
    pub fn get_project_graph(&self, rid: &ResourceId) -> Option<&ContainerTree> {
        self.graphs.get(&rid)
    }

    /// Gets a `mut`able reference to a [`Project`]'s [`ContainerTree`].
    ///
    /// # Arguments
    /// 1. [`ResourceId`] of the [`Project`].
    pub fn get_project_graph_mut(&mut self, rid: &ResourceId) -> Option<&mut ContainerTree> {
        self.graphs.get_mut(&rid)
    }

    /// Resturns whether the `Project`;s graph is loaded.
    pub fn is_project_graph_loaded(&self, rid: &ResourceId) -> bool {
        self.graphs.contains_key(rid)
    }

    /// Gets the entire graph the `Container` is in.
    pub fn get_graph_of_container(&self, container: &ResourceId) -> Option<&ContainerTree> {
        let Some(project) = self.container_projects.get(&container) else {
            return None;
        };

        self.graphs.get(project)
    }

    /// Gets the entire graph the `Container` is in, `mut`ably.
    fn get_graph_of_container_mut(&mut self, container: &ResourceId) -> Option<&mut ContainerTree> {
        let Some(project) = self.container_projects.get(&container) else {
            return None;
        };

        self.graphs.get_mut(project)
    }

    // TODO: DRY `insert_project_graph` and `insert_sub_graph`.
    /// Inserts a [`Project`](LocalProjet)'s [`ContainerTree`].
    /// Canonicalizes paths.
    ///
    /// # Returns
    /// + The old [`ContainerTree`].
    pub fn insert_project_graph_canonical(
        &mut self,
        project: ResourceId,
        graph: ContainerTree,
    ) -> Result<Option<ContainerTree>> {
        // map containers
        for (cid, node) in graph.nodes().iter() {
            self.container_projects.insert(cid.clone(), project.clone());
            self.container_paths
                .insert_canonical(node.base_path().into(), cid.clone())?;

            // map assets
            for (aid, asset) in node.data().assets.iter() {
                let asset_path = node.base_path().join(asset.path.as_path());
                self.insert_asset_canonical(aid.clone(), asset_path, cid.clone())?;
            }
        }

        Ok(self.graphs.insert(project, graph))
    }

    /// Inserts a [`Project`](LocalProjet)'s [`ContainerTree`].
    /// Attempts tp canonicalize paths.
    /// If an error occurs canonicalizing a Container path, nothing is inserted.
    /// If an error occurs canonicalizing an Asset path, the Asset is inserted with the
    /// non-canonicalized path. An `Err` value is returned.
    ///
    /// # Returns
    /// + On success, the old [`ContainerTree`].
    /// + If errors, the old [`ContainerTree`] with error information.
    pub fn insert_project_graph(
        &mut self,
        project: ResourceId,
        graph: ContainerTree,
    ) -> StdResult<Option<ContainerTree>, error::InsertProjectGraph> {
        // map containers
        let mut errors = HashMap::new();
        for (cid, node) in graph.nodes().iter() {
            match self
                .container_paths
                .insert_canonical(node.base_path().into(), cid.clone())
            {
                Ok(_) => {
                    self.container_projects.insert(cid.clone(), project.clone());
                }
                Err(err) => {
                    errors.insert(node.base_path().to_path_buf(), err.kind());
                }
            };
        }

        if !errors.is_empty() {
            for (cid, container) in graph.iter_nodes() {
                self.container_paths.remove(container.base_path());
                self.container_projects.remove(cid);
            }

            return Err(error::InsertProjectGraph::Tree(errors));
        }

        let mut errors = HashMap::new();
        for (cid, node) in graph.nodes().iter() {
            // map assets
            for (aid, asset) in node.data().assets.iter() {
                let asset_path = node.base_path().join(asset.path.as_path());
                match self.insert_asset_canonical(aid.clone(), asset_path.clone(), cid.clone()) {
                    Ok(_) => {}
                    Err(err) => {
                        errors.insert(aid.clone(), err.kind());
                        self.insert_asset(aid.clone(), asset_path, cid.clone());
                    }
                };
            }
        }

        let old_graph = self.graphs.insert(project, graph);
        if errors.is_empty() {
            Ok(old_graph)
        } else {
            Err(error::InsertProjectGraph::Assets(error::AssetsGraph {
                assets: errors,
                graph: old_graph,
            }))
        }
    }

    /// Removes a `Project`'s graph.
    pub fn remove_project_graph(&mut self, project: &ResourceId) -> Option<ContainerTree> {
        let Some(graph) = self.graphs.remove(project) else {
            return None;
        };

        for (_, container) in graph.iter_nodes() {
            for asset in container.assets.values() {
                let asset_path = container.base_path().join(asset.path.as_path());
                self.asset_paths
                    .remove_canonical(&asset_path)
                    .unwrap_or_else(|_| self.asset_paths.remove(&asset_path));

                self.asset_containers.remove(&asset.rid);
            }

            self.container_paths
                .remove_canonical(container.base_path())
                .unwrap_or_else(|_| self.container_paths.remove(container.base_path()));

            self.container_projects.remove(&container.rid);
        }

        Some(graph)
    }

    /// Insert a graph into another.
    pub fn insert_subgraph(
        &mut self,
        parent: &ResourceId,
        graph: ResourceTree<LocalContainer>,
    ) -> Result {
        let Some(project) = self.get_container_project(parent).cloned() else {
            return Err(CoreError::Resource(ResourceError::does_not_exist(
                "`Container` `Project` not found",
            ))
            .into());
        };

        if !self.graphs.contains_key(&project) {
            panic!("`Project` graph not found");
        }

        for (cid, container) in graph.nodes() {
            // map container to project
            self.container_projects.insert(cid.clone(), project.clone());

            // map path to container
            self.container_paths
                .insert_canonical(container.base_path().into(), cid.clone())?;

            // map assets
            for (aid, asset) in container.assets.iter() {
                let asset_path = container.base_path().join(asset.path.as_path());
                self.insert_asset_canonical(aid.clone(), asset_path, cid.clone())?;
            }
        }

        self.graphs
            .get_mut(&project)
            .unwrap()
            .insert_tree(parent, graph)?;

        Ok(())
    }

    /// Move a subgraph.
    ///
    /// # Notes
    /// + This does not affect node paths.
    /// + This does not affect the file system.
    pub fn move_subgraph(&mut self, root: &ResourceId, parent: &ResourceId) -> Result {
        let Some(project) = self.get_container_project(root).cloned() else {
            return Err(CoreError::Resource(ResourceError::does_not_exist(
                "`Container` `Project` not found",
            ))
            .into());
        };

        let graph = self
            .graphs
            .get_mut(&project)
            .expect("`Project` graph not found");

        graph.mv(root, parent)?;
        Ok(())
    }

    /// Remove the subgraph with the given root.
    ///
    /// # Returns
    /// Removed subgraph.
    #[tracing::instrument(skip(self))]
    pub fn remove_subgraph(&mut self, root: &ResourceId) -> Result<ContainerTree> {
        let Some(project) = self.get_container_project(root).cloned() else {
            return Err(CoreError::Resource(ResourceError::does_not_exist(
                "`Container` `Project` not found",
            ))
            .into());
        };

        let graph = self
            .graphs
            .get_mut(&project)
            .expect("`Project` graph not found");

        let sub_graph = graph.remove(root)?;
        for (cid, container) in sub_graph.nodes() {
            // remove maps
            self.container_projects.remove(cid);
            self.container_paths
                .remove_canonical(&container.base_path())
                .unwrap_or_else(|_| self.container_paths.remove(&container.base_path()));

            for (aid, asset) in container.assets.iter() {
                self.asset_containers.remove(aid);

                let asset_path = container.base_path().join(asset.path.as_path());
                self.asset_paths
                    .remove_canonical(&asset_path)
                    .unwrap_or_else(|_| self.asset_paths.remove(&asset_path));
            }
        }

        Ok(sub_graph)
    }

    /// Updates the path to a subtree.
    ///
    /// # Notes
    /// + This does not affect the graph in any way.
    /// + This does not affect the file system.
    pub fn update_subgraph_path(
        &mut self,
        root: &ResourceId,
        path: impl Into<PathBuf>,
    ) -> StdResult<(), UpdateSubgraphPathError> {
        let path = path.into();
        let Some(graph) = self.get_graph_of_container(root) else {
            return Err(UpdateSubgraphPathError::RootNotFound);
        };

        let descendants = graph.descendants(root).unwrap();
        let container_path = graph.get(root).unwrap().base_path().to_path_buf();

        for rid in descendants {
            let descendant = self.get_container_mut(&rid).unwrap();
            let descendant_id = descendant.rid.clone();
            let old_path = descendant.base_path().to_path_buf();
            let new_path = old_path.strip_prefix(&container_path).unwrap();
            let new_path = path.join(new_path);
            if new_path == old_path {
                continue;
            }

            let assets = descendant
                .assets
                .values()
                .map(|asset| (asset.rid.clone(), asset.path.as_path().to_owned()))
                .collect::<Vec<_>>();

            descendant.set_base_path(new_path.clone());
            let descendant_path = descendant.base_path().to_owned();
            match self.container_paths.remove_canonical(&old_path) {
                Ok(_) => {}
                Err(_) => {
                    // entry was not present
                    // this is likely because only the capitalization of the path changed
                    // and the file has already been moved
                    // so the canonicalized path is referring to the new capitalization scheme
                    // stranding the old path key stranded when using canonicalization
                    self.container_paths.remove(&old_path).unwrap();
                }
            };

            if let Err(err) = self
                .container_paths
                .insert_canonical(new_path.clone(), descendant_id.clone())
            {
                return Err(UpdateSubgraphPathError::Canonicalization {
                    resource: descendant_id,
                    path: new_path,
                    kind: err.kind(),
                });
            };

            for (aid, asset_path) in assets {
                self.asset_paths.remove(&old_path.join(&asset_path));
                self.asset_paths
                    .insert(descendant_path.join(asset_path), aid);
            }
        }

        Ok(())
    }

    // *****************
    // *** container ***
    // *****************

    /// Gets a [`Container`](LocalContainer).
    pub fn get_container(&self, container: &ResourceId) -> Option<&LocalContainer> {
        let Some(graph) = self.get_graph_of_container(container) else {
            return None;
        };

        let Some(node) = graph.get(container) else {
            return None;
        };

        Some(node.data())
    }

    /// Gets a `mut`able [`Container`](LocalContainer).
    pub fn get_container_mut(&mut self, container: &ResourceId) -> Option<&mut LocalContainer> {
        let Some(graph) = self.get_graph_of_container_mut(container) else {
            return None;
        };

        let Some(node) = graph.get_mut(container) else {
            return None;
        };

        Some(node)
    }

    /// Get a Container with inherited metadata.
    pub fn get_container_with_metadata(&self, container: &ResourceId) -> Option<CoreContainer> {
        let Some(container) = self.get_container(container) else {
            return None;
        };

        let graph = self
            .get_graph_of_container(container.id())
            .expect("could not find `Container`'s graph");

        let metadata = graph.ancestors(container.id()).into_iter().rfold(
            Metadata::new(),
            |mut metadata, ancestor| {
                let container = graph.get(&ancestor).expect("`Container` not found");
                for (key, value) in container.properties.metadata.clone() {
                    metadata.insert(key, value);
                }

                metadata
            },
        );

        let container_path = container.base_path().to_owned();
        let mut container = (*container).clone();
        container.properties.metadata = metadata;
        for asset in container.assets.values_mut() {
            for (key, value) in container.properties.metadata.iter() {
                if !asset.properties.metadata.contains_key(key) {
                    asset.properties.metadata.insert(key.clone(), value.clone());
                }
            }

            let path = fs::canonicalize(container_path.join(asset.path.as_path())).unwrap();
            asset.path = path;
        }

        Some(container)
    }

    /// Finds `Container`'s that match the filter.
    ///
    /// # Arguments
    /// 1. Root of subtree to search within.
    /// 2. Filter.
    ///
    /// # Note
    /// + `Metadata` is not inherited.
    ///
    /// # See also
    /// + [`find_containers_with_metadata`]
    pub fn find_containers(
        &self,
        root: &ResourceId,
        filter: StdFilter,
    ) -> HashSet<&LocalContainer> {
        let mut found = HashSet::new();
        let Some(graph) = self.get_graph_of_container(root) else {
            return found;
        };

        let nodes = graph
            .descendants(&root)
            .expect("`Container` not found in graph");

        for node in nodes {
            let node = graph.get(&node).expect("`Container` not found in graph");

            // @todo[4]: Implement for `LocalContainer`.
            let container: CoreContainer = (*node.data()).clone();
            if filter.matches(&container) {
                found.insert(node.data());
            }
        }

        found
    }

    // TODO[h] Assets should have paths canonicalized and made absolute.
    /// Finds `Container`'s that match the filter with inherited `Metadata`.
    ///
    /// # Arguments
    /// 1. Root of subtree to search within.
    /// 2. Filter.
    ///
    /// # See also
    /// + [`find_containers`]
    #[tracing::instrument(skip(self))]
    pub fn find_containers_with_metadata(
        &self,
        root: &ResourceId,
        filter: StdFilter,
    ) -> HashSet<CoreContainer> {
        /// Recursively finds mathcing `Containers`, inheriting metadata.
        #[tracing::instrument(skip(graph))]
        fn find_containers_with_metadata_recursive(
            root: &ResourceId,
            graph: &ContainerTree,
            filter: StdFilter,
            mut metadata: Metadata,
        ) -> HashSet<CoreContainer> {
            let mut found = HashSet::new();
            let root = graph.get(root).expect("`Container` not in graph");

            let children = graph.children(root.id()).expect("`Container` not in graph");
            for (key, value) in root.data().properties.metadata.clone().into_iter() {
                metadata.insert(key, value);
            }

            for child in children {
                let node = graph.get(&child).expect("child `Container` not in graph");
                for matching in find_containers_with_metadata_recursive(
                    node.id(),
                    &graph,
                    filter.clone(),
                    metadata.clone(),
                )
                .into_iter()
                {
                    found.insert(matching);
                }
            }

            let mut container: CoreContainer = (*root.data()).clone();
            container.properties.metadata = metadata;
            if filter.matches(&container) {
                for asset in container.assets.values_mut() {
                    for (key, value) in container.properties.metadata.iter() {
                        if !asset.properties.metadata.contains_key(key) {
                            asset.properties.metadata.insert(key.clone(), value.clone());
                        }
                    }

                    let path = root.base_path().join(asset.path.as_path());
                    asset.path = path;
                }

                found.insert(container);
            }

            found
        }

        // run fn
        let Some(graph) = self.get_graph_of_container(root) else {
            return HashSet::new();
        };

        let metadata =
            graph
                .ancestors(root)
                .into_iter()
                .rfold(Metadata::new(), |mut metadata, ancestor| {
                    let container = graph.get(&ancestor).expect("`Container` not found");
                    for (key, value) in container.properties.metadata.clone() {
                        metadata.insert(key, value);
                    }

                    metadata
                });

        find_containers_with_metadata_recursive(root, graph, filter, metadata)
    }

    /// Get a `Container`'s id by its path.
    ///
    /// # See also
    /// + [`get_path_container_canonical`]
    pub fn get_path_container(&self, path: &Path) -> Option<&ResourceId> {
        self.container_paths.get(path)
    }

    /// Gets a `Container`'s id by path.
    ///
    /// # Notes
    /// + Path is canonicalized.
    pub fn get_path_container_canonical(
        &self,
        path: &Path,
    ) -> StdResult<Option<&ResourceId>, io::Error> {
        self.container_paths.get_canonical(path)
    }

    // *************
    // *** asset ***
    // *************

    /// Gets an [`Asset`](LocalAsset)'s [`Container`](LocalContainer) [`ResourceId`]
    /// from the database if it exists, otherwise `None`.
    pub fn get_asset_container_id(&self, rid: &ResourceId) -> Option<&ResourceId> {
        self.asset_containers.get(rid)
    }

    /// Gets an [`Asset`](LocalAsset)'s [`Container`](LocalContainer)
    /// from the database if it exists, otherwise `None`.
    pub fn get_asset_container(&self, rid: &ResourceId) -> Option<&LocalContainer> {
        let Some(container) = self.asset_containers.get(rid) else {
            return None;
        };

        let container = self
            .get_container(container)
            .expect("`Container` not found in graph");

        Some(container)
    }

    /// Get the [`ResourceId`] of the `Asset` associated with the path.
    ///
    /// # See also
    /// + `get_path_asset_id_canoncial`
    pub fn get_path_asset_id(&self, path: impl AsRef<Path>) -> Option<&ResourceId> {
        self.asset_paths.get(path.as_ref())
    }

    /// Get the [`ResourceId`] of the `Asset` associated with the path.
    /// `path` is first canonicalized.
    pub fn get_path_asset_id_canonical(
        &self,
        path: impl AsRef<Path>,
    ) -> StdResult<Option<&ResourceId>, io::Error> {
        self.asset_paths.get_canonical(path.as_ref())
    }

    /// Inserts maps for an [`Asset`].
    pub fn insert_asset(
        &mut self,
        asset: ResourceId,
        path: PathBuf,
        container: ResourceId,
    ) -> Option<ResourceId> {
        self.asset_containers.insert(asset.clone(), container);
        self.asset_paths.insert(path, asset)
    }

    /// Inserts maps for an [`Asset`].
    /// Canonicalizes `asset_path`.
    pub fn insert_asset_canonical(
        &mut self,
        asset: ResourceId,
        path: PathBuf,
        container: ResourceId,
    ) -> StdResult<Option<ResourceId>, io::Error> {
        self.asset_containers.insert(asset.clone(), container);
        self.asset_paths.insert_canonical(path, asset)
    }

    /// Adds an [`Asset`](CoreAsset) to a `Container`.
    pub fn add_asset(&mut self, mut asset: Asset, container: ResourceId) -> Result<Option<Asset>> {
        let Some(project) = self.container_projects.get(&container) else {
            return Err(CoreError::Resource(ResourceError::does_not_exist(
                "`Container` is not loaded",
            ))
            .into());
        };

        let graph = self
            .graphs
            .get_mut(project)
            .expect("`Project` present without graph");

        let Some(container) = graph.get_mut(&container) else {
            return Err(CoreError::Resource(ResourceError::does_not_exist(
                "`Container` is not loaded",
            ))
            .into());
        };

        // check if asset with same path already extists
        for c_asset in container.assets.values() {
            if asset.path == c_asset.path {
                asset.rid = c_asset.rid.clone();
                break;
            }
        }

        let aid = asset.rid.clone();
        let cid = container.rid.clone();
        let asset_path = container.base_path().join(asset.path.as_path());
        let o_asset = container.insert_asset(asset);
        container.save()?;

        let asset_path = match fs::canonicalize(&asset_path) {
            Ok(path) => path,
            Err(_) => {
                if cfg!(target_os = "windows") {
                    syre_local::common::ensure_windows_unc(asset_path)
                } else {
                    asset_path
                }
            }
        };

        self.insert_asset(aid, asset_path, cid);
        Ok(o_asset)
    }

    /// Removes an `Asset` from its `Container`.
    ///
    /// # Returns
    /// Tuple of the `Asset` and the `Asset`'s canonicalized path.
    #[tracing::instrument(skip(self))]
    pub fn remove_asset(&mut self, rid: &ResourceId) -> Result<Option<(Asset, PathBuf)>> {
        let Some(cid) = self.asset_containers.get(rid).cloned() else {
            return Err(CoreError::Resource(ResourceError::does_not_exist(
                "`Container` is not loaded",
            ))
            .into());
        };

        let Some(graph) = self.get_graph_of_container_mut(&cid) else {
            return Err(CoreError::Resource(ResourceError::does_not_exist(
                "`Container`'s graph is not loaded",
            ))
            .into());
        };

        let Some(container) = graph.get_mut(&cid) else {
            return Err(CoreError::Resource(ResourceError::does_not_exist(
                "`Container` not in graph",
            ))
            .into());
        };

        let container_path = container.base_path().to_path_buf();
        let asset = container.assets.remove(rid);
        container.save()?;
        self.asset_containers.remove(rid);

        if let Some(asset) = asset {
            let path = container_path.join(asset.path.as_path());
            let path = match fs::canonicalize(path.clone()) {
                Ok(path) => path,

                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                    if cfg!(target_os = "windows") {
                        syre_local::common::ensure_windows_unc(path)
                    } else {
                        path
                    }
                }

                Err(err) => panic!("unhandled error {err}"),
            };
            self.asset_paths.remove(&path);

            Ok(Some((asset, path)))
        } else {
            Ok(None)
        }
    }

    /// Updates an [`Asset`]'s path.
    /// Sets the `Asset`'s path relative to its `Container`.
    ///
    /// # Arguments
    /// + `path` should be the path relative to its `Container`.
    pub fn update_asset_path(&mut self, asset: &ResourceId, path: impl Into<PathBuf>) -> Result {
        let path = path.into();
        assert!(path.is_relative());

        let container = self.get_asset_container_id(asset).unwrap().clone();
        let container = self.get_container_mut(&container).unwrap();
        let container_path = container.base_path().to_path_buf();
        let asset = container.assets.get_mut(asset).unwrap();
        let aid = asset.rid.clone();
        let asset_path = asset.path.clone();

        asset.path = path.clone();
        container.save()?;

        let old_asset_path = container_path.join(&asset_path);
        self.asset_paths.remove(&old_asset_path);

        let path = container_path.join(path);
        self.asset_paths.insert_canonical(path, aid)?;
        Ok(())
    }

    /// Moves an [`Asset`] to another [`Container`](CoreContainer).
    ///
    /// # Notes
    /// + Does not manipulate the `Asset`'s file.
    pub fn move_asset(&mut self, asset: &ResourceId, container: &ResourceId) -> Result {
        let Some(asset_container) = self.get_asset_container_id(asset).cloned() else {
            return Err(CoreError::Resource(ResourceError::does_not_exist(
                "`Asset` does not exist",
            ))
            .into());
        };

        let asset_container = self.get_container_mut(&asset_container).unwrap();
        let aid = asset.clone();
        let asset = asset_container.remove_asset(asset).unwrap();
        let asset_path_old = asset_container.base_path().join(asset.path.as_path());
        asset_container.save()?;

        let Some(container) = self.get_container_mut(container) else {
            return Err(CoreError::Resource(ResourceError::does_not_exist(
                "`Container` does not exist",
            ))
            .into());
        };

        let cid = container.rid.clone();
        let asset_path = container.base_path().join(asset.path.as_path());
        container.insert_asset(asset);
        container.save()?;

        self.asset_containers.insert(aid.clone(), cid);
        self.asset_paths.remove(&asset_path_old);
        self.asset_paths.insert(asset_path, aid);

        Ok(())
    }

    /// Finds `Asset`'s that match the filter.
    ///
    /// # Arguments
    /// 1. Root of subtree to search within.
    /// 2. Filter.
    ///
    /// # Note
    /// + `Metadata` is not inherited.
    ///
    /// # See also
    /// + [`find_assets_with_metadata`]
    pub fn find_assets(&self, root: &ResourceId, filter: StdFilter) -> HashSet<Asset> {
        let mut found = HashSet::new();
        let Some(graph) = self.get_graph_of_container(root) else {
            return found;
        };

        let nodes = graph
            .descendants(root)
            .expect("`Container` not found in graph");

        for node in nodes {
            let container = graph.get(&node).expect("`Container` not found in graph");

            for asset in container.data().assets.values() {
                if filter.matches(asset) {
                    // set path to absolute
                    let mut asset = asset.clone();
                    let path = container.base_path().join(&asset.path);
                    asset.path = path;

                    found.insert(asset);
                }
            }
        }

        found
    }

    /// Finds `Asset`'s that match the filter with inherited `Metadata`.
    ///
    /// # Arguments
    /// 1. Root of subtree to search within.
    /// 2. Filter.
    ///
    /// # See also
    /// + [`find_assets`]
    pub fn find_assets_with_metadata(
        &self,
        root: &ResourceId,
        filter: StdFilter,
    ) -> HashSet<Asset> {
        /// Recursively finds mathcing `Containers`, inheriting metadata.
        fn find_assets_with_metadata_recursive(
            root: &ResourceId,
            graph: &ContainerTree,
            filter: StdFilter,
            mut metadata: Metadata,
        ) -> HashSet<Asset> {
            let mut found = HashSet::new();
            let root = graph.get(root).expect("`Container` not in graph");

            let children = graph.children(root.id()).expect("`Container` not in graph");
            for (key, value) in root.data().properties.metadata.clone().into_iter() {
                metadata.insert(key, value);
            }

            for child in children {
                let node = graph.get(&child).expect("child `Container` not in graph");
                for matching in find_assets_with_metadata_recursive(
                    node.id(),
                    &graph,
                    filter.clone(),
                    metadata.clone(),
                )
                .into_iter()
                {
                    found.insert(matching);
                }
            }

            for asset in root.data().assets.values() {
                let mut asset = asset.clone();
                for (key, value) in metadata.clone().into_iter() {
                    asset.properties.metadata.entry(key).or_insert(value);
                }

                if filter.matches(&asset) {
                    // set path to absolute
                    let path = root.base_path().join(&asset.path);
                    asset.path = path;

                    found.insert(asset);
                }
            }

            found
        }

        // find mathing containers
        let Some(graph) = self.get_graph_of_container(root) else {
            return HashSet::new();
        };

        let metadata =
            graph
                .ancestors(root)
                .into_iter()
                .rfold(Metadata::new(), |mut metadata, ancestor| {
                    let container = graph.get(&ancestor).expect("`Container` not found");
                    for (key, value) in container.properties.metadata.clone() {
                        metadata.insert(key, value);
                    }

                    metadata
                });

        find_assets_with_metadata_recursive(root, graph, filter, metadata)
    }

    // **************
    // *** script ***
    // **************

    pub fn insert_project_scripts(
        &mut self,
        project: ResourceId,
        scripts: Analyses,
    ) -> Option<Analyses> {
        // map scripts
        for script in scripts.keys() {
            self.analysis_projects
                .insert(script.clone(), project.clone());
        }

        self.analyses.insert(project, scripts)
    }

    /// Gets a `Project`'s `Script`s.
    pub fn get_project_scripts(&self, project: &ResourceId) -> Option<&Analyses> {
        self.analyses.get(&project)
    }

    /// Gets a `mut`able reference to a `Project`'s `Script`s.
    pub fn get_project_scripts_mut(&mut self, project: &ResourceId) -> Option<&mut Analyses> {
        self.analyses.get_mut(&project)
    }

    /// Returns if the `Project`'s `Scripts` are loaded.
    pub fn are_project_scripts_loaded(&self, project: &ResourceId) -> bool {
        self.analyses.contains_key(project)
    }

    /// Gets the `Project` of a `Script`.
    pub fn get_script_project(&self, script: &ResourceId) -> Option<&ResourceId> {
        self.analysis_projects.get(&script)
    }

    pub fn insert_script(
        &mut self,
        project: ResourceId,
        script: Script,
    ) -> Result<Option<AnalysisKind>> {
        let Some(analyses) = self.analyses.get_mut(&project) else {
            return Err(CoreError::Resource(ResourceError::does_not_exist(
                "`Project` does not exist",
            ))
            .into());
        };

        let sid = script.rid.clone();
        let o_script = analyses.insert(sid.clone(), script.into());
        analyses.save()?;

        // map script
        self.analysis_projects.insert(sid, project);
        Ok(o_script)
    }

    /// Remove a `Script` from a `Project`.
    /// Removes all `Container` associations.
    ///
    /// # Returns
    /// Removed `Script`.
    pub fn remove_project_script(
        &mut self,
        project: &ResourceId,
        script: &ResourceId,
    ) -> Result<Option<AnalysisKind>> {
        let Some(analyses) = self.analyses.get_mut(&project) else {
            // project does not exist
            return Err(CoreError::Resource(ResourceError::does_not_exist(
                "`Project`'s `Scripts` does not exist",
            ))
            .into());
        };

        // remove association from contiainers
        let Some(graph) = self.graphs.get_mut(project) else {
            return Err(CoreError::Resource(ResourceError::does_not_exist(
                "`Project`'s graph does not exist",
            ))
            .into());
        };

        for (_cid, container) in graph.iter_nodes_mut() {
            container.analyses.remove(script);
            container.save()?;
        }

        // remove from project
        let o_script = analyses.remove(script);
        analyses.save()?;

        // remove map script
        self.analysis_projects.remove(script);

        Ok(o_script)
    }

    /// Remove an analysis from a `Project`.
    /// Removes all `Container` associations.
    ///
    /// # Returns
    /// Removed analysis.
    pub fn remove_project_analysis(
        &mut self,
        project: &ResourceId,
        analysis: &ResourceId,
    ) -> Result<Option<AnalysisKind>> {
        let Some(analyses) = self.analyses.get_mut(&project) else {
            // project does not exist
            return Err(CoreError::Resource(ResourceError::does_not_exist(
                "`Project`'s `Scripts` does not exist",
            ))
            .into());
        };

        // remove association from contiainers
        let Some(graph) = self.graphs.get_mut(project) else {
            return Err(CoreError::Resource(ResourceError::does_not_exist(
                "`Project`'s graph does not exist",
            ))
            .into());
        };

        for (_cid, container) in graph.iter_nodes_mut() {
            container.analyses.remove(analysis);
            container.save()?;
        }

        // remove from project
        let o_analysis = analyses.remove(analysis);
        analyses.save()?;

        // remove map script
        self.analysis_projects.remove(analysis);
        Ok(o_analysis)
    }

    pub fn get_analysis(&self, analysis: &ResourceId) -> Option<&AnalysisKind> {
        let Some(project) = self.analysis_projects.get(analysis) else {
            return None;
        };

        let Some(analyses) = self.analyses.get(&project) else {
            return None;
        };

        analyses.get(&analysis)
    }

    pub fn get_analysis_mut(&mut self, analysis: &ResourceId) -> Option<&mut AnalysisKind> {
        let Some(project) = self.analysis_projects.get(analysis) else {
            return None;
        };

        let Some(scripts) = self.analyses.get_mut(project) else {
            return None;
        };

        scripts.get_mut(analysis)
    }

    pub fn insert_excel_template(
        &mut self,
        project: ResourceId,
        template: ExcelTemplate,
    ) -> Result<Option<AnalysisKind>> {
        let Some(analyses) = self.analyses.get_mut(&project) else {
            return Err(CoreError::Resource(ResourceError::does_not_exist(
                "`Project` does not exist",
            ))
            .into());
        };

        let tid = template.rid.clone();
        let o_analysis = analyses.insert(tid.clone(), template.into());
        analyses.save()?;

        // map script
        self.analysis_projects.insert(tid, project);
        Ok(o_analysis)
    }
}

pub mod error {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::io;
    use std::path::PathBuf;
    use syre_core::graph::ResourceTree;
    use syre_core::types::ResourceId;
    use syre_local::error::IoErrorKind;
    use syre_local::project::resources::Container;
    use thiserror::Error;

    type ContainerTree = ResourceTree<Container>;

    #[derive(Debug)]
    pub enum UpdateProjectPath {
        /// The `Project` was not found.
        NotFound,

        /// The `Project`'s new path could not be canonicalized.
        Canonicalize(io::ErrorKind),
    }

    #[derive(Error, Debug)]
    #[error("{assets:?}")]
    pub struct AssetsGraph {
        /// Assets that error on canonicalization.
        pub assets: HashMap<ResourceId, io::ErrorKind>,

        /// Old graph
        pub graph: Option<ResourceTree<Container>>,
    }

    #[derive(Error, Debug)]
    pub enum InsertProjectGraph {
        #[error("{0:?}")]
        Tree(HashMap<PathBuf, io::ErrorKind>),

        #[error("{0:?}")]
        Assets(AssetsGraph),
    }
}

#[cfg(test)]
#[path = "./object_store_test.rs"]
mod object_store_test;
