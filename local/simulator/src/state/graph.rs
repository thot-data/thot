use super::{HasName, Ptr, WPtr};
use has_id::HasId;
use std::path::{Path, PathBuf};

pub type Node<T> = Ptr<T>;
pub type NodeRef<T> = WPtr<T>;

/// Node map of (original, new).
pub type NodeMap<T> = Vec<(Ptr<T>, Ptr<T>)>;

pub struct Tree<D> {
    root: NodeRef<D>,
    nodes: Vec<Node<D>>,
    children: Vec<(NodeRef<D>, Vec<NodeRef<D>>)>,
    parents: Vec<(NodeRef<D>, NodeRef<D>)>,
}

impl<D> Tree<D> {
    pub fn new(root: D) -> Self {
        let root = Ptr::new(root);
        let children = vec![(Node::downgrade(&root), vec![])];

        Self {
            root: Node::downgrade(&root),
            nodes: vec![root],
            children,
            parents: vec![],
        }
    }

    pub fn root(&self) -> Node<D> {
        self.root.upgrade().unwrap()
    }

    pub fn nodes(&self) -> &Vec<Node<D>> {
        &self.nodes
    }

    pub fn contains(&self, node: &Node<D>) -> bool {
        self.nodes.iter().any(|n| Node::ptr_eq(&node, n))
    }

    pub fn insert(&mut self, node: D, parent: &Node<D>) -> Result<Node<D>, error::Insert<D>> {
        let parent = self
            .nodes
            .iter()
            .find(|node| Node::ptr_eq(&node, &parent))
            .ok_or(error::Insert::InvalidParent)?;

        let (_, children) = self
            .children
            .iter_mut()
            .find(|(p, _)| {
                if let Some(p) = p.upgrade() {
                    Node::ptr_eq(&p, parent)
                } else {
                    false
                }
            })
            .unwrap();

        let node = Ptr::new(node);
        let node_weak = Node::downgrade(&node);
        if children.iter().any(|child| child.ptr_eq(&node_weak)) {
            return Err(error::Insert::AlreadyContains(node_weak));
        }
        children.push(node_weak.clone());
        self.children.push((node_weak.clone(), vec![]));
        self.parents
            .push((node_weak.clone(), Node::downgrade(parent)));

        self.nodes.push(node.clone());
        Ok(node)
    }

    /// Inserts a tree at the given node.
    pub fn insert_tree(&mut self, tree: Self, parent: &Node<D>) -> Result<(), error::Insert<D>> {
        if !self.contains(parent) {
            return Err(error::Insert::InvalidParent);
        }

        for node in tree.nodes() {
            if self.nodes.iter().any(|n| Node::ptr_eq(n, node)) {
                return Err(error::Insert::AlreadyContains(Node::downgrade(node)));
            }
        }

        let Self {
            root,
            nodes,
            children,
            parents,
        } = tree;

        let parent = Node::downgrade(parent);
        self.nodes.extend(nodes);
        self.children.extend(children);
        self.parents.extend(parents);
        self.parents.push((root.clone(), parent.clone()));
        let (_, children) = self
            .children
            .iter_mut()
            .find(|(p, _)| parent.ptr_eq(p))
            .unwrap();

        children.push(root);
        Ok(())
    }

    /// Removes a sub tree.
    ///
    /// # Panics
    /// + If the graph's root node is being removed.
    pub fn remove(&mut self, root: &Node<D>) -> Option<Self> {
        if Node::ptr_eq(root, &self.root()) {
            panic!("can not remove root node");
        }

        let descendants = self
            .descendants(root)
            .into_iter()
            .map(|descendant| Node::downgrade(&descendant))
            .collect::<Vec<_>>();

        if descendants.is_empty() {
            return None;
        }

        let mut nodes = Vec::with_capacity(descendants.len());
        let mut children = Vec::with_capacity(descendants.len());
        let mut parents = Vec::with_capacity(descendants.len());
        for descendant in descendants {
            let index = self
                .parents
                .iter()
                .position(|(child, _)| descendant.ptr_eq(child))
                .unwrap();

            parents.push(self.parents.swap_remove(index));

            let index = self
                .children
                .iter()
                .position(|(parent, _)| descendant.ptr_eq(parent))
                .unwrap();

            children.push(self.children.swap_remove(index));

            let index = self
                .nodes
                .iter()
                .position(|n| descendant.ptr_eq(&Node::downgrade(n)))
                .unwrap();

            nodes.push(self.nodes.swap_remove(index));
        }

        // remove root from parent's children in original graph
        let root_parent = parents
            .iter()
            .find_map(|(child, parent)| {
                if child.ptr_eq(&Node::downgrade(root)) {
                    Some(parent)
                } else {
                    None
                }
            })
            .unwrap();

        let parent_children = self
            .children
            .iter_mut()
            .find_map(|(p, children)| {
                if p.ptr_eq(&root_parent) {
                    Some(children)
                } else {
                    None
                }
            })
            .unwrap();
        let index = parent_children
            .iter()
            .position(|child| child.ptr_eq(&Node::downgrade(root)))
            .unwrap();

        parent_children.swap_remove(index);

        // remove root's parent in duplicated graph
        let root = Node::downgrade(root);
        let index = parents
            .iter()
            .position(|(child, _)| root.ptr_eq(child))
            .unwrap();

        parents.swap_remove(index);

        Some(Self {
            root,
            nodes,
            children,
            parents,
        })
    }

