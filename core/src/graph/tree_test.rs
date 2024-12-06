use super::*;
use has_id::HasId;
use rand::Rng;

// *************
// *** tests ***
// *************

#[test]
fn new_should_work() {
    let root = Data::new();
    ResourceTree::new(root);
}

#[test]
fn from_components_should_work() {
    // setup
    let tree = create_tree();
    let root = tree.root().clone();
    let root_children = tree
        .children(tree.root())
        .expect("could not get root children")
        .clone();

    let c1 = root_children
        .get_index(0)
        .expect("could not get root child");

    let c2 = root_children
        .get_index(1)
        .expect("could not get root child");

    let c1_children = tree.children(&c1).expect("could not get children").clone();
    let c2_children = tree.children(&c2).expect("could not get children").clone();
    let (nodes, edges) = tree.into_components();

    // test
    let tree =
        ResourceTree::from_parts(nodes, edges).expect("could not create tree from components");

    assert_eq!(&root, tree.root(), "incorrect root found");
    assert!(
        tree.parent(tree.root())
            .expect("root `Node` not in graph")
            .is_none(),
        "root should not have parent"
    );

    assert_eq!(
        &root_children,
        tree.children(tree.root())
            .expect("could not get root children"),
        "incorrect root children"
    );

    assert_eq!(
        &c1_children,
        tree.children(&c1).expect("could not get children"),
        "incorrect root children"
    );

    assert_eq!(
        &c2_children,
        tree.children(&c2).expect("could not get children"),
        "incorrect root children"
    );
}

#[test]
fn insert_should_work() {
    // setup
    let root = Data::new();
    let mut tree = ResourceTree::new(root);

    let c1 = Data::new();
    let c2 = Data::new();
    let c11 = Data::new();

    let root = tree.root().clone();
    let c1_id = c1.id().clone();
    let c2_id = c2.id().clone();
    let c11_id = c11.id().clone();

    // test
    tree.insert(root.clone(), c1)
        .expect("could not insert root child `Node`");

    assert!(
        tree.children(&root)
            .expect("root `Node` not found")
            .contains(&c1_id),
        "root `Node` should have edge to child `Node`"
    );

    assert_eq!(
        Some(&root),
        tree.parent(&c1_id).expect("parent `Node` not found"),
        "child `Node` parent incorrect"
    );

    tree.insert(root.clone(), c2)
        .expect("could not insert root child `Node`");

    assert!(
        tree.children(&root)
            .expect("root `Node` not found")
            .contains(&c2_id),
        "root `Node` should have edge to child `Node`"
    );

    assert_eq!(
        Some(&root),
        tree.parent(&c2_id).expect("parent `Node` not found"),
        "child `Node` parent incorrect"
    );

    tree.insert(c1_id.clone(), c11)
        .expect("could not insert grandchild `Node`");

    assert!(
        tree.children(&c1_id)
            .expect("child `Node` not found")
            .contains(&c11_id),
        "child `Node` should have edge to grandchild `Node`"
    );

    assert_eq!(
        Some(&c1_id),
        tree.parent(&c11_id).expect("parent `Node` not found"),
        "child `Node` parent incorrect"
    );

    assert!(
        !tree
            .children(&root)
            .expect("root `Node` not found")
            .contains(&c11_id),
        "root `Node` should not have edge to grandchild `Node`"
    );
}

#[test]
fn get_should_work() {
    // setup
    let root = Data::new();
    let data_0 = root.inner().clone();
    let tree = ResourceTree::new(root);
    let root = tree.root();

    // test
    let f_root = tree.get(root).expect("root `Node` should exist");
    assert_eq!(&data_0, f_root.inner(), "root data incorrect");
}

#[test]
fn root_should_work() {
    // setup
    let root = Data::new();
    let data_0 = root.inner().clone();
    let tree = ResourceTree::new(root);

    // test
    let root_id = tree.root();
    let f_root = tree.get(root_id).expect("root `Node` should exist");
    assert_eq!(&data_0, f_root.inner(), "root data incorrect");
}

