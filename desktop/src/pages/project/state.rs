pub use container::{AnalysisAssociation, Asset, State as Container};
pub use display::State as Display;
pub use flags::State as Flags;
pub use graph::State as Graph;
pub use metadata::Metadata;
pub use project::{Analysis, State as Project};
pub use workspace::State as Workspace;
pub use workspace_graph::State as WorkspaceGraph;

pub mod workspace {
    use leptos::prelude::*;

    #[derive(Clone)]
    pub struct State {
        preview: RwSignal<Preview>,
    }

    impl State {
        pub fn new() -> Self {
            Self {
                preview: RwSignal::new(Preview::default()),
            }
        }

        pub fn preview(&self) -> &RwSignal<Preview> {
            &self.preview
        }
    }

    #[derive(Clone)]
    pub struct Preview {
        pub assets: bool,
        pub analyses: bool,
        pub kind: bool,
        pub description: bool,
        pub tags: bool,
        pub metadata: bool,
    }

    impl Preview {
        /// Set all previews to `false`.
        pub fn clear(&mut self) {
            self.assets = false;
            self.analyses = false;
            self.kind = false;
            self.description = false;
            self.tags = false;
            self.metadata = false;
        }

        /// How many previews are active.
        pub fn length(&self) -> usize {
            let mut length = 0;
            if self.assets {
                length += 1;
            }
            if self.analyses {
                length += 1;
            }
            if self.kind {
                length += 1;
            }
            if self.description {
                length += 1;
            }
            if self.tags {
                length += 1;
            }
            if self.metadata {
                length += 1;
            }

            length
        }
    }

    impl Default for Preview {
        fn default() -> Self {
            Self {
                assets: true,
                analyses: false,
                kind: false,
                description: false,
                tags: false,
                metadata: false,
            }
        }
    }
}

pub mod workspace_graph {
    use leptos::prelude::*;
    use std::sync::Arc;
    use syre_core::types::ResourceId;
    use syre_local_database as db;

    pub type ContainerVisibility = Vec<(super::graph::Node, ArcRwSignal<bool>)>;

    #[derive(Clone, Debug)]
    pub struct State {
        /// All selection resources.
        selection_resources: SelectionResources,
        container_visibility: RwSignal<ContainerVisibility>,
    }

    impl State {
        pub fn new(graph: &super::graph::State) -> Self {
            let selection_resources = graph.nodes().with_untracked(|nodes| {
                nodes
                    .iter()
                    .flat_map(|node| {
                        let mut resources = vec![];
                        node.properties().with_untracked(|properties| {
                            if let db::state::DataResource::Ok(properties) = properties {
                                resources.push(ResourceSelection::new(
                                    properties.rid().read_only(),
                                    ResourceKind::Container,
                                ));
                            }
                        });

                        node.assets().with_untracked(|assets| {
                            if let db::state::DataResource::Ok(assets) = assets {
                                assets.with_untracked(|assets| {
                                    let assets = assets.iter().map(|asset| {
                                        ResourceSelection::new(
                                            asset.rid().read_only(),
                                            ResourceKind::Asset,
                                        )
                                    });
                                    resources.extend(assets);
                                })
                            }
                        });

                        resources
                    })
                    .collect::<Vec<_>>()
            });

            let container_visibility = graph.nodes().with_untracked(|nodes| {
                nodes
                    .iter()
                    .cloned()
                    .map(|node| (node, ArcRwSignal::new(true)))
                    .collect()
            });

            Self {
                selection_resources: SelectionResources::new(selection_resources),
                container_visibility: RwSignal::new(container_visibility),
            }
        }

        pub fn selection_resources(&self) -> &SelectionResources {
            &self.selection_resources
        }

        pub fn container_visiblity(&self) -> &RwSignal<ContainerVisibility> {
            &self.container_visibility
        }

        /// Get the visibility signal for a specific container.
        pub fn container_visibility_get(
            &self,
            container: &super::graph::Node,
        ) -> Option<ArcRwSignal<bool>> {
            self.container_visibility.with_untracked(|containers| {
                containers.iter().find_map(|(node, visibility)| {
                    Arc::ptr_eq(node, container).then_some(visibility.clone())
                })
            })
        }

        pub fn container_visibility_show_all(&self) {
            self.container_visibility.with_untracked(|visibilities| {
                visibilities.iter().for_each(|(_, visibility)| {
                    if !visibility.get_untracked() {
                        visibility.set(true);
                    }
                })
            });
        }
    }

    #[derive(Debug, Clone)]
    pub struct SelectionResources {
        resources: RwSignal<Vec<ResourceSelection>>,
        selected: RwSignal<Vec<Resource>>,
    }

    impl SelectionResources {
        pub fn new(resources: Vec<ResourceSelection>) -> Self {
            let selected = resources
                .iter()
                .filter_map(|resource| {
                    resource.selected.get_untracked().then_some(Resource {
                        rid: resource.rid.clone(),
                        kind: resource.kind,
                    })
                })
                .collect();

            Self {
                resources: RwSignal::new(resources),
                selected: RwSignal::new(selected),
            }
        }

        pub fn selected(&self) -> ReadSignal<Vec<Resource>> {
            self.selected.read_only()
        }

        /// Get a resources selection state.
        pub fn get(&self, rid: &ResourceId) -> Option<ArcReadSignal<bool>> {
            self.resources.with_untracked(|resources| {
                resources.iter().find_map(|resource| {
                    resource
                        .rid
                        .with_untracked(|resource_id| resource_id == rid)
                        .then_some(resource.selected.read_only())
                })
            })
        }

        /// Set whether a resource is selected.
        ///
        /// # Returns
        /// `Err` if a resource with the given id is not found.
        pub fn set(&self, rid: &ResourceId, selected: bool) -> Result<(), ()> {
            self.resources
                .with_untracked(|resources| {
                    resources
                        .iter()
                        .find(|resource| {
                            resource
                                .rid
                                .with_untracked(|resource_id| resource_id == rid)
                        })
                        .map(|resource| {
                            if resource
                                .selected
                                .with_untracked(|resource| *resource != selected)
                            {
                                resource.selected.set(selected);
                                self.selected.update(|resources| {
                                    if selected {
                                        resources.push(Resource {
                                            rid: resource.rid,
                                            kind: resource.kind,
                                        })
                                    } else {
                                        resources.retain(|selected| {
                                            selected.rid.with_untracked(|selected| selected != rid)
                                        })
                                    }
                                })
                            }
                        })
                })
                .ok_or(())
        }

        /// Set all resources to be unselected.
        pub fn clear(&self) {
            self.resources.with_untracked(|resources| {
                resources.iter().for_each(|resource| {
                    if resource.selected.get_untracked() {
                        resource.selected.set(false);
                    }
                });
            });

            self.selected.update(|selected| selected.clear());
        }

