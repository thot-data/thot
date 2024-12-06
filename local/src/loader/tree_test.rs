use super::*;
use crate::project::container;

#[test]
fn container_tree_load_should_work() {
    // setup
    let dir = tempfile::tempdir().unwrap();
    let c1_dir = tempfile::tempdir_in(dir.path()).unwrap();
    let c2_dir = tempfile::tempdir_in(dir.path()).unwrap();

    let builder = container::builder::InitOptions::init();
    let rid = builder
        .build(dir.path())
        .expect("could not init root `Container`");

    let cid_1 = builder
        .build(&c1_dir)
        .expect("could not init child `Container`");

    let cid_2 = builder
        .build(&c2_dir)
        .expect("could not init child `Container`");

    // test
    let graph = Loader::load(dir.path()).expect("could not load `Container` tree");

    assert_eq!(&rid, graph.root(), "incorrect root");
    assert!(
        graph
            .children(graph.root())
            .expect("root children not found")
            .contains(&cid_1),
        "child `Container` not a child"
    );

    assert!(
        graph
            .children(graph.root())
            .expect("root children not found")
            .contains(&cid_2),
        "child `Container` not a child"
    );

    assert!(graph.get(&rid).is_some(), "root `Container` not loaded");
    assert!(graph.get(&cid_1).is_some(), "child `Container` not loaded");
    assert!(graph.get(&cid_2).is_some(), "child `Container` not loaded");
}
