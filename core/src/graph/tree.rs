//! A tree graph
use super::ResourceNode;
use crate::{
    error::{Graph as GraphError, Resource as ResourceError},
    project::Container,
    types::{ResourceId, ResourceMap},
    Result,
};
use has_id::HasId;
use indexmap::IndexSet;
use std::{
    collections::{
        hash_map::{Iter, IterMut},
        HashSet,
    },
    fmt,
    path::{Component, Path, PathBuf},
    result::Result as StdResult,
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// *************
// *** types ***
// *************

/// Map from node id to node.
pub type NodeMap<D> = ResourceMap<ResourceNode<D>>;

/// Map from parent node to children.
pub type EdgeMap = ResourceMap<IndexSet<ResourceId>>;

// *********************
// *** resource tree ***
// *********************

/// A tree graph.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, PartialEq)]
pub struct ResourceTree<D>
where
    D: HasId<Id = ResourceId>,
{
    root: ResourceId,
    nodes: NodeMap<D>,
    edges: EdgeMap,

    /// Map from children to their parent's id.
    parents: ResourceMap<Option<ResourceId>>,
}

impl<D> ResourceTree<D>
where
    D: HasId<Id = ResourceId>,
{
    pub fn new(root: D) -> Self {
        let mut nodes = NodeMap::new();
        let node = ResourceNode::new(root);
        let root = node.id().clone();
        nodes.insert(root.clone(), node);

        let mut edges = EdgeMap::new();
        edges.insert(root.clone(), IndexSet::new());

        let mut parents = ResourceMap::new();
        parents.insert(root.clone(), None);

        Self {
            root,
            nodes,
            edges,
            parents,
        }
    }

    /// Create a tree from nodes and edges.
    ///
    /// # Errors
    /// + If the nodes and edges do not create a valid tree.
    pub fn from_parts(nodes: NodeMap<D>, edges: EdgeMap) -> Result<Self> {
        let mut parents = ResourceMap::new();
        let mut root = nodes
            .keys()
            .map(|id| id.clone())
            .collect::<HashSet<ResourceId>>();

        // compute parents, find root.
        for (id, node) in nodes.iter() {
            let Some(children) = edges.get(&id) else {
                return Err(GraphError::invalid_graph("node does not have edge map").into());
            };

            for child in children {
                parents.insert(child.clone(), Some(node.id().clone()));
                root.remove(child);
            }
        }

        let root = match root.len() {
            0 => return Err(GraphError::invalid_graph("root `Node` not found").into()),
            1 => {
                let Some(root) = root.into_iter().next() else {
                    return Err(GraphError::invalid_graph("could not get root").into());
                };

                root
            }
            _ => return Err(GraphError::invalid_graph("multiple root `Node`s found").into()),
        };

        parents.insert(root.clone(), None);

        Ok(Self {
            root,
            nodes,
            edges,
            parents,
        })
    }

    /// Get the id of the root of the tree.
    pub fn root(&self) -> &ResourceId {
        &self.root
    }

    pub fn nodes(&self) -> &NodeMap<D> {
        &self.nodes
    }

    pub fn edges(&self) -> &EdgeMap {
        &self.edges
    }

    /// Returns an iterator over the graph's nodes.
    pub fn iter_nodes(&self) -> Iter<ResourceId, ResourceNode<D>> {
        self.nodes.iter()
    }

    /// Returns a `mut`able iterator over the graph's nodes.
    pub fn iter_nodes_mut(&mut self) -> IterMut<ResourceId, ResourceNode<D>> {
        self.nodes.iter_mut()
    }

    /// Get a [`Node`] by its id.
    pub fn get(&self, id: &ResourceId) -> Option<&ResourceNode<D>> {
        self.nodes.get(&id)
    }

    /// Get a `mut`able [`Node`] by its id.
    pub fn get_mut(&mut self, id: &ResourceId) -> Option<&mut ResourceNode<D>> {
        self.nodes.get_mut(&id)
    }

    /// Inserts a new node into the tree.
    ///
    /// # Arguments
    /// 1. Parent id.
    /// 2. Node data.
    ///
    /// # Errors
    /// + [`ResourceError`] if the `parent` node does not exist.
    pub fn insert(&mut self, parent: ResourceId, data: D) -> Result {
        // check parent exists
        if !self.nodes.contains_key(&parent) {
            return Err(ResourceError::does_not_exist("parent `Node` not found").into());
        }

        let Some(children) = self.edges.get_mut(&parent) else {
            return Err(ResourceError::does_not_exist("parent `Node` not found").into());
        };

        let node = ResourceNode::new(data);
        let id = node.id().clone();

        self.nodes.insert(id.clone(), node);
        children.insert(id.clone());
        self.edges.insert(id.clone(), IndexSet::new());
        self.parents.insert(id.clone(), Some(parent.clone()));

        Ok(())
    }

    /// Returns the children `Containers` of the parent, if found.
    ///
    /// # Returns
    /// `None` if the parent `Node` is not found.
    pub fn children(&self, parent: &ResourceId) -> Option<&IndexSet<ResourceId>> {
        self.edges.get(parent)
    }

    /// Get the leaves of the tree.
    pub fn leaves(&self) -> Vec<ResourceId> {
        self.edges
            .iter()
            .filter_map(|(container, children)| {
                if children.is_empty() {
                    Some(container.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Returns the parent of a `Node`.
    ///
    /// # Returns
    /// `None` if the `Node` is the graph root.
    ///
    /// # Errors
    /// + If the child does not exist.
    pub fn parent(&self, child: &ResourceId) -> StdResult<Option<&ResourceId>, ResourceError> {
        let Some(parent) = self.parents.get(&child) else {
            return Err(ResourceError::does_not_exist("`Node` not found"));
        };

        Ok(parent.as_ref())
    }

    /// Returns the siblings of a `Node`.
    ///
    /// # Returns
    /// `None` if the `Node` is not found.
    pub fn siblings(&self, node: &ResourceId) -> Option<Vec<ResourceId>> {
        let Ok(parent) = self.parent(node) else {
            // node not found
            return None;
        };

        let Some(parent) = parent else {
            // node is root
            return Some(Vec::new());
        };

        Some(
            self.children(parent)
                .unwrap()
                .iter()
                .map(|rid| rid.clone())
                .collect(),
        )
    }

    /// Returns the path of ancestors to the tree root.
    /// Begins with self.
    ///
    /// # Returns
    /// + Empty `Vec` if the `root` `Node` is not found.
    /// + `Vec` of the ancestor path to the tree root, beginning with self.
    pub fn ancestors(&self, root: &ResourceId) -> Vec<ResourceId> {
        let mut ancestors = Vec::new();
        let mut current = self.get(root).map(|node| node.id());
        while let Some(id) = current {
            ancestors.push(id.clone());
            current = self.parent(id).expect("parent not found");
        }

        return ancestors;
    }

    /// Returns all the descendants of the root.
    ///
    /// # Returns
    /// + `None` if the root `Node` does not exist.
    /// + Descendant ids, including `root`, otherwise.
    pub fn descendants(&self, root: &ResourceId) -> Option<HashSet<ResourceId>> {
        let mut descendants = HashSet::new();
        if !self.get(&root).is_some() {
            return None;
        }

        descendants.insert(root.clone());

        let Some(children) = self.children(root) else {
            return Some(descendants);
        };

        for child in children {
            if let Some(c_descendants) = self.descendants(child) {
                for did in c_descendants {
                    descendants.insert(did);
                }
            }
        }

        return Some(descendants);
    }

    /// Inserts a [`Tree`] as a subtree.
    pub fn insert_tree(&mut self, parent: &ResourceId, tree: Self) -> Result {
        // insert root
        let Some(p_edges) = self.edges.get_mut(&parent) else {
            return Err(ResourceError::does_not_exist("parent edges not found").into());
        };

        let root = tree.root().clone();
        p_edges.insert(root.clone());
        self.parents.insert(root, Some(parent.clone()));

        // insert tree
        let (nodes, edges) = tree.into_components();

        for (id, node) in nodes.into_iter() {
            self.nodes.insert(id, node);
        }

        for (parent, children) in edges.into_iter() {
            for child in children.clone() {
                self.parents.insert(child, Some(parent.clone()));
            }

            self.edges.insert(parent, children);
        }

        Ok(())
    }

    /// Removes a subtree.
    ///
    /// # Returns
    /// The removed subtree.
    pub fn remove(&mut self, root: &ResourceId) -> Result<Self> {
        let Some(parent) = self.parent(root)?.cloned() else {
            return Err(GraphError::IllegalOperation("can not remove root".into()).into());
        };

        let (nodes, edges) = self.remove_components(root)?;
        let p_edges = self.edges.get_mut(&parent).unwrap();
        p_edges.shift_remove(root);

        let mut parents = ResourceMap::new();
        for (parent, children) in edges.clone() {
            for child in children {
                parents.insert(child.clone(), Some(parent.clone()));
            }
        }

        Ok(Self {
            root: root.clone(),
            nodes,
            edges,
            parents,
        })
    }

    /// Moves a subtree to a different parent.
    ///
    /// # Errors
    /// + If root `Node` does not exist.
    /// + If the new parent does not exist.
    pub fn mv(&mut self, root: &ResourceId, parent: &ResourceId) -> Result {
        // remove from original parent
        let Some(Some(o_parent)) = self.parents.get(&root) else {
            return Err(ResourceError::does_not_exist("parent `Node` does not exist").into());
        };

        let Some(op_edges) = self.edges.get_mut(o_parent) else {
            return Err(ResourceError::does_not_exist("`Node` edges do not exist").into());
        };

        op_edges.shift_remove(root);

        // add to new parent
        let Some(np_edges) = self.edges.get_mut(parent) else {
            return Err(ResourceError::does_not_exist("`Node` edges do not exist").into());
        };

        np_edges.insert(root.clone());
        self.parents.insert(root.clone(), Some(parent.clone()));

        Ok(())
    }

    /// Sets the index of a `Node` amongst its parent's children.
    ///
    /// # See also
    /// + Follows the rules of [`indexset::IndexSet::move_index`].
    pub fn move_index(&mut self, node: &ResourceId, index: usize) -> Result {
        let Some(Some(parent)) = self.parents.get(&node) else {
            return Err(ResourceError::does_not_exist("`Node` parent does not exist").into());
        };

        let Some(edges) = self.edges.get_mut(parent) else {
            return Err(ResourceError::does_not_exist("`Node` edges do not exist").into());
        };

        let Some(curr_index) = edges.get_index_of(node) else {
            return Err(ResourceError::does_not_exist("`Node` edges do not exist").into());
        };

        edges.move_index(curr_index, index);
        Ok(())
    }

    /// Consumes self, returning the graph's nodes and edges.
    pub fn into_components(self) -> (NodeMap<D>, EdgeMap) {
        (self.nodes, self.edges)
    }

    /// Recursively removes a subtree.
    fn remove_components(&mut self, root: &ResourceId) -> Result<(NodeMap<D>, EdgeMap)> {
        let mut nodes = NodeMap::new();
        let mut edges = EdgeMap::new();

        // remove root node
        let Some(node) = self.nodes.remove(root) else {
            return Err(ResourceError::does_not_exist("`Node` does not exist").into());
        };

        self.parents.remove(node.id());
        nodes.insert(node.id().clone(), node);

        // remove children
        let Some(children) = self.edges.remove(&root) else {
            return Err(ResourceError::does_not_exist("`Node` edges do not exist").into());
        };

        for child in children.iter() {
            let (c_nodes, c_edges) = self.remove_components(&child)?;

            for (c_id, c_node) in c_nodes.into_iter() {
                nodes.insert(c_id, c_node);
            }

            for (parent, p_edges) in c_edges.into_iter() {
                edges.insert(parent, p_edges);
            }
        }

        edges.insert(root.clone(), children);

        Ok((nodes, edges))
    }
}

impl<D> ResourceTree<D>
where
    D: HasId<Id = ResourceId> + Clone,
{
    /// Clones a subtree.
    pub fn clone_tree(&self, root: &ResourceId) -> Result<Self> {
        let Some(root_node) = self.nodes.get(&root) else {
            return Err(ResourceError::does_not_exist("root `Node` not found").into());
        };

        let mut tree = Self::new(root_node.clone().into_data());
        for child in self.children(root).expect("children not found") {
            tree.insert_tree(root, self.clone_tree(child)?)?;
        }

        Ok(tree)
    }
}

impl<D> fmt::Debug for ResourceTree<D>
where
    D: fmt::Debug + HasId<Id = ResourceId>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}\n{:?}", self.nodes(), self.edges())
    }
}

impl ResourceTree<Container> {
    /// Get a node by its path.
    /// The path is dictated by the container's name.
    pub fn get_path(
        &self,
        path: impl AsRef<Path>,
    ) -> StdResult<Option<&ResourceNode<Container>>, InvalidPath> {
        let mut components = path.as_ref().components();
        let Some(component) = components.next() else {
            return Err(InvalidPath);
        };

        if !matches!(component, Component::RootDir) {
            return Err(InvalidPath);
        }

        let mut node_id = &self.root;
        while let Some(component) = components.next() {
            let Component::Normal(name) = component else {
                return Err(InvalidPath);
            };

            let Some(child) = self.children(node_id).unwrap().iter().find(|child| {
                let node = self.get(child).unwrap();
                node.properties.name == name.to_str().unwrap()
            }) else {
                return Ok(None);
            };

            node_id = child;
        }

        Ok(Some(self.get(node_id).unwrap()))
    }

    /// Get the path of a node.
    pub fn path(&self, node: &ResourceId) -> Option<PathBuf> {
        let ancestors = self.ancestors(node);
        if ancestors.is_empty() {
            return None;
        }
        if let [ancestor] = &ancestors[..] {
            let root = self.get(ancestor).unwrap();
            assert_eq!(root.rid(), &self.root);
            return Some(PathBuf::from(Component::RootDir.as_os_str()));
        }

        let path = ancestors
            .iter()
            .take(ancestors.len() - 1)
            .map(|node| {
                let node = self.get(node).unwrap();
                let container = node.data();
                Component::Normal(std::ffi::OsStr::new(&container.properties.name))
            })
            .chain(std::iter::once(Component::RootDir))
            .rev()
            .collect();

        Some(path)
    }
}

#[derive(Debug)]
pub struct InvalidPath;

#[cfg(test)]
#[path = "./tree_test.rs"]
mod tree_test;
