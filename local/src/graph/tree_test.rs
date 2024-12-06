use super::*;
use crate::loader::tree::Loader as ContainerTreeLoader;
use crate::project::container;

#[test]
fn container_tree_transform_core_to_local_should_work() {
    let root_name = "root";
    let c1_name = "child 1";
    let c2_name = "child 2";
    let c11_name = "child 11";
    let c21_name = "child 21";

    let root = CoreContainer::new(root_name);
    let c1 = CoreContainer::new(c1_name);
    let c2 = CoreContainer::new(c2_name);
    let c11 = CoreContainer::new(c11_name);
    let c21 = CoreContainer::new(c21_name);

    let root_id = root.rid().clone();
    let c1_id = c1.rid().clone();
    let c2_id = c2.rid().clone();
    let c11_id = c11.rid().clone();
    let c21_id = c21.rid().clone();

    let mut core_tree = ResourceTree::new(root);
    core_tree.insert(root_id.clone(), c1).unwrap();
    core_tree.insert(root_id.clone(), c2).unwrap();
    core_tree.insert(c1_id.clone(), c11).unwrap();
    core_tree.insert(c2_id.clone(), c21).unwrap();

    let base_path = PathBuf::from("/base/path/");
    let local_tree = ContainerTreeTransformer::core_to_local(core_tree, base_path.clone());

    assert_eq!(*local_tree.root(), root_id);
    assert_eq!(
        local_tree.get(&root_id).unwrap().base_path(),
        base_path.join(root_name)
    );
    assert_eq!(
        local_tree.get(&c1_id).unwrap().base_path(),
        local_tree.get(&root_id).unwrap().base_path().join(c1_name),
    );
    assert_eq!(
        local_tree.get(&c2_id).unwrap().base_path(),
        local_tree.get(&root_id).unwrap().base_path().join(c2_name),
    );
    assert_eq!(
        local_tree.get(&c11_id).unwrap().base_path(),
        local_tree.get(&c1_id).unwrap().base_path().join(c11_name),
    );
    assert_eq!(
        local_tree.get(&c21_id).unwrap().base_path(),
        local_tree.get(&c2_id).unwrap().base_path().join(c21_name),
    );
}

#[test]
fn container_tree_duplicate_without_assets_to_should_work() {
    // setup
    let dir = tempfile::tempdir().unwrap();
    let c1_dir = tempfile::tempdir_in(dir.path()).unwrap();
    let c2_dir = tempfile::tempdir_in(dir.path()).unwrap();
    let dup_dir = tempfile::tempdir().unwrap();
    let dup_child_dir = tempfile::tempdir().unwrap();

    let c11_dir = tempfile::tempdir_in(c1_dir.path()).unwrap();
    let c12_dir = tempfile::tempdir_in(c1_dir.path()).unwrap();

    let builder = container::builder::InitOptions::init();
    let _rid = builder
        .build(dir.path())
        .expect("could not init root `Container`");

    let cid_1 = builder
        .build(&c1_dir)
        .expect("could not init child `Container`");

    let _cid_2 = builder
        .build(&c2_dir)
        .expect("could not init child `Container`");

    let _cid_11 = builder
        .build(&c11_dir)
        .expect("could not init grandchild `Container`");

    let _cid_12 = builder
        .build(&c12_dir)
        .expect("could not init grandchild `Container`");

    let graph = ContainerTreeLoader::load(dir.path()).expect("could not load `Container` tree");

    // test
    // root
    let dup =
        ContainerTreeDuplicator::duplicate_without_assets_to(dup_dir.path(), &graph, graph.root())
            .expect("could not duplicate tree from root");

    let root_children = dup
        .children(dup.root())
        .expect("could not get root children");

    assert_eq!(
        graph.children(graph.root()).unwrap().len(),
        root_children.len(),
        "incorrect number of children"
    );

    assert_eq!(
        graph.get(graph.root()).unwrap().properties.name,
        dup.get(dup.root()).unwrap().properties.name
    );

    let mut c_names = child_names(graph.root(), &graph);
    let mut cdup_names = child_names(dup.root(), &dup);
    c_names.sort();
    cdup_names.sort();
    assert_eq!(c_names, cdup_names);

    // child
    let c_dup =
        ContainerTreeDuplicator::duplicate_without_assets_to(dup_child_dir.path(), &graph, &cid_1)
            .expect("could not duplicate tree from root");

    let c1_children = c_dup
        .children(c_dup.root())
        .expect("could not get root children");

    assert_eq!(
        graph.children(&cid_1).unwrap().len(),
        c1_children.len(),
        "incorrect number of children"
    );

    let mut c_names = child_names(&cid_1, &graph);
    let mut cdup_names = child_names(c_dup.root(), &c_dup);
    c_names.sort();
    cdup_names.sort();
    assert_eq!(c_names, cdup_names);
}

// ***************
// *** helpers ***
// ***************

fn child_names(parent: &ResourceId, graph: &ContainerTree) -> Vec<String> {
    graph
        .children(parent)
        .unwrap()
        .iter()
        .map(|cid| {
            let child = graph.get(cid).unwrap();
            child.properties.name.clone()
        })
        .collect()
}
