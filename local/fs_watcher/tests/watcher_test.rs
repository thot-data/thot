#![feature(assert_matches)]
use std::{
    assert_matches::assert_matches,
    fs,
    ops::{Deref, DerefMut},
    path::PathBuf,
    thread,
    time::Duration,
};
use syre_fs_watcher::{event, EventKind};
use syre_local::common;

const TIMEOUT: Duration = Duration::from_millis(1000);

// NB: Test may be flaky due to timing of debounce.
#[test_log::test]
fn test_watcher_app() {
    let dir = tempfile::tempdir().unwrap();
    let config_dir = tempfile::tempdir_in(dir.path()).unwrap();
    let user_manifest = tempfile::NamedTempFile::new_in(config_dir.path()).unwrap();
    let project_manifest = tempfile::NamedTempFile::new_in(config_dir.path()).unwrap();
    let local_config = tempfile::NamedTempFile::new_in(config_dir.path()).unwrap();

    fs::write(user_manifest.path(), "{}").unwrap();
    fs::write(project_manifest.path(), "[]").unwrap();
    fs::write(local_config.path(), "{}").unwrap();

    let (fs_event_tx, fs_event_rx) = crossbeam::channel::unbounded();
    let (fs_command_tx, fs_command_rx) = crossbeam::channel::unbounded();

    let fs_watcher_config = syre_fs_watcher::server::Config::new(
        user_manifest.path(),
        project_manifest.path(),
        local_config.path(),
    );

    let mut fs_watcher =
        syre_fs_watcher::server::Builder::new(fs_command_rx, fs_event_tx, fs_watcher_config);

    fs_watcher.add_path(config_dir.path());
    thread::spawn(move || fs_watcher.run());
    thread::sleep(Duration::from_millis(500)); // let thread start

    let mut user_manifest = Manifest::new(user_manifest.path());
    let mut project_manifest = Manifest::new(project_manifest.path());
    let mut local_config = Manifest::new(local_config.path());

    fs::remove_file(&user_manifest.path).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap();
    let event = event.unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Config(event::Config::UserManifest(
            event::StaticResourceEvent::Removed
        ))
    );

    user_manifest.save();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap();
    let event = event.unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Config(event::Config::UserManifest(
            event::StaticResourceEvent::Created
        ))
    );

    user_manifest.push("user");
    user_manifest.save();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap();
    let event = event.unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Config(event::Config::UserManifest(
            event::StaticResourceEvent::Modified(event::ModifiedKind::Data)
        ))
    );

    fs::remove_file(&project_manifest.path).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap();
    let event = event.unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Config(event::Config::ProjectManifest(
            event::StaticResourceEvent::Removed
        ))
    );

    project_manifest.save();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap();
    let event = event.unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Config(event::Config::ProjectManifest(
            event::StaticResourceEvent::Created
        ))
    );

    project_manifest.push("project");
    project_manifest.save();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap();
    let event = event.unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Config(event::Config::ProjectManifest(
            event::StaticResourceEvent::Modified(event::ModifiedKind::Data)
        ))
    );

    fs::remove_file(&local_config.path).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap();
    let event = event.unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Config(event::Config::LocalConfig(
            event::StaticResourceEvent::Removed
        ))
    );

    local_config.save();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap();
    let event = event.unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Config(event::Config::LocalConfig(
            event::StaticResourceEvent::Created
        ))
    );

    local_config.push("settings");
    local_config.save();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap();
    let event = event.unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Config(event::Config::LocalConfig(
            event::StaticResourceEvent::Modified(event::ModifiedKind::Data)
        ))
    );
}

