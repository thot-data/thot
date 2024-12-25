//! Project state.
use super::Error;
use crate::{
    common,
    state::{DataResource, FileResource, FolderResource},
};
pub use action::Action;
use std::path::PathBuf;
use syre_local::{file_resource::LocalResource, TryReducible};

/// Project state.
#[derive(Debug)]
pub struct State {
    path: PathBuf,
    fs_resource: FolderResource<project::State>,
}

impl State {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            fs_resource: FolderResource::Absent,
        }
    }

    pub fn with_project(path: impl Into<PathBuf>, project: project::State) -> Self {
        Self {
            path: path.into(),
            fs_resource: FolderResource::Present(project),
        }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn fs_resource(&self) -> &FolderResource<project::State> {
        &self.fs_resource
    }
}

impl State {
    /// # Arguments
    /// + `path`: Path to project base directory.
    ///
    /// # Panics
    /// + If graph is present, but invalid.
    pub fn load(path: impl Into<PathBuf>) -> Self {
        use crate::state;
        use syre_local::{
            project::resources::{project::LoadError, Analyses, Project},
            types::AnalysisKind,
        };

        let path = path.into();
        let mut state = Self::new(&path);
        if !state.path().is_dir() {
            return state;
        }

        let mut project = project::Builder::default();
        match Project::load_from(state.path()) {
            Ok(prj) => {
                let (properties, settings, path) = prj.into_parts();
                assert_eq!(&path, state.path());

                project.set_properties_ok(properties);
                project.set_settings_ok(settings);
            }

            Err(LoadError {
                properties,
                settings,
            }) => {
                project.set_properties(properties);
                project.set_settings(settings);
            }
        };

        let analyses = Analyses::load_from(state.path()).map(|analyses| {
            let analysis_root = if let DataResource::Ok(ref project) = project.properties() {
                project
                    .analysis_root
                    .as_ref()
                    .map(|analysis_root| analyses.base_path().join(analysis_root))
            } else {
                None
            };

            analyses
                .to_vec()
                .into_iter()
                .map(|analysis| match analysis {
                    AnalysisKind::Script(ref script) => match &analysis_root {
                        Some(analysis_root) => {
                            if analysis_root.join(&script.path).is_file() {
                                state::Analysis::present(analysis)
                            } else {
                                state::Analysis::absent(analysis)
                            }
                        }
                        None => state::Analysis::absent(analysis),
                    },
                    AnalysisKind::ExcelTemplate(ref template) => match &analysis_root {
                        Some(analysis_root) => {
                            if analysis_root.join(&template.template.path).is_file() {
                                state::Analysis::present(analysis)
                            } else {
                                state::Analysis::absent(analysis)
                            }
                        }
                        None => state::Analysis::absent(analysis),
                    },
                })
                .collect::<Vec<_>>()
        });
        project.set_analyses(analyses);

        if let Result::Ok(properties) = project.properties().as_ref() {
            if let Ok(graph) = graph::State::load(state.path().join(&properties.data_root)) {
                project.set_graph(graph)
            }
        }

        state
            .try_reduce(Action::CreateFolder(project.build()))
            .unwrap();

        state
    }
}

impl TryReducible for State {
    type Action = Action;
    type Error = Error;
    fn try_reduce(&mut self, action: Self::Action) -> Result<(), Self::Error> {
        match action {
            Action::SetPath(path) => {
                self.path = path;
                Ok(())
            }
            Action::RemoveFolder => {
                self.fs_resource = FolderResource::Absent;
                Ok(())
            }
            Action::CreateFolder(project) => {
                self.fs_resource = FolderResource::Present(project);
                Ok(())
            }
            Action::RemoveConfig
            | Action::SetProperties(_)
            | Action::SetSettings(_)
            | Action::SetAnalyses(_)
            | Action::SetAnalysesAbsent
            | Action::Graph(_)
            | Action::Container { .. } => {
                let FolderResource::Present(project) = self.fs_resource.as_mut() else {
                    return Err(Error::InvalidTransition);
                };

                project.try_reduce(action)
            }
        }
    }
}