        /// Set the selection to be only the given resource.
        ///
        /// # Returns
        /// `Err` if a resource with the given id is not found.
        /// REgardless of whether the resource is found or not, all other resources are deselected.
        pub fn select_only(&self, rid: &ResourceId) -> Result<(), ()> {
            let resource = self
                .resources
                .with_untracked(|resources| {
                    let mut found_resource = None;
                    resources.iter().for_each(|resource| {
                        let is_selected = resource.selected.get_untracked();
                        resource.rid.with_untracked(|resource_id| {
                            if resource_id != rid && is_selected {
                                resource.selected.set(false);
                            } else if resource_id == rid {
                                if !is_selected {
                                    resource.selected.set(true);
                                }

                                let _ = found_resource.insert(resource);
                            }
                        })
                    });

                    found_resource.cloned()
                })
                .ok_or(())?;

            self.selected.update(|selected| {
                selected.retain(|selected| selected.rid.with_untracked(|selected| selected == rid));
                if selected.len() != 1 {
                    assert!(selected.is_empty());

                    selected.push(Resource {
                        rid: resource.rid,
                        kind: resource.kind,
                    })
                }
            });

            Ok(())
        }
    }

    impl SelectionResources {
        pub fn push(&self, resource: ResourceSelection) {
            let selected = resource.selected.get_untracked().then_some(Resource {
                rid: resource.rid,
                kind: resource.kind,
            });

            self.resources.update(|resources| resources.push(resource));
            if let Some(selected) = selected {
                self.selected.update(|resources| resources.push(selected));
            }
        }

        pub fn extend(&self, resources: Vec<ResourceSelection>) {
            if resources.is_empty() {
                return;
            }

            let selected = resources
                .iter()
                .filter_map(|resource| {
                    resource.selected.get_untracked().then_some(Resource {
                        rid: resource.rid,
                        kind: resource.kind,
                    })
                })
                .collect::<Vec<_>>();

            self.resources
                .update(|selection| selection.extend(resources));

            if !selected.is_empty() {
                self.selected.update(|resources| resources.extend(selected));
            }
        }

        pub fn remove(&self, rids: &Vec<ResourceId>) {
            if rids.is_empty() {
                return;
            }

            let selected = self.selected.with_untracked(|selected| {
                rids.iter()
                    .filter(|&rid| {
                        selected
                            .iter()
                            .any(|selected| selected.rid.with_untracked(|selected| selected == rid))
                    })
                    .collect::<Vec<_>>()
            });

            if !selected.is_empty() {
                self.selected.update(|resources| {
                    resources.retain(|resource| {
                        resource
                            .rid
                            .with_untracked(|resource| !selected.contains(&resource))
                    })
                });
            }

            self.resources.update(|resources| {
                resources.retain(|resource| resource.rid.with_untracked(|rid| !rids.contains(rid)));
            });
        }
    }

    #[derive(Clone, Debug)]
    pub struct Resource {
        rid: ReadSignal<ResourceId>,
        kind: ResourceKind,
    }

    impl Resource {
        pub fn new(rid: ReadSignal<ResourceId>, kind: ResourceKind) -> Self {
            Self { rid, kind }
        }

        pub fn rid(&self) -> &ReadSignal<ResourceId> {
            &self.rid
        }

        pub fn kind(&self) -> &ResourceKind {
            &self.kind
        }
    }

    #[derive(Clone, Debug)]
    pub struct ResourceSelection {
        rid: ReadSignal<ResourceId>,
        kind: ResourceKind,
        selected: ArcRwSignal<bool>,
    }

    impl ResourceSelection {
        pub fn new(rid: ReadSignal<ResourceId>, kind: ResourceKind) -> Self {
            Self {
                rid,
                kind,
                selected: ArcRwSignal::new(false),
            }
        }
    }

    #[derive(PartialEq, Clone, Copy, Debug)]
    pub enum ResourceKind {
        Container,
        Asset,
    }
}

pub mod project {
    use chrono::{DateTime, Utc};
    use leptos::prelude::*;
    use std::path::PathBuf;
    use syre_core as core;
    use syre_core::{
        project::Project as CoreProject,
        types::{ResourceId, ResourceMap, UserId, UserPermissions},
    };
    use syre_local::{project::config::Settings as ProjectSettings, types::AnalysisKind};
    use syre_local_database as db;

    pub type AnalysesState = db::state::DataResource<RwSignal<Vec<Analysis>>>;

    #[derive(Clone)]
    pub struct State {
        path: RwSignal<PathBuf>,
        rid: RwSignal<ResourceId>,
        properties: Properties,
        analyses: RwSignal<AnalysesState>,
        settings: RwSignal<db::state::DataResource<Settings>>,
    }

    impl State {
        /// # Notes
        /// Assumes `properties` is `Ok`.
        pub fn new(path: impl Into<PathBuf>, data: db::state::ProjectData) -> Self {
            let db::state::DataResource::Ok(properties) = data.properties() else {
                panic!("expected `properties` to be `Ok`");
            };

            let analyses = data.analyses().map(|analyses| {
                let analyses = analyses
                    .iter()
                    .map(|analysis| Analysis::from_state(analysis))
                    .collect();

                RwSignal::new(analyses)
            });

            Self {
                path: RwSignal::new(path.into()),
                rid: RwSignal::new(properties.rid().clone()),
                properties: Properties::new(properties.clone()),
                analyses: RwSignal::new(analyses),
                settings: RwSignal::new(
                    data.settings()
                        .map(|settings| Settings::new(settings.clone())),
                ),
            }
        }

        pub fn path(&self) -> RwSignal<PathBuf> {
            self.path.clone()
        }

        pub fn rid(&self) -> RwSignal<ResourceId> {
            self.rid.clone()
        }

        pub fn properties(&self) -> &Properties {
            &self.properties
        }

        pub fn analyses(&self) -> RwSignal<AnalysesState> {
            self.analyses.clone()
        }
    }

    impl State {
        pub fn as_properties(&self) -> core::project::Project {
            let mut properties = core::project::Project::with_id(
                self.rid.get_untracked(),
                self.properties.name.get_untracked(),
            );
            properties.description = self.properties.description.get_untracked();
            properties.data_root = self.properties.data_root.get_untracked();
            properties.analysis_root = self.properties.analysis_root.get_untracked();

            properties
        }
    }

    #[derive(Clone)]
    pub struct Properties {
        name: RwSignal<String>,
        description: RwSignal<Option<String>>,
        data_root: RwSignal<PathBuf>,
        analysis_root: RwSignal<Option<PathBuf>>,
        meta_level: RwSignal<u16>,
    }

    impl Properties {
        pub fn new(properties: CoreProject) -> Self {
            let CoreProject {
                name,
                description,
                data_root,
                analysis_root,
                meta_level,
                ..
            } = properties;

            Self {
                name: RwSignal::new(name),
                description: RwSignal::new(description),
                data_root: RwSignal::new(data_root),
                analysis_root: RwSignal::new(analysis_root),
                meta_level: RwSignal::new(meta_level),
            }
        }

        pub fn name(&self) -> RwSignal<String> {
            self.name.clone()
        }

        pub fn description(&self) -> RwSignal<Option<String>> {
            self.description.clone()
        }

        pub fn data_root(&self) -> RwSignal<PathBuf> {
            self.data_root.clone()
        }

        pub fn analysis_root(&self) -> RwSignal<Option<PathBuf>> {
            self.analysis_root.clone()
        }

        pub fn meta_level(&self) -> RwSignal<u16> {
            self.meta_level.clone()
        }
    }