    pub fn parent(&self, child: &Node<D>) -> Option<Node<D>> {
        let child = Node::downgrade(child);
        let (_, parent) = self.parents.iter().find(|(c, _)| child.ptr_eq(c))?;
        Some(parent.upgrade().unwrap())
    }

    pub fn children(&self, parent: &Node<D>) -> Option<Vec<Node<D>>> {
        let parent = Node::downgrade(parent);
        let (_, children) = self.children.iter().find(|(p, _)| parent.ptr_eq(p))?;
        let children = children
            .into_iter()
            .map(|child| child.upgrade().unwrap())
            .collect();

        Some(children)
    }

    /// Gets descendants of the given root node.
    ///
    /// # Returns
    /// + List of all descendants, including self.
    /// + Empty if root is not in the graph.
    pub fn descendants(&self, root: &Node<D>) -> Vec<Node<D>> {
        if !self.contains(root) {
            return vec![];
        }

        let mut descendants = self
            .children(root)
            .unwrap()
            .clone()
            .iter()
            .flat_map(|child| self.descendants(child))
            .collect::<Vec<_>>();

        descendants.push(root.clone());
        descendants
    }

    /// Get the ancestors or the given node.
    ///
    /// # Returns
    /// + Ordered list of ancestors, beginning with the given node.
    /// + Empty if node does not exist in the graph.
    pub fn ancestors(&self, child: &Node<D>) -> Vec<&Node<D>> {
        let mut child = Node::downgrade(child);
        let this = child.upgrade().unwrap();
        let Some(this) = self.nodes.iter().find(|n| Node::ptr_eq(&this, n)) else {
            return vec![];
        };

        let mut ancestors = vec![];
        ancestors.push(this);
        while let Some((_, parent)) = self.parents.iter().find(|(c, _)| child.ptr_eq(c)) {
            child = parent.clone();
            let parent = parent.upgrade().unwrap();
            let parent = self
                .nodes
                .iter()
                .find(|n| Node::ptr_eq(&parent, n))
                .unwrap();
            ancestors.push(parent);
        }

        ancestors
    }
}

impl<D> Tree<D>
where
    D: Clone + std::fmt::Debug,
{
    pub fn duplicate(&self) -> Self {
        let (dup, _) = self.duplicate_with_map();
        dup
    }

    /// Duplicates the tree.
    ///
    /// # Returns
    /// Tuple of (duplicate, [(original node, duplicate node)])
    pub fn duplicate_with_map(&self) -> (Self, NodeMap<D>) {
        fn get_mapped<'a, D>(needle: &'a Node<D>, map: &'a NodeMap<D>) -> Option<&'a Node<D>>
        where
            D: Clone,
        {
            map.iter().find_map(|(from, to)| {
                if Node::ptr_eq(from, &needle) {
                    return Some(to);
                }

                None
            })
        }

        let mut node_map = Vec::with_capacity(self.nodes.len());
        let nodes = self
            .nodes
            .iter()
            .map(|node| {
                let node_clone = Ptr::new(node.borrow().clone());
                node_map.push((node.clone(), node_clone.clone()));
                node_clone
            })
            .collect();

        let root = self.root.upgrade().unwrap();
        let root = get_mapped(&root, &node_map).unwrap();
        let root = Ptr::downgrade(root);

        let children = self
            .children
            .iter()
            .map(|(parent, children)| {
                let parent = parent.upgrade().unwrap();
                let children = children
                    .iter()
                    .map(|child| {
                        let child = child.upgrade().unwrap();
                        let to = get_mapped(&child, &node_map).unwrap();
                        Ptr::downgrade(to)
                    })
                    .collect();

                let parent = get_mapped(&parent, &node_map).unwrap();
                let parent = Ptr::downgrade(parent);

                (parent, children)
            })
            .collect();

        let parents = self
            .parents
            .iter()
            .map(|(child, parent)| {
                let child = child.upgrade().unwrap();
                let parent = parent.upgrade().unwrap();

                let child = get_mapped(&child, &node_map).unwrap();
                let parent = get_mapped(&parent, &node_map).unwrap();
                (Ptr::downgrade(&child), Ptr::downgrade(&parent))
            })
            .collect();

        (
            Self {
                nodes,
                root,
                children,
                parents,
            },
            node_map,
        )
    }

    pub fn duplicate_subtree(&self, root: &Node<D>) -> Result<Self, super::error::Error> {
        let (graph, _) = self.duplicate_subtree_with_map(root)?;
        Ok(graph)
    }

    pub fn duplicate_subtree_with_map(
        &self,
        root: &Node<D>,
    ) -> Result<(Self, NodeMap<D>), super::error::Error> {
        use super::find_mapped_to;

        if !self.contains(root) {
            return Err(super::error::Error::DoesNotExist);
        }

        let (nodes, node_map): (Vec<_>, Vec<_>) = self
            .descendants(root)
            .into_iter()
            .map(|node| {
                let dup = Node::new(node.borrow().clone());
                (dup.clone(), (node, dup))
            })
            .unzip();

        let root_dup = find_mapped_to(root, &node_map).unwrap();
        let children = self
            .children
            .iter()
            .filter_map(|(parent, children)| {
                let parent = parent.upgrade().unwrap();
                let parent = find_mapped_to(&parent, &node_map)?;
                let parent = Ptr::downgrade(parent);

                let children = children
                    .iter()
                    .map(|child| {
                        let child = child.upgrade().unwrap();
                        let child = find_mapped_to(&child, &node_map).unwrap();
                        Ptr::downgrade(child)
                    })
                    .collect();

                Some((parent, children))
            })
            .collect();

        let parents = self
            .parents
            .iter()
            .filter_map(|(child, parent)| {
                let child = child.upgrade().unwrap();
                let parent = parent.upgrade().unwrap();
                if Ptr::ptr_eq(&child, root) {
                    return None;
                }

                let child = find_mapped_to(&child, &node_map)?;
                let parent = find_mapped_to(&parent, &node_map).unwrap();

                Some((Ptr::downgrade(child), Ptr::downgrade(parent)))
            })
            .collect();

        let graph = Self {
            root: Ptr::downgrade(&root_dup),
            nodes,
            children,
            parents,
        };

        Ok((graph, node_map))
    }
}