pub mod project {
    use super::{action, graph, Action, DataResource, Error, FolderResource};
    use crate::state::{self, FileResource};
    use std::{
        io::{self, ErrorKind},
        path::PathBuf,
    };
    use syre_core::project::Project as CoreProject;
    use syre_local::{error::IoSerde, project::config::Settings, TryReducible};

    #[derive(Debug)]
    pub struct Builder {
        properties: DataResource<CoreProject>,
        settings: DataResource<Settings>,
        analyses: DataResource<Vec<state::Analysis>>,
        graph: FolderResource<graph::State>,
    }

    impl Builder {
        pub fn properties(&self) -> &DataResource<CoreProject> {
            &self.properties
        }

        pub fn set_properties(&mut self, properties: DataResource<CoreProject>) {
            self.properties = properties;
        }

        pub fn set_properties_ok(&mut self, properties: CoreProject) {
            self.properties = DataResource::Ok(properties);
        }

        pub fn set_properties_err(&mut self, properties: impl Into<IoSerde>) {
            self.properties = DataResource::Err(properties.into());
        }

        pub fn set_settings(&mut self, settings: DataResource<Settings>) {
            self.settings = settings;
        }

        pub fn set_settings_ok(&mut self, settings: Settings) {
            self.settings = DataResource::Ok(settings);
        }

        pub fn set_settings_err(&mut self, settings: impl Into<IoSerde>) {
            self.settings = DataResource::Err(settings.into());
        }

        pub fn set_analyses(&mut self, analyses: DataResource<Vec<state::Analysis>>) {
            self.analyses = analyses;
        }

        pub fn set_analyses_ok(&mut self, analyses: Vec<state::Analysis>) {
            self.analyses = DataResource::Ok(analyses);
        }

        pub fn set_analyses_err(&mut self, analyses: impl Into<IoSerde>) {
            self.analyses = DataResource::Err(analyses.into());
        }

        pub fn set_graph(&mut self, graph: graph::State) {
            self.graph = FolderResource::Present(graph);
        }

        pub fn remove_graph(&mut self) {
            self.graph = FolderResource::Absent;
        }

        pub fn build(self) -> State {
            let Self {
                properties,
                settings,
                analyses,
                graph,
            } = self;

            State {
                properties,
                settings,
                analyses,
                graph,
            }
        }
    }

    impl Default for Builder {
        /// Initialize all resources in a "missing" state.
        fn default() -> Self {
            Self {
                properties: DataResource::Err(ErrorKind::NotFound.into()),
                settings: DataResource::Err(ErrorKind::NotFound.into()),
                analyses: DataResource::Err(ErrorKind::NotFound.into()),
                graph: FolderResource::Absent,
            }
        }
    }

    #[derive(Debug)]
    pub struct State {
        properties: DataResource<CoreProject>,
        settings: DataResource<Settings>,
        analyses: DataResource<Vec<state::Analysis>>,
        graph: FolderResource<graph::State>,
    }

    impl State {
        pub fn properties(&self) -> DataResource<&CoreProject> {
            self.properties.as_ref().map_err(|err| err.clone())
        }

        pub fn settings(&self) -> DataResource<&Settings> {
            self.settings.as_ref().map_err(|err| err.clone())
        }

        pub fn analyses(&self) -> DataResource<&Vec<state::Analysis>> {
            self.analyses.as_ref().map_err(|err| err.clone())
        }

        pub fn graph(&self) -> &FolderResource<graph::State> {
            &self.graph
        }

        /// Creates a [`state::ProjectData`] from the state.
        pub fn project_data(&self) -> state::ProjectData {
            state::ProjectData {
                properties: self.properties.clone(),
                settings: self.settings.clone(),
                analyses: self.analyses.clone(),
            }
        }
    }

    impl TryReducible for State {
        type Action = Action;
        type Error = Error;

