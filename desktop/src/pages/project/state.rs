pub use container::{AnalysisAssociation, Asset, State as Container};
pub use graph::State as Graph;
pub use metadata::Metadata;
pub use project::{Analysis, State as Project};
pub use workspace::State as Workspace;
pub use workspace_graph::State as WorkspaceGraph;

pub mod workspace {
    use leptos::*;

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
    use leptos::*;
    use std::rc::Rc;
    use syre_core::types::ResourceId;
    use syre_local_database as db;

    pub type ContainerVisibility = Vec<(super::graph::Node, RwSignal<bool>)>;

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
                    .map(|node| (node, create_rw_signal(true)))
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
        ) -> Option<RwSignal<bool>> {
            self.container_visibility.with_untracked(|containers| {
                containers.iter().find_map(|(node, visibility)| {
                    Rc::ptr_eq(node, container).then_some(visibility.clone())
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
                resources: create_rw_signal(resources),
                selected: create_rw_signal(selected),
            }
        }

        pub fn selected(&self) -> ReadSignal<Vec<Resource>> {
            self.selected.read_only()
        }

        /// Get a resources selection state.
        pub fn get(&self, rid: &ResourceId) -> Option<ReadSignal<bool>> {
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
        selected: RwSignal<bool>,
    }

    impl ResourceSelection {
        pub fn new(rid: ReadSignal<ResourceId>, kind: ResourceKind) -> Self {
            Self {
                rid,
                kind,
                selected: create_rw_signal(false),
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
    use leptos::*;
    use std::path::PathBuf;
    use syre_core as core;
    use syre_core::{
        project::Project as CoreProject,
        types::{ResourceId, ResourceMap, UserId, UserPermissions},
    };
    use syre_local::types::{AnalysisKind, ProjectSettings};
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
            self.fs_resource.with(|resource| resource.is_present())
        }
    }
}

pub mod graph {
    use super::Container;
    use crate::common;
    use leptos::*;
    use std::{
        cell::RefCell,
        ffi::OsString,
        iter,
        num::NonZeroUsize,
        ops::Deref,
        path::{Component, Path, PathBuf},
        rc::Rc,
    };
    use syre_core::types::ResourceId;
    use syre_local_database as db;

    pub type Node = Rc<Data>;

    #[derive(Debug, Clone)]
    pub struct Data {
        state: Container,
        graph: GraphData,
    }

    impl Data {
        /// # Arguments
        /// 1. `container`: Container state.
        /// 2. `subtree_width`: Width of the subtree rooted at container.
        /// 3. `subtree_height`: Height of the subtree rooted at container.
        /// 4. `sibling_index`: Index amongst siblings.
        pub fn new(
            container: db::state::Container,
            subtree_width: NonZeroUsize,
            subtree_height: NonZeroUsize,
            sibling_index: usize,
        ) -> Self {
            Self {
                state: Container::new(container),
                graph: GraphData::new(subtree_width, subtree_height, sibling_index),
            }
        }

        pub fn state(&self) -> &Container {
            &self.state
        }

        pub fn subtree_height(&self) -> ReadSignal<NonZeroUsize> {
            self.graph.subtree_height.read_only()
        }

        pub fn subtree_width(&self) -> ReadSignal<NonZeroUsize> {
            self.graph.subtree_width.read_only()
        }

        pub fn sibling_index(&self) -> ReadSignal<usize> {
            self.graph.sibling_index.read_only()
        }
    }

    impl Deref for Data {
        type Target = Container;
        fn deref(&self) -> &Self::Target {
            &self.state
        }
    }

    #[derive(Clone, Debug)]
    pub struct GraphData {
        subtree_width: RwSignal<NonZeroUsize>,
        subtree_height: RwSignal<NonZeroUsize>,

        /// Index amongst siblings.
        sibling_index: RwSignal<usize>,
    }

    impl GraphData {
        pub fn new(
            subtree_width: NonZeroUsize,
            subtree_height: NonZeroUsize,
            sibling_index: usize,
        ) -> Self {
            Self {
                subtree_width: create_rw_signal(subtree_width),
                subtree_height: create_rw_signal(subtree_height),
                sibling_index: create_rw_signal(sibling_index),
            }
        }

        pub(self) fn set_subtree_width(&self, width: NonZeroUsize) {
            self.subtree_width.set(width);
        }

        pub(self) fn set_subtree_height(&self, height: NonZeroUsize) {
            self.subtree_height.set(height);
        }

        pub(self) fn set_sibling_index(&self, index: usize) {
            self.sibling_index.set(index);
        }
    }

    pub type Children = Vec<(Node, RwSignal<Vec<Node>>)>;

    #[derive(Clone)]
    pub struct State {
        nodes: RwSignal<Vec<Node>>, // NOTE: `nodes` is redundant with `children`, could be removed.
        root: Node,
        children: RwSignal<Children>,
        parents: Rc<RefCell<Vec<(Node, RwSignal<Node>)>>>,
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

            let graph_data = (0..nodes.len())
                .map(|root| Self::graph_data(root, &children))
                .collect::<Vec<_>>();

            let sibling_index = (0..nodes.len())
                .into_iter()
                .map(|node| {
                    parents[node]
                        .map(|parent| {
                            children[parent]
                                .iter()
                                .position(|child| *child == node)
                                .unwrap()
                        })
                        .unwrap_or(0)
                })
                .collect::<Vec<_>>();

            let nodes = nodes
                .into_iter()
                .enumerate()
                .map(|(index, container)| {
                    Rc::new(Data::new(
                        container,
                        graph_data[index].0,
                        graph_data[index].1,
                        sibling_index[index],
                    ))
                })
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
                parents: Rc::new(RefCell::new(parents)),
            }
        }

        /// # Arguments
        /// `root`: Index of the subtree's root.
        /// `graph`: Graph edges as indices.
        ///
        /// # Returns
        /// Tuple of `(subgraph width, subgraph height)` for the given node.
        fn graph_data(root: usize, graph: &Vec<Vec<usize>>) -> (NonZeroUsize, NonZeroUsize) {
            let children_data = graph[root]
                .iter()
                .map(|child| Self::graph_data(*child, graph))
                .collect::<Vec<_>>();

            let width = children_data
                .iter()
                .map(|data| data.0)
                .reduce(|total, width| total.checked_add(width.get()).unwrap())
                .unwrap_or(NonZeroUsize::new(1).unwrap());

            let height = children_data
                .iter()
                .map(|data| data.1)
                .max()
                .map(|height| height.checked_add(1).unwrap())
                .unwrap_or(NonZeroUsize::new(1).unwrap());

            (width, height)
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
                    if Rc::ptr_eq(p, parent) {
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
            self.parents.borrow().iter().find_map(|(c, parent)| {
                if Rc::ptr_eq(c, child) {
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
            if Rc::ptr_eq(&self.root, root) {
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
                        node.state.properties().with_untracked(|properties| {
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
                        node.state.assets().with_untracked(|assets| {
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
                    node.state.assets().with_untracked(|assets| {
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
            use std::cmp;

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

            parent.graph.subtree_height.update(|height| {
                *height = cmp::max(
                    height.clone(),
                    root.subtree_height()
                        .get_untracked()
                        .checked_add(1)
                        .unwrap(),
                )
            });
            for ancestor in self.ancestors(&parent).iter().skip(1) {
                let height = ancestor.subtree_height().get_untracked();
                let height_new = self
                    .children(ancestor)
                    .unwrap()
                    .with_untracked(|children| {
                        children
                            .iter()
                            .map(|child| child.subtree_height().get_untracked())
                            .collect::<Vec<_>>()
                    })
                    .into_iter()
                    .max()
                    .unwrap()
                    .checked_add(1)
                    .unwrap();

                if height_new > height {
                    ancestor.graph.set_subtree_height(height_new);
                } else if height_new == height {
                    break;
                } else {
                    panic!("inserting should not reduce height");
                }
            }

            let siblings = self.children(&parent).unwrap();
            parent.graph.subtree_width.update(|width| {
                let root_width = root.subtree_width().get_untracked();
                if siblings.with_untracked(|siblings| siblings.is_empty()) {
                    *width = root_width;
                } else {
                    *width = width.checked_add(root_width.get()).unwrap();
                }
            });
            for ancestor in self.ancestors(&parent).iter().skip(1) {
                let width = ancestor.subtree_width().get_untracked();
                let width_new = self.children(ancestor).unwrap().with_untracked(|children| {
                    children
                        .iter()
                        .map(|child| child.subtree_width().get_untracked())
                        .reduce(|total, width| total.checked_add(width.get()).unwrap())
                        .unwrap()
                });

                if width_new > width {
                    ancestor.graph.set_subtree_width(width_new);
                } else if width_new == width {
                    break;
                } else {
                    panic!("inserting should not reduce width");
                }
            }

            root.graph
                .set_sibling_index(siblings.with_untracked(|siblings| siblings.len()));

            // NB: Order of adding parents then children then nodes is
            // important for recursion in graph view.
            self.parents
                .borrow_mut()
                .extend(Rc::into_inner(parents).unwrap().into_inner());

            self.parents
                .borrow_mut()
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
            use std::cmp;

            let Some(root) = self.find(path.as_ref())? else {
                return Err(error::Remove::NotFound);
            };

            let parent = self.parent(&root).unwrap();
            let descendants = self.descendants(&root);
            assert!(!descendants.is_empty());

            let root_width = root.graph.subtree_width.get_untracked().get();
            let parent_width = parent
                .with_untracked(|parent| parent.subtree_width().get_untracked())
                .get();
            let delta_width = if root_width == 1 && parent_width == 1 {
                0
            } else if root_width == parent_width {
                root_width - 1
            } else {
                root_width
            };

            parent.with_untracked(|parent| {
                self.children(parent).unwrap().with_untracked(|siblings| {
                    root.graph.sibling_index.with_untracked(|root_index| {
                        for sibling in siblings.iter().skip(root_index + 1) {
                            sibling.graph.sibling_index.update(|index| {
                                *index -= 1;
                            })
                        }
                    })
                })
            });

            if delta_width > 0 {
                for ancestor in self.ancestors(&root).iter().skip(1) {
                    ancestor.graph.subtree_width.update(|width| {
                        let val = width.get() - delta_width;
                        *width = NonZeroUsize::new(val).unwrap();
                    });
                }
            }

            let ancestors = self.ancestors(&root);
            for (parent, ancestor) in iter::zip(ancestors.iter().skip(1), ancestors.iter()) {
                let delta_height = cmp::max(
                    root.graph.subtree_height.with_untracked(|height| {
                        let sibling_height_max =
                            self.children(parent).unwrap().with_untracked(|siblings| {
                                siblings
                                    .iter()
                                    .filter_map(|sibling| {
                                        (!Node::ptr_eq(sibling, &ancestor))
                                            .then_some(sibling.graph.subtree_height.get_untracked())
                                    })
                                    .max()
                                    .map(|sibling_max_height| sibling_max_height.get())
                                    .unwrap_or(0)
                            });

                        height.get() as isize - sibling_height_max as isize
                    }),
                    0,
                ) as usize;

                if delta_height == 0 {
                    break;
                }

                parent.graph.subtree_height.update(|height| {
                    let val = height.get() - delta_height;
                    *height = NonZeroUsize::new(val).unwrap()
                });
            }

            // NB: Parents do not update signal when child is removed.
            self.parents.borrow_mut().retain(|(child, _)| {
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

            node.state.name().set(to.into());
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

pub mod container {
    use super::Metadata;
    use chrono::*;
    use leptos::*;
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
            self.rid.clone()
        }

        pub fn name(&self) -> RwSignal<String> {
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
        pub fn new(settings: syre_local::types::ContainerSettings) -> Self {
            let syre_local::types::ContainerSettings {
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
    use leptos::*;
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