    #[derive(Clone)]
    pub struct Settings {
        created: RwSignal<DateTime<Utc>>,
        creator: RwSignal<Option<UserId>>,
        permissions: RwSignal<ResourceMap<UserPermissions>>,
    }

    impl Settings {
        pub fn new(settings: ProjectSettings) -> Self {
            let ProjectSettings {
                created,
                creator,
                permissions,
                ..
            } = settings;

            Self {
                created: RwSignal::new(created),
                creator: RwSignal::new(creator),
                permissions: RwSignal::new(permissions),
            }
        }
    }

    #[derive(Clone)]
    pub struct Analysis {
        properties: RwSignal<AnalysisKind>,
        fs_resource: RwSignal<db::state::FileResource>,
    }

    impl Analysis {
        pub fn from_state(analysis: &db::state::Analysis) -> Self {
            Self {
                properties: RwSignal::new(analysis.properties().clone()),
                fs_resource: RwSignal::new(analysis.fs_resource().clone()),
            }
        }

        pub fn properties(&self) -> RwSignal<AnalysisKind> {
            self.properties.clone()
        }

        pub fn fs_resource(&self) -> RwSignal<db::state::FileResource> {
            self.fs_resource.clone()
        }

        pub fn is_present(&self) -> bool {
            self.fs_resource.read().is_present()
        }
    }
}

pub mod graph {
    use super::Container;
    use crate::common;
    use leptos::prelude::*;
    use std::{
        ffi::OsString,
        path::{Component, Path, PathBuf},
        sync::{Arc, Mutex},
    };
    use syre_core::types::ResourceId;
    use syre_local_database as db;

    pub type Node = Arc<Container>;
    pub type Children = Vec<(Node, RwSignal<Vec<Node>>)>;

    #[derive(Clone)]
    pub struct State {
        nodes: RwSignal<Vec<Node>>, // TODO: `nodes` is redundant with `children`, could be removed.
        root: Node,
        children: RwSignal<Children>, // TODO: Rename to `edges`.
        parents: Arc<Mutex<Vec<(Node, RwSignal<Node>)>>>,
    }

    impl State {
        pub fn new(graph: db::state::Graph) -> Self {
            let db::state::Graph { nodes, children } = graph;

            // TODO: Know that index 0 is root, so can skip it.
            let parents = (0..nodes.len())
                .into_iter()
                .map(|child| {
                    children
                        .iter()
                        .position(|children| children.contains(&child))
                })
                .collect::<Vec<_>>();

            let nodes = nodes
                .into_iter()
                .map(|container| Node::new(Container::new(container)))
                .collect::<Vec<_>>();

            let root = nodes[0].clone();
            let children = children
                .into_iter()
                .enumerate()
                .map(|(parent, children)| {
                    let children = children
                        .into_iter()
                        .map(|child| nodes[child].clone())
                        .collect::<Vec<_>>();

                    (nodes[parent].clone(), RwSignal::new(children))
                })
                .collect::<Vec<_>>();

            let parents = parents
                .into_iter()
                .enumerate()
                .filter_map(|(child, parent)| {
                    parent
                        .map(|parent| (nodes[child].clone(), RwSignal::new(nodes[parent].clone())))
                })
                .collect();

            Self {
                nodes: RwSignal::new(nodes),
                root,
                children: RwSignal::new(children),
                parents: Arc::new(Mutex::new(parents)),
            }
        }

        pub fn nodes(&self) -> RwSignal<Vec<Node>> {
            self.nodes.clone()
        }

        pub fn root(&self) -> &Node {
            &self.root
        }

        pub fn edges(&self) -> ReadSignal<Children> {
            self.children.read_only()
        }

        pub fn children(&self, parent: &Node) -> Option<RwSignal<Vec<Node>>> {
            self.children.with_untracked(|children| {
                children.iter().find_map(|(p, children)| {
                    if Arc::ptr_eq(p, parent) {
                        Some(children.clone())
                    } else {
                        None
                    }
                })
            })
        }