#[test]
fn ancestors_should_work() {
    // setup
    let root = Data::new();
    let mut tree = ResourceTree::new(root);

    let c1 = Data::new();
    let c2 = Data::new();
    let c11 = Data::new();

    let c1_id = c1.id().clone();
    let c2_id = c2.id().clone();
    let c11_id = c11.id().clone();

    tree.insert(tree.root().clone(), c1)
        .expect("could not insert root child `Node`");

    tree.insert(tree.root().clone(), c2)
        .expect("could not insert root child `Node`");

    tree.insert(c1_id.clone(), c11)
        .expect("could not insert `Node`");

    // test
    let blank_ans = tree.ancestors(&ResourceId::new());
    assert_eq!(0, blank_ans.len(), "unfound node should have no ancestors");

    let root_ans = tree.ancestors(tree.root());
    assert_eq!(
        vec![tree.root().clone()],
        root_ans,
        "root ancestors should only be self"
    );

    let c1_ans = tree.ancestors(&c1_id);
    assert_eq!(2, c1_ans.len(), "incorrect ancestor path length");
    assert_eq!(c1_id, c1_ans[0], "ancestors should start with self");
    assert_eq!(tree.root(), &c1_ans[1], "ancestors should end with root");

    let c2_ans = tree.ancestors(&c2_id);
    assert_eq!(2, c2_ans.len(), "incorrect ancestor path length");
    assert_eq!(c2_id, c2_ans[0], "ancestors should start with self");
    assert_eq!(tree.root(), &c2_ans[1], "ancestors should end with root");

    let c11_ans = tree.ancestors(&c11_id);
    assert_eq!(3, c11_ans.len(), "incorrect ancestor path length");
    assert_eq!(c11_id, c11_ans[0], "ancestors should start with self");
    assert_eq!(c1_id, c11_ans[1], "ancestors should contain parent");
    assert_eq!(tree.root(), &c11_ans[2], "ancestors should end with root");
}

#[test]
fn descendants_should_work() {
    // setup
    let root = Data::new();
    let mut tree = ResourceTree::new(root);

    let c1 = Data::new();
    let c2 = Data::new();
    let c11 = Data::new();

    let c1_id = c1.id().clone();
    let c2_id = c2.id().clone();
    let c11_id = c11.id().clone();

    tree.insert(tree.root().clone(), c1)
        .expect("could not insert root child `Node`");

    tree.insert(tree.root().clone(), c2)
        .expect("could not insert root child `Node`");

    tree.insert(c1_id.clone(), c11)
        .expect("could not insert `Node`");

    // test
    let root_decs = tree
        .descendants(tree.root())
        .expect("could not get root descendants");

    assert_eq!(4, root_decs.len(), "incorrect number of descendants");
    assert!(
        root_decs.contains(tree.root()),
        "descendants should include self"
    );

    assert!(
        root_decs.contains(&c1_id),
        "descendants should include child"
    );

    assert!(
        root_decs.contains(&c2_id),
        "descendants should include child"
    );

    assert!(
        root_decs.contains(&c11_id),
        "descendants should include grandchild"
    );

    let c1_decs = tree
        .descendants(&c1_id)
        .expect("could not get child descendants");

    assert_eq!(2, c1_decs.len(), "incorrect number of descendants");
    assert!(c1_decs.contains(&c1_id), "descendants should include self");
    assert!(
        c1_decs.contains(&c11_id),
        "descendants should include child"
    );

    let c2_decs = tree
        .descendants(&c2_id)
        .expect("could not get child descendants");

    assert_eq!(1, c2_decs.len(), "incorrect number of descendants");
    assert!(c2_decs.contains(&c2_id), "descendants should include self");

    let c11_decs = tree
        .descendants(&c11_id)
        .expect("could not get grandchild descendants");

    assert_eq!(1, c11_decs.len(), "incorrect number of descendants");
    assert!(
        c11_decs.contains(&c11_id),
        "descendants should include self"
    );
}

#[test]
fn clone_tree_should_work() {
    // setup
    let tree = create_tree();
    let c1 = tree
        .children(tree.root())
        .expect("could not get root children")
        .get_index(0)
        .expect("root `Node` has no children");

    let children = tree.children(c1).expect("could not get children of `Node`");

    // test
    let c_tree = tree.clone_tree(c1).expect("could not clone tree");
    assert_eq!(c1, c_tree.root(), "root `Node`s not equal");

    let c_children = c_tree
        .children(c_tree.root())
        .expect("could not get children of root in cloned tree");
    assert_eq!(children.len(), c_children.len(), "children are not equal");
    for child in children {
        assert!(c_children.contains(child), "children are not equal");
    }
}

#[test]
fn insert_tree_should_work() {
    // setup
    let mut tree = create_tree();
    let sub_tree = create_tree();

    let root = tree.root().clone();
    let sub_root = sub_tree.root().clone();

    let c1_id = tree
        .children(&root)
        .expect("could not get children of root `Node`")
        .get_index(0)
        .expect("root `Node` does not have child")
        .clone();

    let sub_root_children = sub_tree
        .children(&sub_root)
        .expect("could not get children of root `Node`")
        .clone();

    // test
    tree.insert_tree(&c1_id, sub_tree)
        .expect("could not insert tree");

    assert!(
        tree.children(&c1_id)
            .expect("could not get `Node` children")
            .contains(&sub_root),
        "subtree not added"
    );

    let root_parent = tree.parent(tree.root()).expect("parent `Node` not found");

    assert!(root_parent.is_none());
    assert_eq!(
        Some(&c1_id),
        tree.parent(&sub_root).expect("`Node` parent not found"),
        "subtree root parent not correct"
    );

    let sub_children = tree
        .children(&sub_root)
        .expect("could not get `Node` children");

    for child in sub_root_children {
        assert!(sub_children.contains(&child), "subtree child not inserted");
        assert_eq!(
            Some(&sub_root),
            tree.parent(&child).expect("`Node` parent not found"),
            "parent edge not found"
        );
    }
}

