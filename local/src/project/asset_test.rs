use super::*;
use crate::project::container;
use fake::{
    faker::filesystem::raw::{FileName, FilePath},
    locales::EN,
    Fake,
};

// ********************
// *** AssetBuilder ***
// ********************

#[test]
fn asset_builder_container_path_should_work() {
    // setup
    let _dir = tempfile::tempdir().unwrap();
    let builder = container::builder::InitOptions::init();
    builder
        .build(_dir.path())
        .expect("could not init dir as `Container`");

    let mut path = _dir.path().to_path_buf();
    path.push(FileName(EN).fake::<String>());

    let mut asset = AssetBuilder::new(path.clone());

    // test
    // container unset
    let calc_path = asset
        .container_path()
        .expect("could not calculate container path with `container` unset");

    assert_eq!(
        _dir.path(),
        calc_path.as_path(),
        "incorrect container path with `container` unset"
    );

    // container set
    asset.set_container(_dir.path().to_path_buf());
    let calc_path = asset
        .container_path()
        .expect("could not calculate container path with `container` set");

    assert_eq!(
        _dir.path(),
        calc_path.as_path(),
        "incorrect container path with `container` set"
    );
}

#[test]
#[should_panic(expected = "PathNotAContainer")]
fn asset_builder_container_path_with_invalid_container_set_should_error() {
    let path = PathBuf::from(FilePath(EN).fake::<String>());
    let mut asset = AssetBuilder::new(path.clone());
    asset.set_container(
        path.parent()
            .expect("could not get path parent")
            .to_path_buf(),
    );

    asset.container_path().unwrap();
}

#[test]
fn asset_builder_tentative_final_path_with_file_in_correct_location_should_work() {
    // setup
    let _dir = tempfile::tempdir().unwrap();
    let builder = container::builder::InitOptions::init();
    builder
        .build(_dir.path())
        .expect("could not init dir as `Container`");

    // test
    // container unset, bucket unset
    // action should be irrelevant
    let mut path = _dir.path().to_path_buf();
    let file_name = PathBuf::from(FileName(EN).fake::<String>());
    path.push(file_name.clone());

    let asset = AssetBuilder::new(path.clone());

    let final_path = asset
        .tentative_final_path(FsResourceAction::Copy)
        .expect("could not calculate final path with container unset");

    assert_eq!(
        &final_path, &path,
        "incorrect final path with container unset"
    );
}

#[test]
fn asset_builder_tentative_final_path_with_bucket_unset_should_work() {
    // setup
    let _dir = tempfile::tempdir().unwrap();
    let builder = container::builder::InitOptions::init();
    builder
        .build(_dir.path())
        .expect("could not init dir as `Container`");

    let path = PathBuf::from(FilePath(EN).fake::<String>());
    let file_name = path.file_name().expect("invalid file name");
    let file_name = PathBuf::from(file_name);

    let mut asset = AssetBuilder::new(path.clone());
    asset.set_container(_dir.path().to_path_buf());

    // test
    // container unset, bucket unset, reference
    let final_path = asset
        .tentative_final_path(FsResourceAction::Reference)
        .expect(
            "could not calculate final path with container unset for `AssetFileAction::Reference`",
        );

    assert_eq!(
        &path, &final_path,
        "incorrect final path with container unser for `AssetFileAction::Reference`"
    );

    // bucket unset, copy
    let mut expected = _dir.path().to_path_buf();
    expected.push(file_name.clone());

    let final_path = asset
        .tentative_final_path(FsResourceAction::Copy)
        .expect("could not calculate final path with container unset for `AssetFileAction::Copy`");

    assert_eq!(
        expected, final_path,
        "incorrect final path with container unser for `AssetFileAction::Copy`"
    );

    // bucket unset, move
    let mut expected = _dir.path().to_path_buf();
    expected.push(file_name.clone());

    let final_path = asset
        .tentative_final_path(FsResourceAction::Move)
        .expect("could not calculate final path with container unset for `AssetFileAction::Move`");

    assert_eq!(
        expected, final_path,
        "incorrect final path with container unser for `AssetFileAction::Move`"
    );
}

#[test]
fn asset_builder_tentative_final_path_with_bucket_set_should_work() {
    // setup
    let _dir = tempfile::tempdir().unwrap();
    let builder = container::builder::InitOptions::init();
    builder
        .build(_dir.path())
        .expect("could not init dir as `Container`");

    let path = PathBuf::from(FilePath(EN).fake::<String>());
    let file_name = path.file_name().expect("invalid file name");
    let file_name = PathBuf::from(file_name);

    let bucket = PathBuf::from("a/b");

    let mut asset = AssetBuilder::new(path.clone());
    asset.set_container(_dir.path().to_path_buf());
    asset.set_bucket(bucket.clone());

    // test
    // container unset, bucket unset, reference
    let final_path = asset
        .tentative_final_path(FsResourceAction::Reference)
        .expect(
            "could not calculate final path with container unset for `AssetFileAction::Reference`",
        );

    assert_eq!(
        &path, &final_path,
        "incorrect final path with container unser for `AssetFileAction::Reference`"
    );

    // bucket unset, copy
    let mut expected = _dir.path().to_path_buf();
    expected.push(bucket.clone());
    expected.push(file_name.clone());

    let final_path = asset
        .tentative_final_path(FsResourceAction::Copy)
        .expect("could not calculate final path with container unset for `AssetFileAction::Copy`");

    assert_eq!(
        expected, final_path,
        "incorrect final path with container unser for `AssetFileAction::Copy`"
    );

    // bucket unset, move
    let mut expected = _dir.path().to_path_buf();
    expected.push(bucket.clone());
    expected.push(file_name.clone());

    let final_path = asset
        .tentative_final_path(FsResourceAction::Move)
        .expect("could not calculate final path with container unset for `AssetFileAction::Move`");

    assert_eq!(
        expected, final_path,
        "incorrect final path with container unser for `AssetFileAction::Move`"
    );
}

#[test]
#[should_panic(expected = "InvalidPath")]
fn asset_builder_tentative_final_path_with_invalid_path_should_error() {
    todo!();
}

#[test]
fn asset_builder_init_should_work() {
    todo!();
}

#[test]
fn asset_builder_create_move_should_work() {
    todo!();
}

#[test]
fn asset_builder_create_copy_should_work() {
    todo!();
}

#[test]
fn asset_builder_create_reference_should_work() {
    todo!();
}

// *****************
// *** functions ***
// *****************

#[test]
fn container_from_path_ancestor_should_work() {
    // setup
    let mut _dir = tempfile::tempdir().unwrap();
    let builder = container::builder::InitOptions::init();
    let _cid = builder
        .build(_dir.path())
        .expect("init container should work");

    let c_dir = tempfile::tempdir_in(_dir.path()).unwrap();
    let a_path = c_dir.path().join(FileName(EN).fake::<String>());

    // test
    let path =
        container_from_path_ancestor(&a_path).expect("container from ancestor path should work");
    assert_eq!(_dir.path(), &path, "container path should be correct");
}

#[test]
#[should_panic(expected = "ContainerNotFound")]
fn container_from_path_ancestor_should_error_if_no_container_found() {
    // setup
    let mut _dir = tempfile::tempdir().unwrap();
    let c_dir = tempfile::tempdir_in(_dir.path()).unwrap();
    let a_path = c_dir.path().join(FileName(EN).fake::<String>());

    // test
    let _path = container_from_path_ancestor(&a_path).unwrap();
}