        #[tracing::instrument(level = "trace", skip(self))]
        fn try_reduce(&mut self, action: Self::Action) -> std::result::Result<(), Self::Error> {
            match action {
                Action::SetPath(_) | Action::RemoveFolder | Action::CreateFolder(_) => {
                    unreachable!("handled elsewhere");
                }
                Action::RemoveConfig => {
                    self.properties = DataResource::Err(io::ErrorKind::NotFound.into());
                    self.settings = DataResource::Err(io::ErrorKind::NotFound.into());
                    self.analyses = DataResource::Err(io::ErrorKind::NotFound.into());
                    Ok(())
                }
                Action::SetProperties(properties) => {
                    self.properties = properties;
                    Ok(())
                }
                Action::SetSettings(settings) => {
                    self.settings = settings;
                    Ok(())
                }
                Action::SetAnalyses(analyses) => {
                    self.analyses = analyses;
                    Ok(())
                }
                Action::SetAnalysesAbsent => {
                    if let Ok(analyses) = self.analyses.as_mut() {
                        for analysis in analyses.iter_mut() {
                            analysis.set_absent();
                        }
                    }

                    Ok(())
                }
                Action::Graph(action) => self.try_reduce_graph(action),
                Action::Container { path, action } => self.try_reduce_container(path, action),
            }
        }
    }

    impl State {
        #[tracing::instrument(level = "trace", skip(self))]
        fn try_reduce_graph(&mut self, action: action::Graph) -> Result<(), Error> {
            match action {
                super::action::Graph::Set(graph) => {
                    self.graph = graph;
                    Ok(())
                }
                super::action::Graph::Insert {
                    parent,
                    graph: subgraph,
                } => {
                    let FolderResource::Present(ref mut graph) = self.graph else {
                        tracing::error!("graph does not exist");
                        return Err(Error::DoesNotExist);
                    };

                    let Some(parent_node) = graph.find(&parent).unwrap().cloned() else {
                        tracing::error!("parent does not exist");
                        return Err(Error::DoesNotExist);
                    };

                    if let Err(err) = graph.insert(&parent_node, subgraph.clone()) {
                        tracing::error!(?err);
                        match err {
                            graph::error::Insert::ParentNotFound => {
                                return Err(Error::DoesNotExist)
                            }
                            graph::error::Insert::NameCollision => {
                                if cfg!(target_os = "windows") {
                                    tracing::warn!("inserting graph with name collision");
                                    let root = subgraph.root().lock().unwrap();
                                    let existing_path = parent.join(root.name());
                                    drop(root);
                                    let existing =
                                        graph.find(existing_path).unwrap().unwrap().clone();
                                    graph.remove(&existing).unwrap();
                                    graph.insert(&parent_node, subgraph).unwrap();
                                    return Ok(());
                                } else {
                                    return Err(Error::InvalidTransition);
                                }
                            }
                        }
                    }

                    Ok(())
                }
                action::Graph::Remove(path) => {
                    let FolderResource::Present(ref mut graph) = self.graph else {
                        tracing::error!("graph does not exist");
                        return Err(Error::DoesNotExist);
                    };

                    let Some(root) = graph.find(&path).unwrap().cloned() else {
                        tracing::error!("root node does not exist");
                        return Err(Error::DoesNotExist);
                    };

                    graph.remove(&root).map_err(|err| match err {
                        graph::error::Remove::NotFound => {
                            tracing::error!("node not found");
                            Error::DoesNotExist
                        }
                        graph::error::Remove::Root => panic!(),
                    })
                }
                action::Graph::Move { from, to } => {
                    let FolderResource::Present(ref mut graph) = self.graph else {
                        tracing::trace!("graph does not exist");
                        return Err(Error::DoesNotExist);
                    };

                    graph.mv(&from, &to).map_err(|err| match err {
                        graph::error::Move::FromNotFound | graph::error::Move::ParentNotFound => {
                            tracing::trace!("parent node not found");
                            Error::DoesNotExist
                        }
                        graph::error::Move::Root
                        | graph::error::Move::InvalidPaths
                        | graph::error::Move::NameCollision => Error::InvalidTransition,
                    })
                }
            }
        }

