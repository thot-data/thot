use super::*;
use std::ffi::OsString;

#[test]
fn graph_should_work() {
    let root = Data::new(0);
    let mut graph = Tree::new(root);

    let child = Data::new(1);
    let id_1 = child.id().clone();
    graph.insert(child, &graph.root()).unwrap();
    let children = graph.children(&graph.root()).unwrap();
    assert_eq!(children.len(), 1);
    assert!(children.iter().any(|c| c.borrow().id() == &id_1));

    let child = Data::new(2);
    let id_2 = child.id().clone();
    graph.insert(child, &graph.root()).unwrap();
    let children = graph.children(&graph.root()).unwrap();
    assert_eq!(children.len(), 2);
    assert!(children.iter().any(|c| c.borrow().id() == &id_1));
    assert!(children.iter().any(|c| c.borrow().id() == &id_2));

    let parent = graph.find(&id_1).unwrap().clone();
    let child = Data::new(11);
    let id_11 = child.id().clone();
    graph.insert(child, &parent).unwrap();
    let children = graph.children(&parent).unwrap();
    assert_eq!(graph.children(&graph.root()).unwrap().len(), 2);
    assert_eq!(children.len(), 1);
    assert!(children.iter().any(|c| c.borrow().id() == &id_11));

    let child = Data::new(12);
    let id_12 = child.id().clone();
    graph.insert(child, &parent).unwrap();
    let children = graph.children(&parent).unwrap();
    assert_eq!(children.len(), 2);
    assert!(children.iter().any(|c| c.borrow().id() == &id_11));
    assert!(children.iter().any(|c| c.borrow().id() == &id_12));

    let ancestors = graph.ancestors(&parent);
    assert_eq!(ancestors.len(), 2);
    assert!(Node::ptr_eq(ancestors[0], &parent));
    assert!(Node::ptr_eq(ancestors[1], &graph.root()));

    let descendants = graph.descendants(&parent);
    assert_eq!(descendants.len(), 3);

    let removed = graph.remove(&parent).unwrap();
    assert_eq!(removed.nodes().len(), 3);
    assert!(!graph.contains(&parent));
}

#[test_log::test]
fn graph_duplicate_shoud_work() {
    let root = Data::new(0);
    let mut graph = Tree::new(root);

    let child = Data::new(1);
    let id_1 = child.id().clone();
    graph.insert(child, &graph.root()).unwrap();

    let child = Data::new(2);
    let id_2 = child.id().clone();
    graph.insert(child, &graph.root()).unwrap();

    let parent = graph.find(&id_1).unwrap().clone();
    let child = Data::new(11);
    let id_11 = child.id().clone();
    graph.insert(child, &parent).unwrap();

    let child = Data::new(12);
    let id_12 = child.id().clone();
    graph.insert(child, &parent).unwrap();

    let dup = graph.duplicate();
    assert_eq!(dup.nodes().len(), graph.nodes().len());
    assert_eq!(dup.root().borrow().id(), graph.root().borrow().id());
    assert_eq!(
        dup.children(&dup.root()).unwrap().len(),
        graph.children(&graph.root()).unwrap().len(),
    );
}

#[test]
fn graph_path_should_work() {
    let root = Data::new(0);
    let mut graph = Tree::new(root);

    let child = Data::new(1);
    let id_1 = child.id().clone();
    graph.insert(child, &graph.root()).unwrap();

    let child = Data::new(2);
    let id_2 = child.id().clone();
    graph.insert(child, &graph.root()).unwrap();

    let parent = graph.find(&id_1).unwrap().clone();
    let child = Data::new(11);
    let id_11 = child.id().clone();
    graph.insert(child, &parent).unwrap();

    let child = Data::new(12);
    let id_12 = child.id().clone();
    graph.insert(child, &parent).unwrap();

    let root = graph.find(&id_1).unwrap().clone();
    let paths = graph.paths(&root).unwrap();
    assert!(paths.contains(&PathBuf::from("0/1")));
    assert!(paths.contains(&PathBuf::from("0/1/11")));
    assert!(paths.contains(&PathBuf::from("0/1/12")));

    let paths = graph.all_paths();
    assert!(paths.contains(&PathBuf::from("0")));
    assert!(paths.contains(&PathBuf::from("0/1")));
    assert!(paths.contains(&PathBuf::from("0/1/11")));
    assert!(paths.contains(&PathBuf::from("0/1/12")));
    assert!(paths.contains(&PathBuf::from("0/2")));
}

#[derive(Debug, HasId, Clone)]
struct Data {
    #[id]
    id: usize,

    #[allow(dead_code)]
    inner: u32,

    name: OsString,
}

impl Data {
    fn new(data: u32) -> Self {
        Self {
            id: rand::random(),
            inner: data,
            name: data.to_string().into(),
        }
    }
}

impl HasName for Data {
    fn name(&self) -> &std::ffi::OsStr {
        &self.name
    }

    fn set_name(&mut self, name: impl Into<std::ffi::OsString>) {
        self.name = name.into()
    }
}
