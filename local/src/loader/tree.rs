use crate::{common, project::project, project::resources::Container};
use rayon::prelude::*;
use std::{io, path::Path, sync::Arc};
use syre_core::graph::{ResourceNode, ResourceTree};

type ContainerTree = ResourceTree<Container>;

pub struct Loader {}
impl Loader {
    /// Load a `Container` tree into a [`ResourceTree`].
    pub fn load(path: impl AsRef<Path>) -> Result<ContainerTree, error::Error> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(error::Error::Root(io::ErrorKind::NotFound));
        }
        if !path.is_dir() {
            return Err(error::Error::Root(io::ErrorKind::NotADirectory));
        }

        // TODO: Allow recursive ignore files
        let ignore = project::project_root_path(&path)
            .map(|project_path| {
                let ignore_path = common::ignore_file_of(&project_path);
                let mut ignore = ignore::gitignore::GitignoreBuilder::new(&project_path);
                if let Some(err) = ignore.add(&ignore_path) {
                    return Err((ignore_path, err));
                };
                ignore.build().map_err(|err| (ignore_path, err))
            })
            .transpose()
            .map_err(|(path, err)| error::Error::Ignore { path, err })?;

        let state = rayon::ThreadPoolBuilder::new()
            .build()
            .unwrap()
            .install(move || Self::load_tree(path, ignore.as_ref()));

        if state.is_ok() {
            let (nodes, edges, _) = state.to_parts();
            let edges = edges
                .into_iter()
                .map(|(parent, children)| {
                    let parent = parent.lock().unwrap();
                    let parent = parent.as_ref().unwrap().rid().clone();

                    let children = children
                        .into_iter()
                        .map(|child| {
                            let child = child.lock().unwrap();
                            child.as_ref().unwrap().rid().clone()
                        })
                        .collect();

                    (parent, children)
                })
                .collect();

            let nodes = nodes
                .into_iter()
                .map(|node| {
                    let container = Arc::into_inner(node)
                        .unwrap()
                        .into_inner()
                        .unwrap()
                        .unwrap();

                    let container = ResourceNode::new(container);
                    (container.rid().clone(), container)
                })
                .collect();

            Ok(ContainerTree::from_parts(nodes, edges).unwrap())
        } else {
            Err(error::Error::State(state))
        }
    }

    /// Recursive loader.
    fn load_tree(
        path: impl AsRef<Path>,
        ignore: Option<&ignore::gitignore::Gitignore>,
    ) -> state::Tree {
        let path = path.as_ref();
        let root = state::Container::load(path);
        let mut graph = state::Tree::new(root);
        let children = walkdir::WalkDir::new(path)
            .into_iter()
            .skip(1)
            .filter_map(|entry| {
                let Ok(entry) = entry else {
                    return None;
                };

                if !entry.file_type().is_dir()
                    || entry.file_name() == common::app_dir()
                    || entry
                        .file_name()
                        .to_str()
                        .map(|name| name.starts_with("."))
                        .unwrap_or(false)
                {
                    return None;
                }

                if ignore
                    .map(|ignore| ignore.matched(entry.path(), true).is_ignore())
                    .unwrap_or(false)
                {
                    return None;
                }

                Some(entry)
            })
            .collect::<Vec<_>>()
            .into_par_iter()
            .map(|entry| Self::load_tree(entry.path(), ignore))
            .collect::<Vec<_>>();

        let root = graph.root().clone();
        for child in children {
            graph.insert(&root, child);
        }

        graph
    }
}

mod state {
    use super::super::container;
    use crate::{
        error::IoSerde,
        project::{
            config::{ContainerSettings, StoredContainerProperties},
            resources::Container as LocalContainer,
        },
    };
    use std::{
        fs, io,
        path::PathBuf,
        sync::{Arc, Mutex},
    };
    use syre_core::project::Asset;

    pub type Node = Arc<Mutex<Result<LocalContainer, Container>>>;
    pub type EdgeMap = Vec<(Node, Vec<Node>)>;

