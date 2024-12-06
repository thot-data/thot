use std::path::{Component, Path, PathBuf};
use syre_fs_watcher_simulator as simulator;

#[test_log::test]
fn test_simulator() {
    let base_dir = tempfile::tempdir().unwrap();
    let app_config_dir = tempfile::tempdir_in(base_dir.path()).unwrap();

    let user_manifest =
        tempfile::NamedTempFile::with_prefix_in("user_manifest-", app_config_dir.path()).unwrap();

    let project_manifest =
        tempfile::NamedTempFile::with_prefix_in("project_manifest-", app_config_dir.path())
            .unwrap();

    let mut options = simulator::options::Builder::new(base_dir.path().to_path_buf());
    options.set_max_ticks(10000);
    options.set_user_manifest(prepend_root(
        user_manifest.path().strip_prefix(base_dir.path()).unwrap(),
    ));

    options.set_project_manifest(prepend_root(
        project_manifest
            .path()
            .strip_prefix(base_dir.path())
            .unwrap(),
    ));

    let mut sim = simulator::Simulator::new(options.build());
    sim.run();
}

pub fn prepend_root(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(Component::RootDir.as_os_str()).join(path)
}