        #[tracing::instrument(level = "trace", skip(self))]
        fn try_reduce_container(
            &mut self,
            path: PathBuf,
            action: action::Container,
        ) -> std::result::Result<(), Error> {
            let FolderResource::Present(graph) = &self.graph else {
                tracing::trace!("graph not present");
                return Err(Error::DoesNotExist);
            };

            let Some(container) = graph.find(&path).unwrap() else {
                tracing::trace!("container not found");
                return Err(Error::DoesNotExist);
            };

            let mut container = container.lock().unwrap();
            match action {
                action::Container::SetName(name) => {
                    container.name = name;
                }
                action::Container::SetProperties(properties) => {
                    container.properties = properties;
                }
                action::Container::SetSettings(settings) => {
                    container.settings = settings;
                }
                action::Container::SetAssets(assets) => {
                    container.assets = assets;
                }
                action::Container::RemoveConfig => {
                    container.properties = DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound));
                    container.settings = DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound));
                    container.assets = DataResource::Err(IoSerde::Io(io::ErrorKind::NotFound));
                }
                action::Container::Asset { rid, action } => {
                    let Ok(assets) = &mut container.assets else {
                        tracing::error!("container assets do not exist");
                        return Err(Error::DoesNotExist);
                    };

                    let Some(asset) = assets.iter_mut().find(|asset| asset.rid() == &rid) else {
                        tracing::error!("asset does not exist");
                        return Err(Error::DoesNotExist);
                    };

                    match action {
                        action::Asset::SetPresent => asset.fs_resource = FileResource::Present,
                        action::Asset::SetAbsent => asset.fs_resource = FileResource::Absent,
                        action::Asset::SetState(state) => *asset = state,
                    }
                }
            }

            Ok(())
        }
    }
}

pub mod analysis {
    use super::FileResource;
    use crate::state::Analysis;
    use std::path::Path;
    use syre_local::types::AnalysisKind;

    impl Analysis {
        pub fn present(properties: AnalysisKind) -> Self {
            Self {
                properties,
                fs_resource: FileResource::Present,
            }
        }

        pub fn absent(properties: AnalysisKind) -> Self {
            Self {
                properties,
                fs_resource: FileResource::Absent,
            }
        }
    }

    impl Analysis {
        pub fn set_present(&mut self) {
            self.fs_resource = FileResource::Present;
        }

        pub fn set_absent(&mut self) {
            self.fs_resource = FileResource::Absent;
        }
    }

    /// Find an analysis by its path.
    ///
    /// # Arguments
    /// + `path`: Needle. Should be a relative path.
    /// + `analyses`: Haystack.
    pub fn find_analysis_by_path(
        path: impl AsRef<Path>,
        analyses: &Vec<Analysis>,
    ) -> Option<&Analysis> {
        let path = path.as_ref();
        analyses
            .iter()
            .find(|analysis| match analysis.properties() {
                AnalysisKind::Script(script) => path == script.path,
                AnalysisKind::ExcelTemplate(template) => path == template.template.path,
            })
    }

    /// Find an analysis by its path.
    ///
    /// # Arguments
    /// + `path`: Needle. Should be a relative path.
    /// + `analyses`: Haystack.
    pub fn find_analysis_by_path_mut(
        path: impl AsRef<Path>,
        analyses: &mut Vec<Analysis>,
    ) -> Option<&mut Analysis> {
        let path = path.as_ref();
        analyses
            .iter_mut()
            .find(|analysis| match analysis.properties() {
                AnalysisKind::Script(script) => path == script.path,
                AnalysisKind::ExcelTemplate(template) => path == template.template.path,
            })
    }
}

mod container {
    use super::FileResource;
    use crate::state::{Asset, Container};
    use std::{io, path::Path};
    use syre_core::project::Asset as CoreAsset;
    use syre_local::loader::container::Loader;