        /// # Returns
        /// The child's parent if it exists in the map, otherwise `None`.
        ///
        /// # Notes
        /// + `None` is returned in two cases:
        /// 1. The child node does not exist in the graph.
        /// 2. The child node is the graph root.
        /// It is left for the caller to distinguish between tese cases if needed.
        pub fn parent(&self, child: &Node) -> Option<RwSignal<Node>> {
            self.parents.lock().unwrap().iter().find_map(|(c, parent)| {
                if Arc::ptr_eq(c, child) {
                    Some(parent.clone())
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

            let mut ancestors = parent.with_untracked(|parent| self.ancestors(parent));
            ancestors.insert(0, root.clone());
            ancestors
        }

        /// # Returns
        /// Descendants of the root node, including the root node.
        /// If the root node is not found, an empty `Vec` is returned.
        pub fn descendants(&self, root: &Node) -> Vec<Node> {
            let Some(children) = self.children(root) else {
                return vec![];
            };

            let mut descendants = children.with_untracked(|children| {
                children
                    .iter()
                    .flat_map(|child| self.descendants(child))
                    .collect::<Vec<_>>()
            });

            descendants.insert(0, root.clone());
            descendants
        }

        // TODO: Should return signal in case path changes.
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
                    ancestor
                        .name()
                        .get_untracked()
                        .to_string_lossy()
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(SEPARATOR);

            Some(PathBuf::from(SEPARATOR).join(path))
        }

        /// # Returns
        /// If the graph contains the given node.
        pub fn contains(&self, node: &Node) -> bool {
            self.nodes
                .with_untracked(|nodes| nodes.iter().any(|existing| Node::ptr_eq(existing, node)))
        }

        /// Finds a node by its path.
        /// Path should be absolute from the graph root.
        /// i.e. The root path refers to the root node.
        pub fn find(&self, path: impl AsRef<Path>) -> Result<Option<Node>, error::InvalidPath> {
            let mut components = path.as_ref().components();
            let Some(Component::RootDir) = components.next() else {
                return Err(error::InvalidPath);
            };

            let mut node = self.root.clone();
            for component in components {
                match component {
                    Component::Prefix(_)
                    | Component::RootDir
                    | Component::CurDir
                    | Component::ParentDir => return Err(error::InvalidPath),
                    Component::Normal(name) => {
                        let Some(child) =
                            self.children(&node).unwrap().with_untracked(|children| {
                                children.iter().find_map(|child| {
                                    child.name().with_untracked(|child_name| {
                                        if child_name == name {
                                            Some(child.clone())
                                        } else {
                                            None
                                        }
                                    })
                                })
                            })
                        else {
                            return Ok(None);
                        };

                        node = child;
                    }
                }
            }

            Ok(Some(node))
        }

        pub fn find_by_id(&self, rid: &ResourceId) -> Option<Node> {
            self.nodes.with_untracked(|nodes| {
                nodes
                    .iter()
                    .find(|node| {
                        node.properties().with_untracked(|properties| {
                            if let db::state::DataResource::Ok(properties) = properties {
                                properties.rid().with_untracked(|id| id == rid)
                            } else {
                                false
                            }
                        })
                    })
                    .cloned()
            })
        }

        /// Gets the container node that contains the asset.
        pub fn find_by_asset_id(&self, rid: &ResourceId) -> Option<Node> {
            self.nodes.with_untracked(|nodes| {
                nodes
                    .iter()
                    .find(|node| {
                        node.assets().with_untracked(|assets| {
                            if let db::state::DataResource::Ok(assets) = assets {
                                assets.with_untracked(|assets| {
                                    assets
                                        .iter()
                                        .any(|asset| asset.rid().with_untracked(|aid| aid == rid))
                                })
                            } else {
                                false
                            }
                        })
                    })
                    .cloned()
            })
        }

        pub fn find_asset_by_id(&self, rid: &ResourceId) -> Option<super::Asset> {
            self.nodes.with_untracked(|nodes| {
                nodes.iter().find_map(|node| {
                    node.assets().with_untracked(|assets| {
                        if let db::state::DataResource::Ok(assets) = assets {
                            assets.with_untracked(|assets| {
                                assets.iter().find_map(|asset| {
                                    if asset.rid().with_untracked(|aid| aid == rid) {
                                        Some(asset.clone())
                                    } else {
                                        None
                                    }
                                })
                            })
                        } else {
                            None
                        }
                    })
                })
            })
        }
    }

    impl State {
        /// Inserts a subgraph at the indicated path.
        pub fn insert(&self, parent: impl AsRef<Path>, graph: Self) -> Result<(), error::Insert> {
            let Self {
                nodes,
                root,
                children,
                parents,
            } = graph;

            if let Some(node) = nodes.with_untracked(|nodes| {
                nodes.iter().find_map(|node| {
                    if self.contains(node) {
                        Some(node.clone())
                    } else {
                        None
                    }
                })
            }) {
                return Err(error::Insert::NodeAlreadyExists(node.clone()));
            }

            let Some(parent) = self.find(parent)? else {
                return Err(error::Insert::ParentNotFound);
            };

            // NB: Order of adding parents then children then nodes is
            // important for recursion in graph view.
            self.parents
                .lock()
                .unwrap()
                .extend(Arc::into_inner(parents).unwrap().into_inner().unwrap());

            self.parents
                .lock()
                .unwrap()
                .push((root.clone(), RwSignal::new(parent.clone())));

            self.children
                .update(|current| current.extend(children.get_untracked()));

            self.children(&parent)
                .unwrap()
                .update(|children| children.push(root.clone()));

            self.nodes
                .update(|current| current.extend(nodes.get_untracked()));

            Ok(())
        }

        /// Remove a subtree from the graph.
        /// Path should be absolute from the graph root.
        ///
        /// # Notes
        /// + Parent node signals are not updated.
        pub fn remove(&self, path: impl AsRef<Path>) -> Result<Vec<Node>, error::Remove> {
            let Some(root) = self.find(path.as_ref())? else {
                return Err(error::Remove::NotFound);
            };

            let parent = self.parent(&root).unwrap();
            let descendants = self.descendants(&root);
            assert!(!descendants.is_empty());

            // NB: Parents do not update signal when child is removed.
            self.parents.lock().unwrap().retain(|(child, _)| {
                !descendants
                    .iter()
                    .any(|descendant| Node::ptr_eq(child, descendant))
            });

            self.children.update(|children| {
                children.retain(|(parent, _children)| {
                    !descendants
                        .iter()
                        .any(|descendant| Node::ptr_eq(parent, descendant))
                })
            });

            parent.with_untracked(|parent| {
                self.children(&parent)
                    .unwrap()
                    .update(|siblings| siblings.retain(|sibling| !Node::ptr_eq(sibling, &root)));
            });

            self.nodes.update(|nodes| {
                nodes.retain(|node| {
                    !descendants
                        .iter()
                        .any(|descendant| Node::ptr_eq(node, descendant))
                })
            });

            Ok(descendants)
        }

        pub fn rename(
            &self,
            from: impl AsRef<Path>,
            to: impl Into<OsString>,
        ) -> Result<(), error::Move> {
            let Some(node) = self.find(common::normalize_path_sep(from))? else {
                return Err(error::Move::NotFound);
            };

            node.name().set(to.into());
            Ok(())
        }
    }

    pub mod error {
        use super::Node;

        #[derive(Debug)]
        pub struct InvalidPath;

        #[derive(Debug)]
        pub enum Insert {
            ParentNotFound,
            NodeAlreadyExists(Node),
            InvalidPath,
        }

        impl From<InvalidPath> for Insert {
            fn from(_: InvalidPath) -> Self {
                Self::InvalidPath
            }
        }

        #[derive(Debug)]
        pub enum Remove {
            NotFound,
            InvalidPath,
        }

        impl From<InvalidPath> for Remove {
            fn from(_: InvalidPath) -> Self {
                Self::InvalidPath
            }
        }

        #[derive(Debug)]
        pub enum Move {
            NotFound,
            InvalidPath,
            NameConflict,
        }

        impl From<InvalidPath> for Move {
            fn from(_: InvalidPath) -> Self {
                Self::InvalidPath
            }
        }
    }
}

pub mod display {
    use super::{graph, workspace_graph};
    use leptos::prelude::*;
    use std::{
        num::NonZeroUsize,
        sync::{Arc, Mutex},
    };

    /// [`Data`] builder.
    pub struct Builder {
        container: graph::Node,
        visibility: ArcReadSignal<bool>,

        depth: Option<usize>,
        sibling_index: Option<usize>,
        height: Option<NonZeroUsize>,
        width: Option<NonZeroUsize>,
    }

    impl Builder {
        pub fn new(container: graph::Node, visibility: ArcReadSignal<bool>) -> Self {
            Self {
                container,
                visibility,
                depth: None,
                sibling_index: None,
                height: None,
                width: None,
            }
        }

        pub fn depth(&mut self, depth: usize) {
            let _ = self.depth.insert(depth);
        }

        pub fn sibling_index(&mut self, index: usize) {
            let _ = self.sibling_index.insert(index);
        }

        pub fn height(&mut self, height: NonZeroUsize) {
            let _ = self.height.insert(height);
        }

        pub fn width(&mut self, width: NonZeroUsize) {
            let _ = self.width.insert(width);
        }

        pub fn build(self) -> Data {
            let Self {
                container,
                visibility,
                depth,
                sibling_index,
                height,
                width,
            } = self;

            let height = ArcRwSignal::new(height.unwrap());
            let width = ArcRwSignal::new(width.unwrap());

            let height_visible = ArcSignal::derive({
                let visibility = visibility.clone();
                let height = height.clone();
                move || {
                    if *visibility.read() {
                        height.get()
                    } else {
                        NonZeroUsize::new(1).unwrap()
                    }
                }
            });

            let width_visible = ArcSignal::derive({
                let visibility = visibility.clone();
                let width = width.clone();
                move || {
                    if *visibility.read() {
                        width.get()
                    } else {
                        NonZeroUsize::new(1).unwrap()
                    }
                }
            });

            Data {
                container,
                visibility,
                depth: ArcRwSignal::new(depth.unwrap()),
                sibling_index: ArcRwSignal::new(sibling_index.unwrap()),
                height,
                width,
                height_visible,
                width_visible,
            }
        }
    }

