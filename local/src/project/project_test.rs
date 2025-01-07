use super::*;
use crate::common::{app_dir_of, project_file_of, project_settings_file_of};
use syre_core::project::Project as CoreProject;

// ******************
// *** new + init ***
// ******************

#[test]
fn new_should_work() {
    let _dir = tempfile::tempdir().unwrap();
    let root = _dir.path().join("root");
    let root = root.as_path();

    // initial check that root dir does not exist
    assert_eq!(false, root.is_dir(), "root dir should not exist before new");

    // initialize project
    new(root).expect("new should work");

    let app_dir = app_dir_of(root);
    assert_eq!(true, root.is_dir(), "root dir should exist after new");
    assert_eq!(true, app_dir.is_dir(), "app dir should exist after new");

    // ensure project files created
    let prj_files = vec![project_file_of(root), project_settings_file_of(root)];
    for f in prj_files {
        assert!(f.is_file(), "{:?} file should exist after new", f);
    }

    // ensure meta level is 0
    let prj_file = project_file_of(root);
    let prj_json = fs::read_to_string(prj_file.as_path()).expect("could not read project file");
    let prj: CoreProject = serde_json::from_str(&prj_json).expect("project should be valid json");
    assert_eq!(0, prj.meta_level, "project should have meta level 0");
}

#[test]
#[should_panic(expected = "IsADirectory")]
fn new_should_error_if_directory_already_exists() {
    let _dir = tempfile::tempdir().unwrap();
    new(_dir.path()).unwrap();
}

#[test]
fn init_should_work() {
    let _dir = tempfile::tempdir().unwrap();
    let root = _dir.path();

    // initialize project
    let init_res = init(root);
    assert_eq!(true, init_res.is_ok(), "new should return Ok");

    let app_dir = app_dir_of(root);
    assert_eq!(
        true,
        app_dir.is_dir(),
        "app directory should exist after init"
    );

    // ensure files created
    let prj_files = [project_file_of(root), project_settings_file_of(root)];
    for f in prj_files {
        assert!(f.is_file(), "{:?} file should exist after new", f);
    }

    // ensure project has meta level 0
    let prj_file = project_file_of(root);
    let prj_json = fs::read_to_string(prj_file).expect("could not read project file");
    let prj: CoreProject = serde_json::from_str(&prj_json).expect("project should be valid json");
    assert_eq!(0, prj.meta_level, "project has incorrect meta level");
}

#[test]
fn init_if_app_directory_exists_should_do_nothing() {
    // setup
    let _dir = tempfile::tempdir().unwrap();
    let root = _dir.path();
    let rid = init(root).expect("init should work");

    // test
    let found_rid = init(root).expect("init should work even if already initialized");
    assert_eq!(rid, found_rid, "resource ids should match");
}

#[test]
#[should_panic(expected = "NotFound")]
fn init_should_error_if_not_given_an_existing_directory() {
    let _dir = tempfile::tempdir().unwrap();
    let false_root = _dir.path().join("absent");

    init(false_root.as_path()).unwrap();
}

// *************************
// *** path is resource ***
// *************************

#[test]
fn path_is_resource_should_work() {
    let _dir = tempfile::tempdir().unwrap();
    let root = _dir.path();

    assert_eq!(
        false,
        path_is_resource(root),
        "path should not be a resource"
    );

    init(root).expect("init should work");
    assert_eq!(true, path_is_resource(root), "path should be a resource");
}

// *************************
// *** project root path ***
// *************************

#[test]
fn project_root_path_should_work_for_root() {
    // setup
    let _dir = tempfile::tempdir().unwrap();
    let root = _dir.path();
    let rid = init(root).unwrap();

    // test
    let found = project_root_path(root).unwrap();

    let projects = ProjectManifest::load().unwrap();
    assert!(projects.contains(&found));
}

#[test]
fn project_root_path_should_work_for_descendents() {
    // setup
    let mut _dir = tempfile::tempdir().unwrap();
    init(_dir.path()).expect("init should work");
    let cp1 = tempfile::tempdir_in(_dir.path()).unwrap();
    let cp2 = tempfile::tempdir_in(cp1.path()).unwrap();

    // test
    let f1 = project_root_path(&cp1).expect("project root path should work");
    assert_eq!(
        _dir.path(),
        f1.as_path(),
        "found project root from child should be correct"
    );

    let f2 = project_root_path(&cp2).expect("project root path should work");
    assert_eq!(
        _dir.path(),
        f2.as_path(),
        "found project root from grandchild should be correct"
    );
}

#[test]
fn project_root_path_if_exits_resource_path_should_work() {
    todo!();
}

#[test]
#[should_panic(expected = "PathNotInProject")]
fn project_root_path_if_path_is_not_in_a_project_should_error() {
    let _dir = tempfile::tempdir().unwrap();
    project_root_path(_dir.path()).unwrap();
}

#[test]
#[should_panic(expected = "PathNotInProject")]
fn project_root_path_if_no_root_is_found_should_error() {
    todo!();
}

// #[test]
// fn project_resource_root_path_for_root_should_work() {
//     // setup
//     let _dir = tempfile::tempdir().unwrap();
//     let root = _dir.path();
//     let rid = init(root).unwrap();

//     // test
//     let found = project_resource_root_path(root).unwrap();
//     let projects = ProjectManifest::load().unwrap();
//     assert!(projects.contains(&found));
// }

#[test]
fn project_resource_root_path_should_work_for_descendents() {
    todo!();
}

// #[test]
// #[should_panic(expected = "PathNotInProject")]
// fn project_resource_root_path_should_error_if_path_is_not_in_a_project() {
//     let root: PathBuf = DirPath(EN).fake();
//     project_resource_root_path(root.as_path()).unwrap();
// }

#[test]
#[should_panic(expected = "Misconfigured")]
fn project_resource_root_path_should_error_if_no_root_is_found() {
    todo!();
}