impl<D> Tree<D>
where
    D: HasId,
{
    pub fn find(&self, id: &<D as HasId>::Id) -> Option<&Node<D>> {
        self.nodes
            .iter()
            .find(|node| <D as HasId>::id(&node.borrow()) == id)
    }
}

impl<D> Tree<D>
where
    D: HasName,
{
    /// Get the path from the root of the graph to the given node.
    pub fn path(&self, node: &Node<D>) -> Option<PathBuf> {
        let mut ancestors = self.ancestors(node);
        if ancestors.is_empty() {
            return None;
        }

        ancestors.reverse();
        Some(
            ancestors
                .iter()
                .fold(PathBuf::new(), |path, node| path.join(node.borrow().name())),
        )
    }

    pub fn paths(&self, root: &Node<D>) -> Option<Vec<PathBuf>> {
        fn inner<D>(graph: &Tree<D>, root: &Node<D>) -> Vec<PathBuf>
        where
            D: HasName,
        {
            let root_path = PathBuf::from(root.borrow().name());
            let mut paths = graph
                .children(root)
                .unwrap()
                .iter()
                .flat_map(|child| {
                    inner(graph, child)
                        .into_iter()
                        .map(|path| root_path.join(path))
                })
                .collect::<Vec<_>>();

            paths.push(root_path);
            paths
        }

        let root_path = self.path(root)?;
        let root_path = root_path.parent().unwrap();
        Some(
            inner(self, root)
                .into_iter()
                .map(|path| root_path.join(path))
                .collect(),
        )
    }

    pub fn all_paths(&self) -> Vec<PathBuf> {
        self.paths(&self.root()).unwrap()
    }

    pub fn find_by_path(&self, path: impl AsRef<Path>) -> Option<Ptr<D>> {
        let path = path.as_ref();
        let mut node = self.root();
        if path.as_os_str() == node.borrow().name() {
            return Some(node);
        }

        for component in path.components() {
            match component {
                std::path::Component::Normal(name) => {
                    node = self
                        .children(&node)
                        .unwrap()
                        .iter()
                        .find(|child| child.borrow().name() == name)?
                        .clone();
                }

                std::path::Component::RootDir | std::path::Component::CurDir => {}
                _ => panic!("invalid path component"),
            }
        }

        Some(node)
    }

    pub fn insert_at(
        &mut self,
        node: D,
        parent: impl AsRef<Path>,
    ) -> Result<Node<D>, error::Insert<D>> {
        let Some(parent) = self.find_by_path(parent) else {
            return Err(error::Insert::InvalidParent);
        };

        self.insert(node, &parent)
    }
}

impl<D> std::fmt::Debug for Tree<D>
where
    D: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("Tree")
            .field("root", &self.root())
            .field("nodes", &self.nodes)
            .field("children", &self.children)
            .finish()
    }
}

pub mod error {
    use super::*;

    #[derive(Debug)]
    pub enum Insert<D> {
        /// The given parent does not exist in the tree.
        InvalidParent,

        /// The given node is already in the tree.
        AlreadyContains(NodeRef<D>),
    }
}

#[cfg(test)]
#[path = "graph_test.rs"]
mod graph_test;