    pub struct Data {
        container: graph::Node,
        visibility: ArcReadSignal<bool>,

        depth: ArcRwSignal<usize>,
        sibling_index: ArcRwSignal<usize>,
        height: ArcRwSignal<NonZeroUsize>,
        width: ArcRwSignal<NonZeroUsize>,
        height_visible: ArcSignal<NonZeroUsize>,
        width_visible: ArcSignal<NonZeroUsize>,
    }

    impl Data {
        pub fn depth(&self) -> ArcReadSignal<usize> {
            self.depth.read_only()
        }

        pub fn sibling_index(&self) -> ArcReadSignal<usize> {
            self.sibling_index.read_only()
        }

        pub fn width(&self) -> ArcReadSignal<NonZeroUsize> {
            self.width.read_only()
        }

        pub fn height(&self) -> ArcReadSignal<NonZeroUsize> {
            self.height.read_only()
        }
    }

    pub type Node = Arc<Data>;

    #[derive(Clone)]
    pub struct State {
        root: Node,
        edges: RwSignal<Vec<(Node, ArcRwSignal<Vec<Node>>)>>,
        parents: Arc<Mutex<Vec<(Node, ArcRwSignal<Node>)>>>,
    }

    impl State {
        pub fn from(
            graph: &graph::State,
            visibilities: ReadSignal<workspace_graph::ContainerVisibility>,
        ) -> Self {
            let graph_nodes = graph.nodes().read_untracked();
            let mut nodes = graph_nodes
                .iter()
                .map(|state_node| {
                    let visibility = visibilities
                        .read_untracked()
                        .iter()
                        .find_map(|(node, visibility)| {
                            graph::Node::ptr_eq(node, state_node).then_some(visibility.clone())
                        })
                        .unwrap();

                    Builder::new(state_node.clone(), visibility.read_only())
                })
                .collect::<Vec<_>>();

            Self::set_subtree_properties(&mut nodes, graph);
            let nodes = nodes
                .into_iter()
                .map(|node| Node::new(node.build()))
                .collect::<Vec<_>>();

            let root = graph
                .nodes()
                .read_untracked()
                .iter()
                .position(|node| graph::Node::ptr_eq(node, graph.root()))
                .unwrap();
            let root = nodes[root].clone();

            let parents = graph.nodes().with_untracked(|graph_nodes| {
                graph_nodes
                    .iter()
                    .filter_map(|state_node| {
                        graph.parent(state_node).map(|parent| {
                            let parent_idx = graph_nodes
                                .iter()
                                .position(|node| {
                                    graph::Node::ptr_eq(node, &*parent.read_untracked())
                                })
                                .unwrap();

                            let child_idx = graph_nodes
                                .iter()
                                .position(|node| graph::Node::ptr_eq(node, state_node))
                                .unwrap();

                            (
                                nodes[child_idx].clone(),
                                ArcRwSignal::new(nodes[parent_idx].clone()),
                            )
                        })
                    })
                    .collect::<Vec<_>>()
            });

            let edges = nodes
                .iter()
                .enumerate()
                .map(|(idx, node)| {
                    let state_node = &graph.nodes().read_untracked()[idx];
                    let state_children = graph.children(state_node).unwrap();
                    let children = state_children
                        .read_untracked()
                        .iter()
                        .map(|state_child| {
                            let child_idx = graph
                                .nodes()
                                .read_untracked()
                                .iter()
                                .position(|node| graph::Node::ptr_eq(node, state_child))
                                .unwrap();

                            nodes[child_idx].clone()
                        })
                        .collect::<Vec<_>>();

                    (node.clone(), ArcRwSignal::new(children))
                })
                .collect::<Vec<_>>();

            Self {
                edges: RwSignal::new(edges),
                root,
                parents: Arc::new(Mutex::new(parents)),
            }
        }

        /// Set `depth`, `height`, `width`, and `sibling_index` of nodes.
        fn set_subtree_properties(nodes: &mut Vec<Builder>, graph: &graph::State) {
            fn inner(
                nodes: &mut Vec<Builder>,
                graph: &graph::State,
                root: &graph::Node,
                depth: usize,
                sibling_index: usize,
            ) {
                let idx = graph
                    .nodes()
                    .read_untracked()
                    .iter()
                    .position(|node| graph::Node::ptr_eq(node, root))
                    .unwrap();

                nodes[idx].depth(depth);
                nodes[idx].sibling_index(sibling_index);

                let children = graph.children(root).unwrap();
                children
                    .read_untracked()
                    .iter()
                    .enumerate()
                    .for_each(|(sibling_index, child)| {
                        inner(nodes, graph, child, depth + 1, sibling_index)
                    });

                let children_idx = children
                    .read_untracked()
                    .iter()
                    .map(|child| {
                        graph
                            .nodes()
                            .read_untracked()
                            .iter()
                            .position(|node| graph::Node::ptr_eq(node, child))
                            .unwrap()
                    })
                    .collect::<Vec<_>>();

                let max_child_height = children_idx
                    .iter()
                    .map(|idx| nodes[*idx].height.as_ref().unwrap().get())
                    .max()
                    .unwrap_or(0);
                nodes[idx].height(NonZeroUsize::new(max_child_height + 1).unwrap());

                let width = children_idx
                    .iter()
                    .map(|idx| nodes[*idx].width.as_ref().unwrap().clone())
                    .reduce(|width, child_width| width.checked_add(child_width.get()).unwrap())
                    .unwrap_or(NonZeroUsize::new(1).unwrap());
                nodes[idx].width(width);
            }

            inner(nodes, graph, graph.root(), 0, 0)
        }

        fn parent(&self, child: &graph::Node) -> Option<ArcReadSignal<Node>> {
            self.parents
                .lock()
                .unwrap()
                .iter()
                .find_map(|(node, parent)| {
                    graph::Node::ptr_eq(&node.container, child).then_some(parent.read_only())
                })
        }

        fn _parent(&self, child: &Node) -> Option<ArcRwSignal<Node>> {
            self.parents
                .lock()
                .unwrap()
                .iter()
                .find_map(|(node, parent)| Node::ptr_eq(node, child).then_some(parent.clone()))
        }

        /// Find children of a [`Node`].
        fn _children(&self, parent: &Node) -> Option<ArcRwSignal<Vec<Node>>> {
            self.edges
                .read_untracked()
                .iter()
                .find_map(|(node, children)| {
                    Node::ptr_eq(node, &parent).then_some(children.clone())
                })
        }

        /// Updates nodes' `width` from `node`, recursing upwards.
        fn update_widths_from(&self, node: &Node) {
            let node_width = self
                ._children(node)
                .unwrap()
                .read_untracked()
                .iter()
                .map(|child| child.width().get_untracked())
                .reduce(|total, width| total.checked_add(width.get()).unwrap())
                .unwrap();

            node.width.set(node_width);
            if let Some(parent) = self._parent(node) {
                self.update_widths_from(&*parent.read_untracked());
            }
        }
    }

