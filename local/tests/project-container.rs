use std::fs;
use std::path::PathBuf;
use syre_local::loader::tree::Loader as TreeLoader;
use syre_local::project::container;

#[test]
fn initialize_existing_folder_as_project() {
    let root_dir = uninitialized_folder_tree_small();

    let mut builder = container::InitOptions::init();
    builder.recurse(true);
    builder.with_assets();
    builder.build(root_dir.path()).unwrap();

    let graph = TreeLoader::load(root_dir.path()).unwrap();
    assert_eq!(graph.nodes().len(), 7);
    for (_, container) in graph.iter_nodes() {
        if graph.children(container.rid()).unwrap().len() == 0 {
            assert_eq!(container.assets.len(), 2);
        } else {
            assert_eq!(container.assets.len(), 0);
        }
    }
}

/// Creates a small folder tree.
fn uninitialized_folder_tree_small() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();

    let folder_paths =
        ["01/01_01/", "01/01_02/", "02/02_01/", "02/02_02/"].map(|path| PathBuf::from(path));
    let files = ["a01", "a02"];

    for folder in folder_paths {
        let folder = root.path().join(&folder);
        fs::create_dir_all(&folder).unwrap();
        for file in files {
            fs::write(folder.join(file), "").unwrap();
        }
    }

    return root;
}
