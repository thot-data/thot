//! Local [`ResourceTree`](CoreTres).
use crate::{common, error::Result, project::resources};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use syre_core::{
    error::{Error as CoreError, Resource as ResourceError},
    graph::{
        tree::{EdgeMap, NodeMap},
        {ResourceNode, ResourceTree},
    },
    project::{Asset, Container as CoreContainer},
    types::ResourceId,
};

type CoreContainerTree = ResourceTree<CoreContainer>;
type ContainerTree = ResourceTree<resources::Container>;

pub struct ContainerTreeTransformer;
impl ContainerTreeTransformer {
    /// Convert a Container tree to a Core Container tree.
    pub fn local_to_core(tree: &ContainerTree) -> CoreContainerTree {
        let core_nodes = tree
            .nodes()
            .iter()
            .map(|(rid, node)| (rid.clone(), ResourceNode::new((*node.data()).clone())))
            .collect::<NodeMap<CoreContainer>>();

        ResourceTree::from_parts(core_nodes, tree.edges().clone())
            .expect("could not build tree from components")
    }

    /// Convert a subtree of a Container tree to a Core Container tree.
    ///
    /// # Returns
    /// The converted subtree if the root node was found, otherwise `None`.
    pub fn subtree_to_core(
        tree: &ContainerTree,
        root: &ResourceId,
    ) -> Option<ResourceTree<CoreContainer>> {
        let Some(rids) = tree.descendants(root) else {
            return None;
        };

        let mut core_nodes = NodeMap::new();
        let mut edges = EdgeMap::new();
        for rid in rids {
            let node = tree.get(&rid).expect("descendant not it graph");
            core_nodes.insert(rid.clone(), ResourceNode::new((*node.data()).clone()));
            edges.insert(
                rid.clone(),
                tree.children(&rid)
                    .expect("could not get children of descendant")
                    .clone(),
            );
        }

        let graph =
            ResourceTree::from_parts(core_nodes, edges).expect("could not reconstuct graph");

        Some(graph)
    }

    pub fn core_to_local(tree: CoreContainerTree, base_path: impl Into<PathBuf>) -> ContainerTree {
        let base_path = base_path.into();
        let rel_paths = tree
            .nodes()
            .values()
            .map(|node| {
                let mut path = tree
                    .ancestors(node.rid())
                    .into_iter()
                    .map(|rid| &tree.get(&rid).unwrap().properties.name)
                    .collect::<Vec<_>>();

                path.reverse();
                let path = path
                    .into_iter()
                    .fold(base_path.clone(), |path, segment| path.join(segment));

                (node.rid().clone(), path)
            })
            .collect::<HashMap<_, _>>();

        let (nodes, edges) = tree.into_components();
        let nodes = nodes
            .into_values()
            .map(|node| {
                let mut container = resources::Container::new(rel_paths.get(&node.rid()).unwrap());
                container.inner = node.into_data();
                (container.rid().clone(), ResourceNode::new(container))
            })
            .collect::<HashMap<ResourceId, ResourceNode<resources::Container>>>();

        ResourceTree::from_parts(nodes, edges).unwrap()
    }
}

// ******************
// *** Duplicator ***
// ******************