    impl State {
        pub fn insert(&self, parent: &graph::Node, graph: Self) -> Result<(), error::NotFound> {
            let parent = self.find(parent).ok_or(error::NotFound)?;

            let children = self._children(&parent).unwrap();
            graph.insert_update_depth(parent.depth.get_untracked() + 1);
            graph
                .root
                .sibling_index
                .set(children.read_untracked().len());

            self.insert_update_height(&parent, &graph.root);
            self.insert_update_width(&parent, graph.root.width().get_untracked());

            let Self {
                root,
                edges: graph_edges,
                parents,
            } = graph;
            children.update(|children| children.push(root.clone()));
            self.edges
                .update(|edges| edges.extend(graph_edges.get_untracked()));
            self.parents
                .lock()
                .unwrap()
                .extend(Arc::into_inner(parents).unwrap().into_inner().unwrap());
            self.parents
                .lock()
                .unwrap()
                .push((root, ArcRwSignal::new(parent)));

            Ok(())
        }

        /// Update `Node::depth`s placing the root node at the given depth and recursing downward.
        fn insert_update_depth(&self, depth: usize) {
            fn inner(graph: &State, root: &Node, depth: usize) {
                root.depth.set(depth);
                let children = graph._children(root).unwrap();
                children
                    .read_untracked()
                    .iter()
                    .for_each(|child| inner(graph, child, depth + 1));
            }

            inner(self, &self.root, depth)
        }

        /// Update `[Node]::height`s starting at the given node and recursing upwards.
        fn insert_update_height(&self, parent: &Node, child: &Node) {
            let max_sibling_height = self
                ._children(parent)
                .unwrap()
                .read_untracked()
                .iter()
                .filter_map(|node| {
                    (!Node::ptr_eq(node, child)).then_some(node.height().get_untracked())
                })
                .max();

            let update = max_sibling_height
                .map(|max_sibling_height| *child.height().read_untracked() > max_sibling_height)
                .unwrap_or(true);

            if update {
                parent
                    .height
                    .set(child.height().get_untracked().checked_add(1).unwrap());
                if let Some(grandparent) = self._parent(parent) {
                    self.insert_update_height(&grandparent.get_untracked(), parent);
                }
            }
        }

        /// Update `[Node]::width`s starting at the given node and recursing upwards.
        fn insert_update_width(&self, parent: &Node, child_width: NonZeroUsize) {
            const ONE: NonZeroUsize = NonZeroUsize::new(1).unwrap();
            let children = self._children(&parent).unwrap();
            let children_empty = children.read_untracked().is_empty();
            if children_empty && child_width == ONE {
                assert_eq!(parent.width().read_untracked(), ONE);
                return;
            }

            if children_empty {
                *parent.width.write() = child_width;
            } else {
                let parent_width = children
                    .read_untracked()
                    .iter()
                    .map(|child| child.width().get_untracked())
                    .fold(child_width, |total, width| {
                        total.checked_add(width.get()).unwrap()
                    });

                parent.width.set(parent_width);
            }

            if let Some(grandparent) = self._parent(parent) {
                self.update_widths_from(&*grandparent.read_untracked());
            }
        }
    }

    impl State {
        pub fn remove(&self, root: &graph::Node) -> Result<(), error::NotFound> {
            let root = self.find(root).ok_or(error::NotFound)?;
            assert!(!Node::ptr_eq(&root, &self.root));

            self.remove_update_height(&root);
            self.remove_update_width(&root);

            let parent = self._parent(&root).unwrap();
            let descendants = self.descendants(&root);
            assert!(!descendants.is_empty());
            self.edges.write().retain(|(parent, _)| {
                !descendants
                    .iter()
                    .any(|descendant| Node::ptr_eq(descendant, parent))
            });
            self.parents.lock().unwrap().retain(|(child, _)| {
                !descendants
                    .iter()
                    .any(|descendant| Node::ptr_eq(descendant, child))
            });
            let siblings = self._children(&*parent.read_untracked()).unwrap();
            siblings
                .write()
                .retain(|sibling| !Node::ptr_eq(sibling, &root));
            siblings
                .read_untracked()
                .iter()
                .skip(root.sibling_index.get_untracked())
                .for_each(|sibling| {
                    sibling.sibling_index.update(|idx| *idx -= 1);
                });

            Ok(())
        }

        /// Update `[Node]::height`s starting at the given node and recursing upwards.
        fn remove_update_height(&self, root: &Node) {
            let parent = self._parent(root).unwrap();
            let max_sibling_height = self
                ._children(&*parent.read_untracked())
                .unwrap()
                .read_untracked()
                .iter()
                .filter_map(|node| {
                    (!Node::ptr_eq(node, root)).then_some(node.height().get_untracked())
                })
                .max();

            let update = max_sibling_height
                .map(|max_sibling_height| *root.height().read_untracked() > max_sibling_height)
                .unwrap_or(true);

            if update {
                parent.read_untracked().height.set(
                    max_sibling_height
                        .map(|max_sibling_height| max_sibling_height.checked_add(1).unwrap())
                        .unwrap_or(NonZeroUsize::new(1).unwrap()),
                );
                if let Some(grandparent) = self._parent(&*parent.read_untracked()) {
                    self.insert_update_height(
                        &grandparent.get_untracked(),
                        &*parent.read_untracked(),
                    );
                }
            }
        }

        /// Update `[Node]::width`s starting at the given node and recursing upwards.
        fn remove_update_width(&self, root: &Node) {
            const ONE: NonZeroUsize = NonZeroUsize::new(1).unwrap();
            let parent = self._parent(root).unwrap();
            if *parent.read_untracked().width().read_untracked() == ONE {
                return;
            }

            let siblings = self._children(&*parent.read_untracked()).unwrap();
            let parent_width = siblings
                .read_untracked()
                .iter()
                .filter_map(|sibling| {
                    (!Node::ptr_eq(sibling, root)).then_some(sibling.width().get_untracked())
                })
                .reduce(|total, width| total.checked_add(width.get()).unwrap())
                .unwrap_or(ONE);
            parent.read_untracked().width.set(parent_width);

            if let Some(grandparent) = self._parent(&*parent.read_untracked()) {
                self.update_widths_from(&*grandparent.read_untracked());
            }
        }

        /// # Returns
        /// Descendants of the root node, including the root node.
        /// If the root node is not found, an empty `Vec` is returned.
        pub fn descendants(&self, root: &Node) -> Vec<Node> {
            let Some(children) = self._children(root) else {
                return vec![];
            };

            let mut descendants = children
                .read_untracked()
                .iter()
                .flat_map(|child| self.descendants(child))
                .collect::<Vec<_>>();

            descendants.insert(0, root.clone());
            descendants
        }
    }

    impl State {
        pub fn find(&self, container: &graph::Node) -> Option<Node> {
            self.edges.read_untracked().iter().find_map(|(node, _)| {
                graph::Node::ptr_eq(&node.container, container).then_some(node.clone())
            })
        }

        pub fn children(&self, parent: &graph::Node) -> Option<ArcReadSignal<Vec<Node>>> {
            self.edges
                .read_untracked()
                .iter()
                .find_map(|(node, children)| {
                    graph::Node::ptr_eq(&node.container, parent).then_some(children.read_only())
                })
        }