#[test_log::test]
fn test_watcher_project() {
    let dir = tempfile::tempdir().unwrap();
    let config_dir = tempfile::tempdir_in(dir.path()).unwrap();
    let user_manifest = tempfile::NamedTempFile::new_in(config_dir.path()).unwrap();
    let project_manifest = tempfile::NamedTempFile::new_in(config_dir.path()).unwrap();
    let local_config = tempfile::NamedTempFile::new_in(config_dir.path()).unwrap();

    fs::write(project_manifest.path(), "[]").unwrap();

    let (fs_event_tx, fs_event_rx) = crossbeam::channel::unbounded();
    let (fs_command_tx, fs_command_rx) = crossbeam::channel::unbounded();

    let fs_watcher_config = syre_fs_watcher::server::Config::new(
        user_manifest.path(),
        project_manifest.path(),
        local_config.path(),
    );

    let mut fs_watcher =
        syre_fs_watcher::server::Builder::new(fs_command_rx, fs_event_tx, fs_watcher_config);

    fs_watcher.add_path(config_dir.path());
    thread::spawn(move || fs_watcher.run());
    thread::sleep(Duration::from_millis(500)); // let thread start

    let mut project_manifest = Manifest::new(project_manifest.path());

    let prj = tempfile::tempdir_in(dir.path()).unwrap();
    fs_command_tx
        .send(syre_fs_watcher::Command::Watch(prj.path().to_path_buf()))
        .unwrap();

    project_manifest.push(prj.path().to_path_buf());
    project_manifest.save();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Config(event::Config::ProjectManifest(
            event::StaticResourceEvent::Modified(event::ModifiedKind::Data)
        ))
    );

    fs::create_dir(common::app_dir_of(prj.path())).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Project(event::Project::ConfigDir(
            event::StaticResourceEvent::Created
        ))
    );

    fs::File::create(common::project_file_of(prj.path())).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Project(event::Project::Properties(
            event::StaticResourceEvent::Created
        ))
    );

    fs::remove_file(common::project_file_of(prj.path())).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Project(event::Project::Properties(
            event::StaticResourceEvent::Removed
        ))
    );

    fs::File::create(common::project_settings_file_of(prj.path())).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Project(event::Project::Settings(
            event::StaticResourceEvent::Created
        ))
    );

    fs::remove_file(common::project_settings_file_of(prj.path())).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Project(event::Project::Settings(
            event::StaticResourceEvent::Removed
        ))
    );

    fs::File::create(common::analyses_file_of(prj.path())).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Project(event::Project::Analyses(
            event::StaticResourceEvent::Created
        ))
    );

    fs::remove_file(common::analyses_file_of(prj.path())).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Project(event::Project::Analyses(
            event::StaticResourceEvent::Removed
        ))
    );

    let mut project = syre_local::project::resources::Project::new(prj.path()).unwrap();
    project.save().unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(
        event.len(),
        2,
        "project properties and settings should be created"
    );

    project.set_analysis_root("analysis");
    project.save().unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 2);
    assert!(event.iter().any(|event| matches!(
        event.kind(),
        EventKind::Project(event::Project::Properties(
            event::StaticResourceEvent::Modified(event::ModifiedKind::Data)
        ))
    )));

    fs::create_dir(project.analysis_root_path().unwrap()).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Project(event::Project::AnalysisDir(event::ResourceEvent::Created))
    );

    let path = project.analysis_root_path().unwrap().join("tmp");
    fs::create_dir(&path).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Folder(event::ResourceEvent::Created)
    );

    fs::remove_dir(path).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Folder(event::ResourceEvent::Removed)
    );

    let path = project.analysis_root_path().unwrap().join("tmp.txt");
    fs::File::create(&path).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::File(event::ResourceEvent::Created)
    );

    fs::remove_file(&path).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::File(event::ResourceEvent::Removed)
    );

    let mut path = project.analysis_root_path().unwrap().join("analysis");
    for ext in syre_core::project::ScriptLang::supported_extensions() {
        path.set_extension(ext);
        fs::File::create(&path).unwrap();
        let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
        assert_eq!(event.len(), 1);
        assert_matches!(
            event[0].kind(),
            EventKind::AnalysisFile(event::ResourceEvent::Created)
        );

        fs::remove_file(&path).unwrap();
        let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
        assert_eq!(event.len(), 1);
        assert_matches!(
            event[0].kind(),
            EventKind::AnalysisFile(event::ResourceEvent::Removed)
        );
    }

    fs::remove_dir(project.analysis_root_path().unwrap()).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Project(event::Project::AnalysisDir(event::ResourceEvent::Removed))
    );

    fs::create_dir(project.data_root_path()).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Project(event::Project::DataDir(event::ResourceEvent::Created))
    );

    fs::create_dir(common::app_dir_of(project.data_root_path())).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Container(event::Container::ConfigDir(
            event::StaticResourceEvent::Created
        ))
    );

    fs::File::create(common::container_file_of(project.data_root_path())).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Container(event::Container::Properties(
            event::StaticResourceEvent::Created
        ))
    );

    fs::remove_file(common::container_file_of(project.data_root_path())).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Container(event::Container::Properties(
            event::StaticResourceEvent::Removed
        ))
    );

    fs::File::create(common::container_settings_file_of(project.data_root_path())).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Container(event::Container::Settings(
            event::StaticResourceEvent::Created
        ))
    );

    fs::remove_file(common::container_settings_file_of(project.data_root_path())).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Container(event::Container::Settings(
            event::StaticResourceEvent::Removed
        ))
    );

    fs::File::create(common::assets_file_of(project.data_root_path())).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Container(event::Container::Assets(
            event::StaticResourceEvent::Created
        ))
    );

    fs::remove_file(common::assets_file_of(project.data_root_path())).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Container(event::Container::Assets(
            event::StaticResourceEvent::Removed
        ))
    );

    let mut container = syre_local::project::resources::Container::new(project.data_root_path());
    container.save().unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 3);
    assert!(event.iter().any(|event| matches!(
        event.kind(),
        EventKind::Container(event::Container::Properties(
            event::StaticResourceEvent::Created
        ))
    )));
    assert!(event.iter().any(|event| matches!(
        event.kind(),
        EventKind::Container(event::Container::Settings(
            event::StaticResourceEvent::Created
        ))
    )));
    assert!(event.iter().any(|event| matches!(
        event.kind(),
        EventKind::Container(event::Container::Assets(
            event::StaticResourceEvent::Created
        ))
    )));

    container.properties.kind = Some("test".to_string());
    container.save().unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 3);
    assert!(event.iter().any(|event| matches!(
        event.kind(),
        EventKind::Container(event::Container::Properties(
            event::StaticResourceEvent::Modified(event::ModifiedKind::Data)
        ))
    )));
    assert!(event.iter().any(|event| matches!(
        event.kind(),
        EventKind::Container(event::Container::Settings(
            event::StaticResourceEvent::Modified(event::ModifiedKind::Data)
        ))
    )));
    assert!(event.iter().any(|event| matches!(
        event.kind(),
        EventKind::Container(event::Container::Assets(
            event::StaticResourceEvent::Modified(event::ModifiedKind::Data)
        ))
    )));

    let path = container.base_path().join("asset.csv");
    fs::File::create(&path).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::AssetFile(event::ResourceEvent::Created)
    );

    let renamed_path = container.base_path().join("asset-renamed.csv");
    fs::rename(path, &renamed_path).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::AssetFile(event::ResourceEvent::Renamed)
    );

    fs::remove_file(&renamed_path).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::AssetFile(event::ResourceEvent::Removed)
    );

    let path = container.base_path().join("child");
    fs::create_dir(&path).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Folder(event::ResourceEvent::Created)
    );

    fs::remove_dir(&path).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Folder(event::ResourceEvent::Removed)
    );

    fs::create_dir(&path).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Folder(event::ResourceEvent::Created)
    );

    fs::create_dir(common::app_dir_of(&path)).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Container(event::Container::ConfigDir(
            event::StaticResourceEvent::Created
        ))
    );

    fs::File::create(common::assets_file_of(&path)).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Container(event::Container::Assets(
            event::StaticResourceEvent::Created
        ))
    );

    fs::File::create(common::container_settings_file_of(&path)).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Container(event::Container::Settings(
            event::StaticResourceEvent::Created
        ))
    );

    fs::File::create(common::container_file_of(&path)).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Container(event::Container::Properties(
            event::StaticResourceEvent::Created
        ))
    );

    fs::remove_dir_all(&path).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Folder(event::ResourceEvent::Removed)
    );

    let mut container = syre_local::project::resources::Container::new(path);
    container.save().unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(event[0].kind(), EventKind::Graph(event::Graph::Created));

    let mut to = container.base_path().to_path_buf();
    to.set_file_name("child-1");
    fs::rename(container.base_path(), &to).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Container(event::Container::Renamed)
    );
    container.set_base_path(to);

    let path = project.data_root_path().join("child-2");
    let mut container_sibling = syre_local::project::resources::Container::new(&path);
    container_sibling.save().unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(event[0].kind(), EventKind::Graph(event::Graph::Created));

    let to = container.base_path().join(path.file_name().unwrap());
    fs::rename(container_sibling.base_path(), &to).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(event[0].kind(), EventKind::Graph(event::Graph::Moved));
    container_sibling.set_base_path(to);

    fs::remove_dir_all(container_sibling.base_path()).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);

    #[cfg(target_os = "windows")]
    todo!("may not be able to determine folder type to due destruction");

    #[cfg(target_os = "macos")]
    todo!("may not be able to determine folder type to due destruction");

    // On linux, only the top level directory is reproted as being removed
    // so can not determine folder type.
    // TODO: Could potentially us file id to check if folder is still accessible,
    // and if so determine file type.
    #[cfg(target_os = "linux")]
    assert_matches!(
        event[0].kind(),
        EventKind::Folder(event::ResourceEvent::Removed)
    );

    fs::remove_dir_all(project.data_root_path()).unwrap();
    let event = fs_event_rx.recv_timeout(TIMEOUT).unwrap().unwrap();
    assert_eq!(event.len(), 1);
    assert_matches!(
        event[0].kind(),
        EventKind::Project(event::Project::DataDir(event::ResourceEvent::Removed))
    );
}

struct Manifest<T>
where
    T: serde::Serialize,
{
    pub manifest: Vec<T>,
    pub path: PathBuf,
}

impl<T> Manifest<T>
where
    T: serde::Serialize,
{
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            manifest: vec![],
            path: path.into(),
        }
    }

    pub fn save(&self) {
        fs::write(&self.path, serde_json::to_string(&self.manifest).unwrap()).unwrap();
    }
}

impl<T> Deref for Manifest<T>
where
    T: serde::Serialize,
{
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.manifest
    }
}

impl<T> DerefMut for Manifest<T>
where
    T: serde::Serialize,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.manifest
    }
}