pub struct ContainerTreeDuplicator;
impl ContainerTreeDuplicator {
    /// Duplicates a subtree.
    ///
    /// # Arguments
    /// 1. Graph.
    /// 2. Id of the root of the subtree to duplicate.
    ///
    /// # Notes
    /// + `Asset`s are duplicated.
    #[tracing::instrument(skip(graph))]
    pub fn duplicate(graph: &ContainerTree, root: &ResourceId) -> syre_core::Result<ContainerTree> {
        // ensure root exists
        let Some(node) = graph.get(root) else {
            return Err(CoreError::Resource(ResourceError::does_not_exist(
                "`Container` does not exist in graph",
            )));
        };

        let mut container = resources::Container::new(node.base_path());
        container.properties = node.properties.clone();
        container.analyses = node.analyses.clone();
        for asset_base in node.assets.iter() {
            let mut asset = Asset::new(asset_base.path.clone());
            asset.properties = asset_base.properties.clone();
            container.assets.push(asset);
        }

        let dup_root = container.rid().clone();
        let mut dup_graph = ResourceTree::new(container);
        let Some(children) = graph.children(&root).cloned() else {
            return Err(CoreError::Resource(ResourceError::does_not_exist(
                "`Container` does not exist in graph",
            )));
        };

        for child in children {
            let c_tree = Self::duplicate(graph, &child)?;
            dup_graph.insert_tree(&dup_root, c_tree)?;
        }

        Ok(dup_graph)
    }

    /// Duplicates a subtree to a new file path.
    /// Base paths of the [`Containers`] are updated.
    ///
    /// # Arguments
    /// 1. Path to duplicate the tree to.
    /// 2. Graph.
    /// 3. Id of the root of the subtree to duplicate.
    ///
    /// # Notes
    /// + `Asset`s are not copied.
    pub fn duplicate_without_assets_to(
        path: impl AsRef<Path>,
        graph: &ContainerTree,
        root: &ResourceId,
    ) -> Result<ContainerTree> {
        let path = path.as_ref();
        let tmp = tempfile::TempDir::new().unwrap();
        let mut dup = duplicate_without_assets_to(tmp.path(), graph, root)?;
        dup.iter_nodes_mut().for_each(|(_, node)| {
            let rel_path = node.base_path().strip_prefix(tmp.path()).unwrap();
            let abs_path = path.join(rel_path);
            node.set_base_path(abs_path);
        });
        fs::rename(tmp.path(), path).map_err(|err| crate::Error::Io(err.kind()))?;
        Ok(dup)
    }
}

/// Duplicates a subtree to a new file path.
/// Base paths of the [`Containers`] are updated.
///
/// # Arguments
/// 1. Path to duplicate the tree to.
/// 2. Graph.
/// 3. Id of the root of the subtree to duplicate.
///
/// # Notes
/// + `Asset`s are not copied.
fn duplicate_without_assets_to(
    path: &Path,
    graph: &ContainerTree,
    root: &ResourceId,
) -> Result<ContainerTree> {
    // ensure root exists
    let Some(node) = graph.get(root) else {
        return Err(CoreError::Resource(ResourceError::does_not_exist(
            "`Container` does not exist in graph",
        ))
        .into());
    };

    // duplicate container to new location
    // first create entire tree in temp folder, then move to desired location
    let mut container = resources::Container::new(path);
    container.properties = node.properties.clone();
    container.analyses = node.analyses.clone();
    container.save().map_err(|err| match err {
        resources::container::error::Save::CreateDir(error) => error,
        resources::container::error::Save::SaveFiles {
            properties,
            assets,
            settings,
        } => {
            if let Some(err) = properties {
                err
            } else if let Some(err) = assets {
                err
            } else {
                settings.unwrap()
            }
        }
    })?;

    let dup_root = container.rid().clone();
    let mut dup_graph = ResourceTree::new(container);
    let Some(children) = graph.children(&root).cloned() else {
        return Err(CoreError::Resource(ResourceError::does_not_exist(
            "`Container` does not exist in graph",
        ))
        .into());
    };

    for child in children {
        let rel_path = graph
            .get(&child)
            .expect("could not get child node")
            .base_path()
            .file_name()
            .expect("could not get name of `Container`");

        let c_path = path.join(rel_path);
        let c_path = common::normalize_path_separators(c_path);
        let c_tree = duplicate_without_assets_to(&c_path, graph, &child)?;
        dup_graph.insert_tree(&dup_root, c_tree)?;
    }

    Ok(dup_graph)
}

#[cfg(test)]
#[path = "./tree_test.rs"]
mod tree_test;