        pub fn sibling_width_until(&self, container: &graph::Node) -> Option<ArcSignal<usize>> {
            let container = self.find(container)?;
            let Some(parent) = self._parent(&container) else {
                return Some(ArcSignal::derive(move || 0));
            };

            Some(ArcSignal::derive({
                let edges = self.edges.read_only();
                let sibling_index = container.sibling_index();
                move || {
                    let siblings = edges
                        .read_untracked()
                        .iter()
                        .find_map(|(parent_node, children)| {
                            Node::ptr_eq(parent_node, &*parent.read())
                                .then_some(children.read_only())
                        })
                        .unwrap();

                    siblings
                        .read()
                        .iter()
                        .take(sibling_index.get())
                        .map(|node| node.width().get().get())
                        .reduce(|total, width| total + width)
                        .unwrap_or(0)
                }
            }))
        }
    }

    pub mod error {
        #[derive(Debug)]
        pub struct NotFound;
    }
}

pub mod container {
    use super::Metadata;
    use chrono::*;
    use leptos::prelude::*;
    use std::{ffi::OsString, path::PathBuf};
    use syre_core::{
        self as core,
        project::ContainerProperties,
        types::{Creator, ResourceId, ResourceMap, UserId, UserPermissions},
    };
    use syre_local_database as db;

    pub type PropertiesState = db::state::DataResource<Properties>;
    pub type AnalysesState = db::state::DataResource<RwSignal<Vec<AnalysisAssociation>>>;
    pub type AssetsState = db::state::DataResource<RwSignal<Vec<Asset>>>;
    pub type SettingsState = db::state::DataResource<Settings>;

    #[derive(Clone, Debug)]
    pub struct State {
        /// Folder name.
        name: RwSignal<OsString>,
        properties: RwSignal<PropertiesState>,
        analyses: RwSignal<AnalysesState>,
        assets: RwSignal<AssetsState>,
        settings: RwSignal<SettingsState>,
    }

    impl State {
        pub fn new(container: db::state::Container) -> Self {
            let properties = container.properties().map(|properties| {
                let rid = container.rid().cloned().unwrap();
                Properties::new(rid, properties.clone())
            });

            let analyses = container.analyses().map(|associations| {
                RwSignal::new(
                    associations
                        .iter()
                        .map(|association| AnalysisAssociation::new(association.clone()))
                        .collect(),
                )
            });

            let assets = container.assets().map(|assets| {
                let assets = assets
                    .iter()
                    .map(|asset| Asset::new(asset.clone()))
                    .collect();

                RwSignal::new(assets)
            });

            let settings = container
                .settings()
                .map(|settings| Settings::new(settings.clone()));

            Self {
                name: RwSignal::new(container.name().clone()),
                properties: RwSignal::new(properties),
                analyses: RwSignal::new(analyses),
                assets: RwSignal::new(assets),
                settings: RwSignal::new(settings),
            }
        }

        pub fn name(&self) -> RwSignal<OsString> {
            self.name.clone()
        }

        pub fn properties(&self) -> RwSignal<PropertiesState> {
            self.properties.clone()
        }

        pub fn analyses(&self) -> RwSignal<AnalysesState> {
            self.analyses.clone()
        }

        pub fn assets(&self) -> RwSignal<AssetsState> {
            self.assets.clone()
        }

        pub fn settings(&self) -> RwSignal<SettingsState> {
            self.settings.clone()
        }
    }

    #[derive(Clone)]
    pub struct Properties {
        rid: RwSignal<ResourceId>,
        name: RwSignal<String>,
        kind: RwSignal<Option<String>>,
        description: RwSignal<Option<String>>,
        tags: RwSignal<Vec<String>>,
        metadata: RwSignal<Metadata>,
    }

    impl Properties {
        pub fn new(rid: ResourceId, properties: ContainerProperties) -> Self {
            let ContainerProperties {
                name,
                kind,
                description,
                tags,
                metadata,
            } = properties;

            Self {
                rid: RwSignal::new(rid),
                name: RwSignal::new(name),
                kind: RwSignal::new(kind),
                description: RwSignal::new(description),
                tags: RwSignal::new(tags),
                metadata: RwSignal::new(Metadata::from(metadata)),
            }
        }
        pub fn rid(&self) -> RwSignal<ResourceId> {
            self.rid
        }

        pub fn name(&self) -> RwSignal<String> {
            self.name
        }

        pub fn kind(&self) -> RwSignal<Option<String>> {
            self.kind
        }

        pub fn description(&self) -> RwSignal<Option<String>> {
            self.description
        }

        pub fn tags(&self) -> RwSignal<Vec<String>> {
            self.tags
        }

        pub fn metadata(&self) -> RwSignal<Metadata> {
            self.metadata
        }
    }

    impl Properties {
        /// Convert into [`syre_core::project::ContainerProperties`].
        pub fn as_properties(&self) -> syre_core::project::ContainerProperties {
            let metadata = self.metadata().with_untracked(|metadata| {
                metadata
                    .iter()
                    .map(|(key, value)| (key.clone(), value.get_untracked()))
                    .collect()
            });

            syre_core::project::ContainerProperties {
                name: self.name.get_untracked(),
                kind: self.kind.get_untracked(),
                description: self.description.get_untracked(),
                tags: self.tags.get_untracked(),
                metadata,
            }
        }
    }

    #[derive(Clone)]
    pub struct AnalysisAssociation {
        analysis: ResourceId,
        autorun: RwSignal<bool>,
        priority: RwSignal<i32>,
    }

    impl AnalysisAssociation {
        pub fn new(association: core::project::AnalysisAssociation) -> Self {
            let analysis = association.analysis().clone();
            let syre_core::project::AnalysisAssociation {
                autorun, priority, ..
            } = association;

            Self {
                analysis,
                autorun: RwSignal::new(autorun),
                priority: RwSignal::new(priority),
            }
        }

        pub fn analysis(&self) -> &ResourceId {
            &self.analysis
        }

        pub fn autorun(&self) -> RwSignal<bool> {
            self.autorun.clone()
        }

        pub fn priority(&self) -> RwSignal<i32> {
            self.priority.clone()
        }

        /// Crates an [`syre_core::project::AnalysisAssociation`] from the current values.
        pub fn as_association(&self) -> core::project::AnalysisAssociation {
            core::project::AnalysisAssociation::with_params(
                self.analysis.clone(),
                self.autorun.get_untracked(),
                self.priority.get_untracked(),
            )
        }
    }

    #[derive(Clone)]
    pub struct Asset {
        rid: RwSignal<ResourceId>,
        name: RwSignal<Option<String>>,
        kind: RwSignal<Option<String>>,
        description: RwSignal<Option<String>>,
        tags: RwSignal<Vec<String>>,
        metadata: RwSignal<Metadata>,
        path: RwSignal<PathBuf>,
        fs_resource: RwSignal<db::state::FileResource>,
        created: RwSignal<DateTime<Utc>>,
        creator: RwSignal<Creator>,
    }