    impl Container {
        /// # Errors
        /// + If the path is invalid.
        pub fn load(path: impl AsRef<Path>) -> Result<Self, io::ErrorKind> {
            let path = path.as_ref();
            let Some(name) = path.file_name() else {
                return Err(io::ErrorKind::InvalidFilename);
            };

            let syre_local::loader::container::State {
                properties,
                settings,
                assets,
            } = Loader::load_resources(path);

            let assets = assets.map(|assets| {
                assets
                    .into_iter()
                    .map(|asset| {
                        let fs_resource = match path.join(&asset.path).exists() {
                            true => FileResource::Present,
                            false => FileResource::Absent,
                        };

                        Asset {
                            properties: asset,
                            fs_resource,
                        }
                    })
                    .collect()
            });

            Ok(Self {
                name: name.to_os_string(),
                properties,
                settings,
                assets,
            })
        }
    }

    impl Asset {
        pub fn present(asset: CoreAsset) -> Self {
            Self {
                properties: asset,
                fs_resource: FileResource::Present,
            }
        }

        pub fn absent(asset: CoreAsset) -> Self {
            Self {
                properties: asset,
                fs_resource: FileResource::Absent,
            }
        }
    }
}

pub mod graph {
    use crate::{
        common,
        state::{Container, Graph},
    };
    use rayon::prelude::*;
    use std::{
        fs, io,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };
    use syre_core::types::ResourceId;
    use syre_local as local;

    pub type Node = Arc<Mutex<Container>>;
    pub type EdgeMap = Vec<(Node, Vec<Node>)>;

    #[derive(Clone, Debug)]
    pub struct State {
        nodes: Vec<Node>,

        root: Node,

        /// Child-parent relations.
        parents: Vec<(Node, Node)>,

        /// Parent-children relations.
        children: EdgeMap,
    }

    impl State {
        pub fn new(root: Container) -> Self {
            let root = Arc::new(Mutex::new(root));
            Self {
                nodes: vec![root.clone()],
                root: root.clone(),
                parents: vec![],
                children: vec![(root, vec![])],
            }
        }

        /// # Errors
        /// + If `path` is not a directory.
        pub fn load(path: impl AsRef<Path>) -> Result<Self, io::ErrorKind> {
            let path = path.as_ref();
            if !path.exists() {
                return Err(io::ErrorKind::NotFound);
            }
            if !path.is_dir() {
                return Err(io::ErrorKind::NotADirectory);
            }

            Ok(rayon::ThreadPoolBuilder::new()
                .build()
                .unwrap()
                .install(move || Self::load_tree(path)))
        }

        /// Recursive loader.
        ///
        /// # Panics
        /// + If the path is invalid.
        fn load_tree(path: impl AsRef<Path>) -> Self {
            assert!(path.as_ref().is_dir());

            let root = Container::load(&path).unwrap();
            let mut graph = Self::new(root);

            let dir_walker = local::common::ignore::WalkBuilder::new(&path)
                .max_depth(Some(1))
                .filter_entry(|entry| {
                    entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false)
                        && entry.file_name() != local::common::app_dir()
                })
                .build();
            let children = dir_walker
                .into_iter()
                .skip(1)
                .filter_map(|entry| entry.ok())
                .collect::<Vec<_>>()
                .into_par_iter()
                .map(|entry| Self::load_tree(entry.path()))
                .collect::<Vec<_>>();

            let root = graph.root().clone();
            for child in children {
                graph.insert(&root, child).unwrap();
            }

