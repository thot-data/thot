use crate::{graph, project::Container};
use std::{
    iter,
    path::{self, PathBuf},
    sync::Arc,
};

pub type Node = Arc<Container>;
pub type Children = Vec<Node>;

pub struct Tree {
    graph: Vec<(Node, Children)>,
    root: Node,

    /// (child, parent) relationships.
    /// Root node is not included.
    parents: Vec<(Node, Node)>,
}

impl Tree {
    /// Iterator over all nodes.
    pub fn nodes(&self) -> Vec<Node> {
        self.graph
            .iter()
            .map(|(parent, _)| parent.clone())
            .collect()
    }

    pub fn root(&self) -> &Node {
        &self.root
    }

    /// Get the direct children of the parent node.
    pub fn children(&self, parent: &Node) -> Option<&Children> {
        self.graph
            .iter()
            .find_map(|(p, children)| Node::ptr_eq(p, parent).then_some(children))
    }

    /// Get all descendants of the root node.
    ///
    /// # Returns
    /// Empty `Vec` if the root was not found.
    pub fn descendants(&self, root: &Node) -> Vec<Node> {
        if self
            .graph
            .iter()
            .find_map(|(parent, _)| Node::ptr_eq(parent, root).then_some(parent.clone()))
            .is_none()
        {
            return vec![];
        };

        let mut descendants = self
            .children(root)
            .unwrap()
            .iter()
            .flat_map(|children| {
                let descendants = self.descendants(children);
                assert!(!descendants.is_empty());
                descendants
            })
            .collect::<Vec<_>>();

        descendants.insert(0, root.clone());
        descendants
    }

    /// Ancestor chain beginning at `root`.
    ///
    /// # Returns
    /// Empty `Vec` if `root` is not found.
    pub fn ancestors(&self, root: &Node) -> Vec<Node> {
        if Node::ptr_eq(&self.root, root) {
            return vec![root.clone()];
        }

        let Some(parent) = self
            .parents
            .iter()
            .find_map(|(child, parent)| Node::ptr_eq(child, root).then_some(parent))
        else {
            return vec![];
        };

        let mut ancestors = self.ancestors(parent);
        assert!(!ancestors.is_empty());

        ancestors.insert(0, root.clone());
        ancestors
    }

    /// # Returns
    /// Path to the node comprised of names of its ancestors.
    /// Root node is indicated by the root directory path.
    pub fn path(&self, root: &Node) -> Option<PathBuf> {
        let ancestors = self.ancestors(root);
        if ancestors.is_empty() {
            return None;
        }

        let path = iter::once(
            path::Component::RootDir
                .as_os_str()
                .to_string_lossy()
                .to_string(),
        )
        .chain(
            ancestors
                .into_iter()
                .rev()
                .skip(1)
                .map(|container| container.properties.name.clone()),
        )
        .collect::<PathBuf>();

        Some(path)
    }
}

impl From<graph::ResourceTree<Container>> for Tree {
    fn from(value: graph::ResourceTree<Container>) -> Self {
        use std::collections::HashMap;

        let root = value.root().clone();
        let (nodes, edges) = value.into_components();

        let nodes = nodes
            .into_values()
            .map(|node| Node::new(node.into_data()))
            .collect::<Vec<_>>();

        let node_map = nodes
            .iter()
            .map(|node| {
                let rid = node.rid();
                let idx = nodes.iter().position(|other| other.rid() == rid).unwrap();
                (rid, idx)
            })
            .collect::<HashMap<_, _>>();

        let graph = edges
            .iter()
            .map(|(parent, children)| {
                let parent_idx = node_map.get(parent).unwrap().clone();
                let parent = nodes[parent_idx].clone();

                let children = children
                    .iter()
                    .map(|rid| {
                        let idx = node_map.get(rid).unwrap().clone();
                        nodes[idx].clone()
                    })
                    .collect::<Vec<_>>();

                (parent, children)
            })
            .collect::<Vec<_>>();

        let root = node_map.get(&root).unwrap().clone();
        let root = nodes[root].clone();

        let parents = graph
            .iter()
            .flat_map(|(parent, children)| {
                children.iter().map(|child| (child.clone(), parent.clone()))
            })
            .collect();

        Self {
            graph,
            root,
            parents,
        }
    }
}

impl From<&graph::ResourceTree<Container>> for Tree {
    fn from(value: &graph::ResourceTree<Container>) -> Self {
        use std::collections::HashMap;

        let nodes = value
            .nodes()
            .values()
            .map(|node| Node::new(node.data().clone()))
            .collect::<Vec<_>>();

        let node_map = value
            .nodes()
            .keys()
            .map(|rid| {
                let idx = nodes.iter().position(|node| node.rid() == rid).unwrap();
                (rid, idx)
            })
            .collect::<HashMap<_, _>>();

        let graph = value
            .edges()
            .iter()
            .map(|(parent, children)| {
                let parent_idx = node_map.get(parent).unwrap().clone();
                let parent = nodes[parent_idx].clone();

                let children = children
                    .iter()
                    .map(|rid| {
                        let idx = node_map.get(rid).unwrap().clone();
                        nodes[idx].clone()
                    })
                    .collect::<Vec<_>>();

                (parent, children)
            })
            .collect::<Vec<_>>();

        let root = node_map.get(value.root()).unwrap().clone();
        let root = nodes[root].clone();

        let parents = node_map
            .iter()
            .filter_map(|(rid, child_idx)| {
                value.parent(rid).unwrap().map(|parent| {
                    let parent = node_map.get(parent).unwrap().clone();
                    let parent = nodes[parent].clone();
                    let child = nodes[child_idx.clone()].clone();
                    (child, parent)
                })
            })
            .collect();

        Self {
            graph,
            root,
            parents,
        }
    }
}