    impl Asset {
        pub fn new(asset: db::state::Asset) -> Self {
            let fs_resource = if asset.is_present() {
                db::state::FileResource::Present
            } else {
                db::state::FileResource::Absent
            };

            let metadata = (*asset).properties.metadata.clone();
            let metadata = Metadata::from(metadata);

            Self {
                rid: RwSignal::new(asset.rid().clone()),
                name: RwSignal::new((*asset).properties.name.clone()),
                kind: RwSignal::new((*asset).properties.kind.clone()),
                description: RwSignal::new((*asset).properties.description.clone()),
                tags: RwSignal::new((*asset).properties.tags.clone()),
                metadata: RwSignal::new(metadata),
                path: RwSignal::new((*asset).path.clone()),
                fs_resource: RwSignal::new(fs_resource),
                created: RwSignal::new((*asset).properties.created().clone()),
                creator: RwSignal::new((*asset).properties.creator.clone()),
            }
        }

        pub fn rid(&self) -> RwSignal<ResourceId> {
            self.rid.clone()
        }

        pub fn name(&self) -> RwSignal<Option<String>> {
            self.name.clone()
        }

        pub fn kind(&self) -> RwSignal<Option<String>> {
            self.kind.clone()
        }

        pub fn description(&self) -> RwSignal<Option<String>> {
            self.description.clone()
        }

        pub fn tags(&self) -> RwSignal<Vec<String>> {
            self.tags.clone()
        }

        pub fn metadata(&self) -> RwSignal<Metadata> {
            self.metadata.clone()
        }

        pub fn path(&self) -> RwSignal<PathBuf> {
            self.path.clone()
        }

        pub fn fs_resource(&self) -> RwSignal<db::state::FileResource> {
            self.fs_resource.clone()
        }

        pub fn created(&self) -> RwSignal<DateTime<Utc>> {
            self.created.clone()
        }

        pub fn creator(&self) -> RwSignal<Creator> {
            self.creator.clone()
        }
    }

    impl Asset {
        /// Convert into [`syre_core::project::AssetProperties`].
        pub fn as_properties(&self) -> syre_core::project::AssetProperties {
            let mut asset = syre_core::project::asset_properties::Builder::new();
            self.name.with_untracked(|name| {
                if let Some(name) = name {
                    asset.set_name(name);
                }
            });
            self.kind.with_untracked(|kind| {
                if let Some(kind) = kind {
                    asset.set_kind(kind);
                }
            });
            self.description.with_untracked(|description| {
                if let Some(description) = description {
                    asset.set_description(description);
                }
            });
            asset.set_tags(self.tags.get_untracked());
            asset.set_created(self.created.get_untracked());
            asset.set_creator(self.creator.get_untracked());

            let metadata = self.metadata().with_untracked(|metadata| {
                metadata
                    .iter()
                    .map(|(key, value)| (key.clone(), value.get_untracked()))
                    .collect()
            });
            asset.set_metadata(metadata);

            asset.into()
        }
    }

    #[derive(Clone)]
    pub struct Settings {
        creator: RwSignal<Option<UserId>>,
        created: RwSignal<DateTime<Utc>>,
        permissions: RwSignal<ResourceMap<UserPermissions>>,
    }

    impl Settings {
        pub fn new(settings: syre_local::project::config::ContainerSettings) -> Self {
            let syre_local::project::config::ContainerSettings {
                creator,
                created,
                permissions,
            } = settings;

            Self {
                creator: RwSignal::new(creator),
                created: RwSignal::new(created),
                permissions: RwSignal::new(permissions),
            }
        }

        pub fn creator(&self) -> RwSignal<Option<UserId>> {
            self.creator.clone()
        }

        pub fn created(&self) -> RwSignal<DateTime<Utc>> {
            self.created.clone()
        }

        pub fn permissions(&self) -> RwSignal<ResourceMap<UserPermissions>> {
            self.permissions.clone()
        }
    }
}

mod metadata {
    use leptos::prelude::*;
    use syre_core::types::data::Value;

    #[derive(derive_more::Deref, derive_more::DerefMut, Clone)]
    pub struct Metadata(Vec<Metadatum>);
    impl Metadata {
        pub fn from(metadata: syre_core::project::Metadata) -> Self {
            let metadata = metadata
                .into_iter()
                .map(|(key, value)| (key, RwSignal::new(value)))
                .collect();

            Self(metadata)
        }

        pub fn as_properties(&self) -> syre_core::project::Metadata {
            self.0
                .iter()
                .map(|(key, value)| (key.clone(), value()))
                .collect()
        }
    }

    impl IntoIterator for Metadata {
        type Item = Metadatum;
        type IntoIter = std::vec::IntoIter<Self::Item>;

        fn into_iter(self) -> Self::IntoIter {
            self.0.into_iter()
        }
    }

    pub type Metadatum = (String, RwSignal<Value>);
}

mod flags {
    use crate::common;
    use leptos::prelude::*;
    use std::{
        assert_matches::assert_matches,
        path::{Path, PathBuf},
    };
    use syre_local::project::resources::Flag;
    use syre_local_database as db;

    pub type Flags = Vec<(PathBuf, ArcRwSignal<Vec<Flag>>)>;

    /// Flags keyed by full resource graph path.
    #[derive(derive_more::Deref, Clone, Copy)]
    pub struct State(RwSignal<Flags>);
    impl State {
        pub fn new(graph: &db::state::Graph) -> Self {
            fn inner(
                flags: &mut Flags,
                parent_path: impl AsRef<Path>,
                container_idx: usize,
                graph: &db::state::Graph,
            ) {
                let db::state::Graph { nodes, children } = graph;
                let container = &nodes[container_idx];

                let container_path = if container_idx == 0 {
                    parent_path.as_ref().join(std::path::Component::RootDir)
                } else {
                    parent_path.as_ref().join(container.name())
                };

                if let Ok(container_flags) = container.flags() {
                    container_flags
                        .iter()
                        .for_each(|(resource_path, resource_flags)| {
                            let num_components = resource_path.components().count();
                            assert!(num_components > 0);
                            let resource_path = if num_components == 1 {
                                let segment = resource_path.components().next().unwrap();
                                match segment {
                                    std::path::Component::RootDir => container_path.clone(),
                                    std::path::Component::Normal(resource_path) => {
                                        container_path.join(resource_path)
                                    }
                                    _ => panic!("invalid path"),
                                }
                            } else {
                                assert!(resource_path.components().all(|segment| matches!(
                                    segment,
                                    std::path::Component::Normal(_)
                                )));

                                container_path.join(resource_path)
                            };

                            flags.push((
                                common::normalize_path_sep(resource_path),
                                ArcRwSignal::new(resource_flags.clone()),
                            ));
                        });
                };

                for &child in &children[container_idx] {
                    inner(flags, &container_path, child, graph);
                }
            }

            let mut flags = vec![];
            inner(&mut flags, PathBuf::new(), 0, graph);

            Self(RwSignal::new(flags))
        }

        pub fn find(&self, path: impl Into<PathBuf>) -> ArcSignal<Option<ArcRwSignal<Vec<Flag>>>> {
            let path = path.into();
            ArcSignal::derive({
                let flags = self.0;
                move || {
                    flags.read().iter().find_map(|(flags_path, flags)| {
                        (*flags_path == path).then_some(flags.clone())
                    })
                }
            })
        }
    }
}