            graph
        }
    }

    impl State {
        pub fn nodes(&self) -> &Vec<Node> {
            &self.nodes
        }

        pub fn root(&self) -> &Node {
            &self.root
        }

        /// Get the absolute path to the container from the root node.
        /// i.e. The root node has path `/`.
        pub fn path(&self, target: &Node) -> Option<PathBuf> {
            const SEPARATOR: &str = "/";

            let ancestors = self.ancestors(target);
            if ancestors.is_empty() {
                return None;
            }

            let path = ancestors
                .iter()
                .rev()
                .skip(1)
                .map(|ancestor| {
                    let ancestor = ancestor.lock().unwrap();
                    ancestor.name().to_string_lossy().to_string()
                })
                .collect::<Vec<_>>()
                .join(SEPARATOR);

            Some(PathBuf::from(SEPARATOR).join(path))
        }

        /// Insert a subtree into the graph as a child of the given parent.
        pub fn insert(&mut self, parent: &Node, graph: Self) -> Result<(), error::Insert> {
            let Self {
                nodes,
                root,
                children,
                parents,
            } = graph;

            if self
                .nodes
                .iter()
                .find(|node| Arc::ptr_eq(node, parent))
                .is_none()
            {
                return Err(error::Insert::ParentNotFound);
            };

            let root_container = root.lock().unwrap();
            for child in self.children(parent).unwrap() {
                let container = child.lock().unwrap();
                if container.name() == root_container.name() {
                    return Err(error::Insert::NameCollision);
                }
            }
            drop(root_container);

            self.nodes.extend(nodes);
            self.children.extend(children);
            self.parents.extend(parents);
            self.parents.push((root.clone(), parent.clone()));
            self.children_mut(&parent).unwrap().push(root);
            Ok(())
        }

        /// Returns the children for the given node
        /// if the node exists in the graph.
        pub fn children(&self, parent: &Node) -> Option<&Vec<Node>> {
            self.children.iter().find_map(|(p, children)| {
                if Arc::ptr_eq(p, parent) {
                    Some(children)
                } else {
                    None
                }
            })
        }

        /// Returns the children for the given node
        /// if the node exists in the graph.
        fn children_mut(&mut self, parent: &Node) -> Option<&mut Vec<Node>> {
            self.children.iter_mut().find_map(|(p, children)| {
                if Arc::ptr_eq(p, parent) {
                    Some(children)
                } else {
                    None
                }
            })
        }

        /// Retrieve all descendants of the root node,
        /// including self.
        ///
        /// # Returns
        /// `None` if the root node is not found in the graph.
        /// Parents come before  their children.
        /// The root node is at index 0.
        pub fn descendants(&self, root: &Node) -> Option<Vec<Node>> {
            if !self.nodes.iter().any(|node| Arc::ptr_eq(node, root)) {
                return None;
            }

            let mut descendants = vec![root.clone()];
            for child in self.children(root).unwrap() {
                descendants.extend(self.descendants(child).unwrap());
            }

            Some(descendants)
        }

        /// Returns the given node's parent if the node exists
        /// in the graph and has a parent (i.e. Is not the root node).
        pub fn parent(&self, child: &Node) -> Option<&Node> {
            self.parents.iter().find_map(|(c, parent)| {
                if Arc::ptr_eq(c, child) {
                    Some(parent)
                } else {
                    None
                }
            })
        }

        /// # Returns
        /// List of ancestors, in order, starting with the given node until the root.
        /// If the given node is not in the graph, an empty `Vec` is returned.
        pub fn ancestors(&self, root: &Node) -> Vec<Node> {
            if Arc::ptr_eq(&self.root, root) {
                return vec![root.clone()];
            }

            let Some(parent) = self.parent(root) else {
                return vec![];
            };

            let mut ancestors = self.ancestors(parent);
            ancestors.insert(0, root.clone());
            ancestors
        }

        /// Find a container by its path.
        ///
        /// # Arguments
        /// 1. `path`: Absolute path to container, with the
        /// project's data root being the root path.
        ///
        /// # Returns
        /// `Err` if path is not absolute or if any special path components are used.
        /// This includes path prefixes, current dir, and parent dir.
        pub fn find(
            &self,
            path: impl AsRef<Path>,
        ) -> Result<Option<&Node>, crate::error::InvalidPath> {
            let path = path.as_ref();
            if !common::is_root_path(path) {
                return Err(crate::error::InvalidPath);
            }

            let mut node = &self.root;
            for component in path.components().skip(1) {
                match component {
                    std::path::Component::Prefix(_)
                    | std::path::Component::RootDir
                    | std::path::Component::CurDir
                    | std::path::Component::ParentDir => {
                        return Err(crate::error::InvalidPath);
                    }

                    std::path::Component::Normal(name) => {
                        let Some(child) = self.children(node).unwrap().iter().find(|child| {
                            let child = child.lock().unwrap();
                            child.name() == name
                        }) else {
                            return Ok(None);
                        };

                        node = child;
                    }
                }
            }

            Ok(Some(node))
        }

        /// Find a container by its id.
        pub fn find_by_id(&self, container: &ResourceId) -> Option<&Node> {
            self.nodes.iter().find(|node| {
                let container_state = node.lock().unwrap();
                let crate::state::DataResource::Ok(rid) = container_state.rid() else {
                    return false;
                };

                rid == container
            })
        }

        /// Remove a subgraph.
        ///
        /// # Arguments
        /// 1. `root`: Root of the subgraph to remove.
        pub fn remove(&mut self, root: &Node) -> Result<(), error::Remove> {
            let Some(descendants) = self.descendants(root) else {
                return Err(error::Remove::NotFound);
            };

            let Some(parent) = self.parent(root).cloned() else {
                assert!(Node::ptr_eq(root, &self.root));
                return Err(error::Remove::Root);
            };

            self.children_mut(&parent)
                .unwrap()
                .retain(|child| !Node::ptr_eq(child, root));

            self.children.retain(|(parent, _)| {
                !descendants
                    .iter()
                    .any(|descendant| Node::ptr_eq(parent, descendant))
            });

            self.parents.retain(|(child, _)| {
                !descendants
                    .iter()
                    .any(|descendant| Node::ptr_eq(child, descendant))
            });

            self.nodes.retain(|node| {
                !descendants
                    .iter()
                    .any(|descendant| Node::ptr_eq(node, descendant))
            });

            Ok(())
        }

        /// Move a subgraph to a new parent.
        ///
        /// # Arguments
        /// 1. `from`: Absolute path of the node to move.
        /// 2. `to`: Absolute path of the new location.
        pub fn mv(
            &mut self,
            from: impl AsRef<Path>,
            to: impl AsRef<Path>,
        ) -> Result<(), error::Move> {
            let from_path = from.as_ref();
            let to_path = to.as_ref();
            if !(from_path.is_absolute() && to_path.is_absolute()) {
                return Err(error::Move::InvalidPaths);
            }

            let root_path = Path::new(std::path::Component::RootDir.as_os_str());
            if from_path == root_path || to_path == root_path {
                return Err(error::Move::Root);
            }

            if to_path.starts_with(from_path) {
                return Err(error::Move::InvalidPaths);
            }

            let Some(from_node) = self.find(from_path).unwrap().cloned() else {
                return Err(error::Move::FromNotFound);
            };

            let Some(to_parent) = self.find(to_path.parent().unwrap()).unwrap().cloned() else {
                return Err(error::Move::ParentNotFound);
            };

            let from_parent = self.parent(&from_node).unwrap().clone();
            if Arc::ptr_eq(&from_parent, &to_parent) {
                return Ok(());
            }

            self.children_mut(&from_parent)
                .unwrap()
                .retain(|child| !Arc::ptr_eq(child, &from_node));
            self.children_mut(&to_parent)
                .unwrap()
                .push(from_node.clone());

            self.parents
                .retain(|(child, _)| !Arc::ptr_eq(child, &from_node));
            self.parents.push((from_node.clone(), to_parent));

            let mut container = from_node.lock().unwrap();
            container.name = to_path.file_name().unwrap().to_os_string();
            Ok(())
        }
    }

    impl State {
        /// Converts the graph to a [`crate::state::Graph`].
        pub fn as_graph(&self) -> Graph {
            assert!(Arc::ptr_eq(&self.nodes[0], &self.root));

            let nodes = self
                .nodes
                .iter()
                .map(|node| {
                    let container = node.lock().unwrap();
                    (*container).clone()
                })
                .collect();

            let children = self
                .nodes
                .iter()
                .map(|parent| {
                    self.children(parent)
                        .unwrap()
                        .iter()
                        .map(|child| {
                            self.nodes
                                .iter()
                                .position(|node| Arc::ptr_eq(node, child))
                                .unwrap()
                        })
                        .collect()
                })
                .collect();

            Graph { nodes, children }
        }

        /// Converts a subgraph to a [`crate::state::Graph`].
        pub fn subgraph_as_graph(&self, root: impl AsRef<Path>) -> Result<Graph, error::NotFound> {
            let Some(root) = self.find(root).unwrap() else {
                return Err(error::NotFound);
            };

            let nodes = self.descendants(root).unwrap();
            let (nodes, children) = nodes
                .iter()
                .map(|node| {
                    let children = self
                        .children(node)
                        .unwrap()
                        .iter()
                        .map(|child| {
                            nodes
                                .iter()
                                .position(|node| Arc::ptr_eq(node, child))
                                .unwrap()
                        })
                        .collect();

                    let container = node.lock().unwrap();
                    ((*container).clone(), children)
                })
                .unzip();

            Ok(Graph { nodes, children })
        }
    }

    pub mod error {
        #[derive(Debug)]
        pub enum Insert {
            ParentNotFound,
            NameCollision,
        }

        #[derive(Debug)]
        pub enum Remove {
            /// The node was not in the graph.
            NotFound,

            /// The node was the root node.
            Root,
        }

        #[derive(Debug)]
        pub enum Move {
            /// Root node can not be moved.
            Root,
            FromNotFound,

            /// The new parent was not found.
            ParentNotFound,

            /// The paths are invalid relative to each other.
            /// e.g. If to is a child of from.
            InvalidPaths,
            NameCollision,
        }

        #[derive(Debug)]
        pub struct NotFound;
    }
}