#[test]
fn remove_should_work() {
    // setup
    let mut tree = create_tree();
    let c1 = tree
        .children(tree.root())
        .expect("could not get root `Node` children")
        .get_index(0)
        .expect("`Node` has no children")
        .clone();

    let c1_children = tree
        .children(&c1)
        .expect("could not get `Node` children")
        .clone();

    // test
    let sub_tree = tree.remove(&c1).expect("could not remove tree");
    assert_eq!(&c1, sub_tree.root(), "incorrect tree removed");
    assert!(
        tree.parent(&c1).is_err(),
        "removed `Node` parent should error"
    );

    let sub_children = sub_tree
        .children(sub_tree.root())
        .expect("could not get subtree root `Node` children");

    assert_eq!(&c1_children, sub_children, "children do not match");

    for child in sub_children {
        assert_eq!(
            Some(&c1),
            sub_tree
                .parent(&child)
                .expect("could not get parent of `Node`"),
            "incorrect parent"
        );

        assert!(
            tree.parent(&child).is_err(),
            "removed `Node` should not have parent"
        );
    }

    let p_children = tree
        .children(&tree.root())
        .expect("could not get children of parent");

    assert!(
        !p_children.contains(&c1),
        "parent should not contain removed root"
    );
}

#[test]
fn mv_should_work() {
    // setup
    let root = Data::new();
    let mut tree = ResourceTree::new(root);

    let c1 = Data::new();
    let c2 = Data::new();
    let c11 = Data::new();
    let c111 = Data::new();

    let c1_id = c1.id().clone();
    let c2_id = c2.id().clone();
    let c11_id = c11.id().clone();
    let c111_id = c111.id().clone();

    tree.insert(tree.root().clone(), c1)
        .expect("could not insert root child `Node`");

    tree.insert(tree.root().clone(), c2)
        .expect("could not insert root child `Node`");

    tree.insert(c1_id.clone(), c11)
        .expect("could not insert `Node`");

    tree.insert(c11_id.clone(), c111)
        .expect("could not insert `Node`");

    // test
    tree.mv(&c11_id, &c2_id).expect("could not move tree");

    assert!(
        tree.children(&c2_id)
            .expect("could not get `Node` children")
            .contains(&c11_id),
        "`Node` not moved"
    );

    assert_eq!(
        Some(&c2_id),
        tree.parent(&c11_id).expect("`Node` parent not found"),
        "`Node` parent not set"
    );

    assert!(tree
        .children(&c11_id)
        .expect("could not get `Node` children")
        .contains(&c111_id));

    assert_eq!(
        Some(&c11_id),
        tree.parent(&c111_id).expect("`Node` parent not found"),
        "`Node` parent not set"
    );
}

#[test]
fn move_index_should_work() {
    // setup
    let mut tree = create_tree();
    let children = tree
        .children(tree.root())
        .expect("could not get root `Node` children");

    let c1 = children
        .get_index(0)
        .expect("could not get child `Node` by index")
        .clone();

    // test
    tree.move_index(&c1, 1)
        .expect("could not move child `Node` index");

    let children = tree
        .children(tree.root())
        .expect("could not get root `Node` children");

    assert_eq!(
        &c1,
        children.get_index(1).expect("child `Node` not found"),
        "`Node` in incorrect position"
    );
}

// *****************
// *** Mock Data ***
// *****************

#[derive(HasId, Clone)]
struct Data {
    #[id]
    id: ResourceId,
    inner: i32,
}

impl Data {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();

        Self {
            id: ResourceId::new(),
            inner: rng.gen(),
        }
    }

    pub fn inner(&self) -> &i32 {
        &self.inner
    }
}

// ***************
// *** helpers ***
// ***************

fn create_tree() -> ResourceTree<Data> {
    let root = Data::new();
    let mut tree = ResourceTree::new(root);

    let c1 = Data::new();
    let c2 = Data::new();
    let c11 = Data::new();
    let c1_id = c1.id().clone();

    tree.insert(tree.root().clone(), c1)
        .expect("could not insert root child `Node`");

    tree.insert(tree.root().clone(), c2)
        .expect("could not insert root child `Node`");

    tree.insert(c1_id.clone(), c11)
        .expect("could not insert root child `Node`");

    tree
}