    #[derive(Debug)]
    pub struct Container {
        path: PathBuf,

        /// Container data.
        /// If `Err` indicates an issue with the root folder.
        data: Result<ContainerData, io::ErrorKind>,
    }

    impl Container {
        pub fn new(path: impl Into<PathBuf>, data: Result<ContainerData, io::ErrorKind>) -> Self {
            Self {
                path: path.into(),
                data,
            }
        }

        /// # Notes
        /// + Canonicalizes path.
        pub fn load(path: impl Into<PathBuf>) -> Result<LocalContainer, Self> {
            let path = path.into();
            let path = match fs::canonicalize(&path) {
                Ok(path) => path,
                Err(err) => {
                    return Err(Self {
                        path,
                        data: Err(err.kind()),
                    })
                }
            };
            if !path.is_dir() {
                return Err(Self {
                    path,
                    data: Err(io::ErrorKind::NotADirectory),
                });
            }

            container::Loader::load(&path).map_err(|state| Self {
                path: path.into(),
                data: Ok(ContainerData {
                    properties: state.properties,
                    settings: state.settings,
                    assets: state.assets,
                }),
            })
        }

        pub fn path(&self) -> &PathBuf {
            &self.path
        }

        pub fn data(&self) -> &Result<ContainerData, io::ErrorKind> {
            &self.data
        }
    }

    #[derive(Debug)]
    pub struct ContainerData {
        properties: Result<StoredContainerProperties, IoSerde>,
        settings: Result<ContainerSettings, IoSerde>,
        assets: Result<Vec<Asset>, IoSerde>,
    }

    impl ContainerData {
        pub fn is_ok(&self) -> bool {
            self.properties.is_ok() && self.settings.is_ok() && self.assets.is_ok()
        }
    }

    #[derive(Debug)]
    pub struct Tree {
        nodes: Vec<Node>,

        root: Node,

        /// Parent-children relations.
        children: EdgeMap,

        /// Child-parent realtions.
        parents: Vec<(Node, Node)>,
    }

    impl Tree {
        pub fn new(root: Result<LocalContainer, Container>) -> Self {
            let root = Arc::new(Mutex::new(root));
            Self {
                nodes: vec![root.clone()],
                root: root.clone(),
                children: vec![(root, vec![])],
                parents: vec![],
            }
        }

        pub fn root(&self) -> &Node {
            &self.root
        }

        /// # Panics
        /// If the parent does not exist in the graph.
        pub fn insert(&mut self, parent: &Node, graph: Self) {
            let Self {
                nodes,
                root,
                children,
                parents,
            } = graph;

            let parent = self
                .nodes
                .iter()
                .find(|node| Arc::ptr_eq(node, parent))
                .unwrap()
                .clone();

            self.nodes.extend(nodes);
            self.children.extend(children);
            self.parents.extend(parents);
            self.parents.push((root.clone(), parent.clone()));
            self.children
                .iter_mut()
                .find_map(|(p, children)| {
                    if Arc::ptr_eq(p, &parent) {
                        Some(children)
                    } else {
                        None
                    }
                })
                .unwrap()
                .push(root);
        }

        /// Returns `true` if all states are `Ok`,
        /// else `false`.
        pub fn is_ok(&self) -> bool {
            self.nodes.iter().any(|node| node.lock().is_ok())
        }

        /// Breaks graph into (nodes, edges, root)
        pub fn to_parts(self) -> (Vec<Node>, EdgeMap, Node) {
            let Self {
                nodes,
                root,
                children,
                parents: _,
            } = self;

            (nodes, children, root)
        }
    }
}

pub mod error {
    use super::state::Tree;
    use std::{io, path::PathBuf};

    #[derive(Debug)]
    pub enum Error {
        /// The tree's root resource could not be accessed.
        Root(io::ErrorKind),

        /// The tree could not be loaded normally.
        State(Tree),

        /// An ignore file could not be read correctly.
        Ignore { path: PathBuf, err: ignore::Error },
    }
}

#[cfg(test)]
#[path = "./tree_test.rs"]
mod tree_test;