pub(crate) mod action {
    use super::{graph, project::State as Project, DataResource, FolderResource};
    use crate::state;
    use std::{ffi::OsString, path::PathBuf};
    use syre_core::{project::Project as CoreProject, types::ResourceId};
    use syre_local::project::config::{ContainerSettings, Settings, StoredContainerProperties};

    #[derive(Debug, derive_more::From)]
    pub enum Action {
        /// Sets the project's path.
        SetPath(PathBuf),

        /// Sets the project's base folder to be `Absent`.
        RemoveFolder,

        /// Sets the project's base folder to be `Present` with the given state.
        CreateFolder(Project),

        /// Sets all config resources to be absent.
        RemoveConfig,

        SetProperties(DataResource<CoreProject>),
        SetSettings(DataResource<Settings>),
        SetAnalyses(DataResource<Vec<state::Analysis>>),

        /// Sets all analyses' file system resource to be absent.
        /// Used if the project's analysis directory is removed.
        SetAnalysesAbsent,

        #[from]
        Graph(Graph),

        Container {
            /// Absolute path to the container from the graph root.
            /// Root path indicates the graph root.
            path: PathBuf,
            action: Container,
        },
    }

    #[derive(Debug)]
    pub enum Graph {
        /// Sets the state of the graph.
        Set(FolderResource<graph::State>),

        /// Insert a subgraph.
        Insert {
            /// Absoulte path from the project's data root to the parent container.
            /// i.e. The root path represents the data root container.
            parent: PathBuf,
            graph: graph::State,
        },

        /// Remove a subgraph with root node at the given path.
        ///
        /// # Panics
        /// + If the given path does not exist in the graph.
        /// + If the path is the root path. (See [`Graph::Set`])
        Remove(
            /// Absoulte path from the project's data root to the subgraph root.
            /// i.e. The root path represents the data root container.
            PathBuf,
        ),

        /// Move a subgraph.
        ///
        /// # Fields
        /// Paths are absolute from the graph root to the container.
        Move { from: PathBuf, to: PathBuf },
    }

    #[derive(Debug)]
    pub enum Container {
        SetName(OsString),
        SetProperties(DataResource<StoredContainerProperties>),
        SetSettings(DataResource<ContainerSettings>),
        SetAssets(DataResource<Vec<state::Asset>>),

        /// Sets all config resources to be absent.
        RemoveConfig,

        Asset {
            rid: ResourceId,
            action: Asset,
        },
    }

    #[derive(Debug)]
    pub enum Asset {
        SetPresent,
        SetAbsent,
        SetState(state::Asset),
    }
}

impl<T> FolderResource<T> {
    pub fn as_mut(&mut self) -> FolderResource<&mut T> {
        match *self {
            Self::Present(ref mut x) => FolderResource::Present(x),
            Self::Absent => FolderResource::Absent,
        }
    }
}
