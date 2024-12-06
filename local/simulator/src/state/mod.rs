use std::{
    cell::RefCell,
    ffi::OsString,
    ops::Deref,
    path::{Path, PathBuf},
    rc::{Rc, Weak},
};
use syre_local::{common, Reducible};

pub mod app;
pub mod fs;
pub mod graph;

use app::FolderResource;

pub struct Ptr<T>(Rc<RefCell<T>>);
impl<T> Ptr<T> {
    pub fn new(value: T) -> Self {
        Self(Rc::new(RefCell::new(value)))
    }

    pub fn downgrade(this: &Self) -> WPtr<T> {
        WPtr(Rc::downgrade(&this.0))
    }

    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        Rc::ptr_eq(&this.0, &other.0)
    }

    pub fn as_ptr(this: &Self) -> *const RefCell<T> {
        Rc::as_ptr(&this.0)
    }
}

impl<T> std::fmt::Debug for Ptr<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{:?} [{:?}]",
            Self::as_ptr(self),
            self.0.borrow()
        ))
    }
}

pub struct WPtr<T>(Weak<RefCell<T>>);
impl<T> WPtr<T> {
    pub fn upgrade(&self) -> Option<Ptr<T>> {
        self.0.upgrade().map(|value| Ptr(value))
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }

    pub fn as_ptr(&self) -> *const RefCell<T> {
        self.0.as_ptr()
    }
}

impl<T> std::fmt::Debug for WPtr<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self.0.as_ptr()))
    }
}

impl<T> Deref for Ptr<T> {
    type Target = Rc<RefCell<T>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Deref for WPtr<T> {
    type Target = Weak<RefCell<T>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Clone for WPtr<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub trait HasName {
    fn name(&self) -> &std::ffi::OsStr;
    fn set_name(&mut self, name: impl Into<std::ffi::OsString>);
}

#[derive(Debug)]
pub struct State {
    fs: fs::State,
    app: app::State,
}

impl State {
    pub fn new(
        path: impl Into<PathBuf>,
        user_manifest: impl Into<PathBuf>,
        project_manifest: impl Into<PathBuf>,
    ) -> Self {
        Self {
            fs: fs::State::new(path),
            app: app::State::new(user_manifest, project_manifest),
        }
    }

    pub fn fs(&self) -> &fs::State {
        &self.fs
    }

    pub fn app(&self) -> &app::State {
        &self.app
    }
}

impl State {
    fn insert_resource(&mut self, resource: app::AppResource) {
        match resource {
            app::AppResource::File(resource) => self.insert_file_resource(resource),
            app::AppResource::Folder(resource) => self.insert_folder_resource(resource),
        }
    }

    fn remove_resource(&mut self, resource: app::AppResource) {
        match resource {
            app::AppResource::File(resource) => self.remove_file_resource(resource),
            app::AppResource::Folder(resource) => self.remove_folder_resource(resource),
        }
    }

    fn insert_folder_resource(&mut self, resource: app::FolderResource) {
        match resource {
            FolderResource::Project(_) => {
                let project = self.app.find_resource_project(resource.into()).unwrap();
                let project = project.borrow().path().clone();
                self.app
                    .reduce(app::ProjectAction::Create(project.clone()).into());

                let folder = self.fs.graph().find_by_path(&project).unwrap();
                for child in self.fs.graph().children(&folder).unwrap() {
                    let path = project.join(child.borrow().name());
                    if let Some(app::AppResource::Folder(resource)) =
                        self.app.find_path_resource(path)
                    {
                        self.insert_folder_resource(resource);
                    }
                }

                for file in folder.borrow().files() {
                    let path = project.join(file.borrow().name());
                    if let Some(app::AppResource::File(resource)) =
                        self.app.find_path_resource(path)
                    {
                        self.insert_file_resource(resource)
                    }
                }
            }
            FolderResource::ProjectConfig(_) | FolderResource::ContainerConfig(_) => {
                unreachable!("handled elsewhere")
            }
            FolderResource::Analyses(_) => todo!(),
            FolderResource::Data(_) => {
                let project = self.app.find_resource_project(resource.into()).unwrap();
                let project = project.borrow().path().clone();
                self.app.reduce(
                    app::ProjectAction::Data {
                        project,
                        action: app::DataAction::InitializeGraph,
                    }
                    .into(),
                );
            }
            FolderResource::Container(_) => todo!(),
        }
    }

    fn insert_file_resource(&mut self, resource: app::FileResource) {
        match resource {
            app::FileResource::UserManifest(_) => todo!(),
            app::FileResource::ProjectManifest(_) => todo!(),
            app::FileResource::ProjectProperties(_) => todo!(),
            app::FileResource::ProjectSettings(_) => todo!(),
            app::FileResource::AnalysisManifest(_) => todo!(),
            app::FileResource::Analysis(_) => todo!(),
            app::FileResource::ContainerProperties(_) => todo!(),
            app::FileResource::ContainerSettings(_) => todo!(),
            app::FileResource::AssetManifest(_) => todo!(),
            app::FileResource::Asset(_) => todo!(),
        }
    }

    fn remove_folder_resource(&mut self, resource: app::FolderResource) {
        match resource {
            FolderResource::Project(resource) => {
                let project = resource.upgrade().unwrap();
                self.app
                    .reduce(app::ProjectAction::Remove(project.borrow().path().clone()).into());
            }
            FolderResource::ProjectConfig(resource) => {
                let project = self
                    .app
                    .find_resource_project(app::FolderResource::ProjectConfig(resource).into())
                    .unwrap();
                let project = project.borrow().path().clone();
                self.app.reduce(
                    app::ProjectAction::Config {
                        project,
                        action: app::ConfigAction::Remove,
                    }
                    .into(),
                );
            }
            FolderResource::Analyses(_) => todo!(),
            FolderResource::Data(_) => {
                unreachable!("Data references a project's data root if it does not exist");
            }
            FolderResource::Container(resource) => {
                let project = self
                    .app
                    .find_resource_project(app::FolderResource::Container(resource.clone()).into())
                    .unwrap()
                    .clone();

                let resource = resource.upgrade().unwrap();
                let project = project.borrow();
                let data = project.data().borrow();
                let graph = data.graph().unwrap();

                if Ptr::ptr_eq(&graph.root(), &resource) {
                    drop(data); // borrow needed for reduce
                    self.app.reduce(
                        app::ProjectAction::Data {
                            project: project.path().clone(),
                            action: app::DataAction::RemoveGraph,
                        }
                        .into(),
                    );
                } else {
                    let path = graph.path(&resource).unwrap();
                    drop(data); // borrow needed for reduce
                    self.app.reduce(
                        app::ProjectAction::Data {
                            project: project.path().clone(),
                            action: app::DataAction::RemoveContainer(path),
                        }
                        .into(),
                    );
                }
            }
            FolderResource::ContainerConfig(resource) => {
                let resource = resource.upgrade().unwrap();
                let container = self.app.find_container_config_container(&resource).unwrap();
                container.borrow_mut().remove_data();
            }
        }
    }

    fn remove_file_resource(&mut self, resource: app::FileResource) {
        match resource {
            app::FileResource::Asset(_) => todo!(),
            app::FileResource::Analysis(_) => todo!(),
            app::FileResource::UserManifest(_)
            | app::FileResource::ProjectManifest(_)
            | app::FileResource::ProjectProperties(_)
            | app::FileResource::ProjectSettings(_)
            | app::FileResource::AnalysisManifest(_)
            | app::FileResource::ContainerProperties(_)
            | app::FileResource::ContainerSettings(_)
            | app::FileResource::AssetManifest(_) => {}
        }
    }

    fn reduce_rename(&mut self, from: PathBuf, to: OsString) {
        // TODO: Be smarter with rename.
        let mut to_path = from.clone();
        to_path.set_file_name(to);
        let from_resource = self.app.find_path_resource(&from);
        let to_resource = self.app.find_path_resource(&to_path);
        match (from_resource, to_resource) {
            (None, None) => {}
            (Some(resource), None) => {
                if let app::AppResource::Folder(app::FolderResource::Project(_)) = resource {
                    self.app.reduce(
                        app::ProjectAction::SetPath {
                            project: from,
                            to: to_path,
                        }
                        .into(),
                    );
                } else {
                    self.remove_resource(resource.into())
                }
            }

            (None, Some(resource)) => self.insert_resource(resource),

            (Some(from_resource), Some(to_resource)) => todo!(),
        }
    }

    fn reduce_move(&mut self, from: PathBuf, to: PathBuf) {
        let from_resource = self.app.find_path_resource(&from);
        let to_resource = self.app.find_path_resource(&to);
        match (from_resource, to_resource) {
            (None, None) => {}
            (Some(resource), None) => {
                if let app::AppResource::Folder(app::FolderResource::Project(_)) = resource {
                    self.app
                        .reduce(app::ProjectAction::SetPath { project: from, to }.into());
                } else {
                    self.remove_resource(resource.into())
                }
            }

            (None, Some(resource)) => self.insert_resource(resource),

            (Some(from_resource), Some(to_resource)) => todo!(),
        }
    }
}

impl Reducible for State {
    type Action = fs::Action;
    fn reduce(&mut self, action: Self::Action) {
        self.fs.reduce(action.clone());
        match action {
            fs::Action::CreateFolder {
                path,
                with_parents: _,
            } => {
                if path.ends_with(common::app_dir()) {
                    let app::AppResource::Folder(parent) =
                        self.app.find_path_resource(path.parent().unwrap()).unwrap()
                    else {
                        unreachable!();
                    };

                    match parent {
                        FolderResource::Project(project) => {
                            let project = project.upgrade().unwrap();
                            let project = project.borrow().path().clone();
                            self.app.reduce(
                                app::ProjectAction::Config {
                                    project,
                                    action: app::ConfigAction::Insert,
                                }
                                .into(),
                            );

                            return;
                        }
                        FolderResource::Container(container) => {
                            let project = self
                                .app
                                .find_resource_project(
                                    app::FolderResource::Container(container.clone()).into(),
                                )
                                .unwrap()
                                .clone();

                            let container = container.upgrade().unwrap();
                            let project = project.borrow();
                            let data = project.data().borrow();
                            let graph = data.graph().unwrap();
                            let container = graph.path(&container).unwrap();
                            let project = project.path().clone();

                            self.app.reduce(
                                app::ProjectAction::Data {
                                    project,
                                    action: app::DataAction::ContainerConfig {
                                        container,
                                        action: app::ConfigAction::Insert,
                                    },
                                }
                                .into(),
                            );

                            return;
                        }
                        FolderResource::ProjectConfig(_)
                        | FolderResource::ContainerConfig(_)
                        | FolderResource::Analyses(_) => {}
                        FolderResource::Data(_) => unreachable!(),
                    }
                }

                if let Some(app_resource) = self.app.find_path_resource(path) {
                    match app_resource {
                        app::AppResource::File(app_resource) => {
                            self.insert_file_resource(app_resource);
                        }

                        app::AppResource::Folder(app_resource) => {
                            self.insert_folder_resource(app_resource);
                        }
                    }
                }
            }

            fs::Action::CreateFile { path, with_parents } => {}
            fs::Action::Remove(path) => {
                if let Some(app_resource) = self.app.find_path_resource(path) {
                    match app_resource {
                        app::AppResource::File(app_resource) => {
                            self.remove_file_resource(app_resource);
                        }

                        app::AppResource::Folder(app_resource) => {
                            self.remove_folder_resource(app_resource);
                        }
                    }
                }
            }
            fs::Action::Rename { from, to } => {
                self.reduce_rename(from, to);
            }
            fs::Action::Move { from, to } => {
                self.reduce_move(from, to);
            }
            fs::Action::Copy { from, to } => {}
            fs::Action::Modify { file, kind } => {
                if let Some(app_resource) = self.app.find_path_resource(&file) {
                    let app::AppResource::File(app_resource) = app_resource else {
                        unreachable!();
                    };

                    match app_resource {
                        app::FileResource::UserManifest(_) => {
                            match kind {
                                fs::ModifyKind::ManifestAdd(user) => self.app.reduce(
                                    app::AppAction::UserManifest(app::ManifestAction::AddItem(
                                        user,
                                    ))
                                    .into(),
                                ),
                                fs::ModifyKind::ManifestRemove(index) => self.app.reduce(
                                    app::AppAction::UserManifest(app::ManifestAction::RemoveItem(
                                        index,
                                    ))
                                    .into(),
                                ),
                                fs::ModifyKind::Corrupt | fs::ModifyKind::Repair => {
                                    // handled by fs state
                                }
                                fs::ModifyKind::Initialize => {
                                    let fs_resource = self.fs.find_file(&file).unwrap();
                                    fs_resource.borrow_mut().write("[]");
                                }
                                fs::ModifyKind::Other => {}
                            }

                            return;
                        }

                        app::FileResource::ProjectManifest(_) => {
                            match kind {
                                fs::ModifyKind::ManifestAdd(path) => {
                                    self.app.reduce(
                                        app::AppAction::ProjectManifest(
                                            app::ManifestAction::AddItem(path.clone()),
                                        )
                                        .into(),
                                    );

                                    if self.fs.exists(&path) {
                                        let project = self.app.find_path_project(path).unwrap();
                                        project.borrow_mut().sync_with_fs(&self.fs);
                                    }
                                }
                                fs::ModifyKind::ManifestRemove(index) => self.app.reduce(
                                    app::AppAction::ProjectManifest(
                                        app::ManifestAction::RemoveItem(index),
                                    )
                                    .into(),
                                ),
                                fs::ModifyKind::Corrupt | fs::ModifyKind::Repair => {}
                                fs::ModifyKind::Initialize => {
                                    let fs_resource = self.fs.find_file(&file).unwrap();
                                    fs_resource.borrow_mut().write("[]");
                                }
                                fs::ModifyKind::Other => {}
                            }

                            return;
                        }

                        app::FileResource::AnalysisManifest(manifest) => {
                            let project = self
                                .app
                                .find_resource_project(
                                    app::FileResource::AnalysisManifest(manifest).into(),
                                )
                                .unwrap();
                            let project = project.borrow().path().clone();

                            match kind {
                                fs::ModifyKind::ManifestAdd(path) => self.app.reduce(
                                    app::ProjectAction::Analyses {
                                        project,
                                        action: app::ManifestAction::AddItem(path.into()),
                                    }
                                    .into(),
                                ),
                                fs::ModifyKind::ManifestRemove(index) => self.app.reduce(
                                    app::ProjectAction::Analyses {
                                        project,
                                        action: app::ManifestAction::RemoveItem(index),
                                    }
                                    .into(),
                                ),
                                fs::ModifyKind::Corrupt | fs::ModifyKind::Repair => {}
                                fs::ModifyKind::Initialize => {
                                    let fs_resource = self.fs.find_file(&file).unwrap();
                                    fs_resource.borrow_mut().write("[]");
                                }
                                fs::ModifyKind::Other => {}
                            }
                        }

                        app::FileResource::AssetManifest(manifest) => {
                            let (project, container) = self
                                .app
                                .find_asset_manifest_project_and_container(
                                    &manifest.upgrade().unwrap(),
                                )
                                .unwrap();
                            let project = project.borrow();
                            let data = project.data().borrow();
                            let graph = data.graph().unwrap();
                            let container = graph.path(&container).unwrap();

                            match kind {
                                fs::ModifyKind::ManifestAdd(path) => self.app.reduce(
                                    app::ProjectAction::Data {
                                        project: project.path().clone(),
                                        action: app::DataAction::ContainerConfig {
                                            container,
                                            action: app::ManifestAction::AddItem(path).into(),
                                        },
                                    }
                                    .into(),
                                ),
                                fs::ModifyKind::ManifestRemove(index) => self.app.reduce(
                                    app::ProjectAction::Data {
                                        project: project.path().clone(),
                                        action: app::DataAction::ContainerConfig {
                                            container,
                                            action: app::ManifestAction::RemoveItem(index).into(),
                                        },
                                    }
                                    .into(),
                                ),
                                fs::ModifyKind::Corrupt | fs::ModifyKind::Repair => {}
                                fs::ModifyKind::Initialize => {
                                    let fs_resource = self.fs.find_file(&file).unwrap();
                                    fs_resource.borrow_mut().write("[]");
                                }
                                fs::ModifyKind::Other => {}
                            }
                        }

                        app::FileResource::ProjectProperties(_) => match kind {
                            fs::ModifyKind::Initialize => {
                                let name = file.parent().unwrap().parent().unwrap();
                                let project =
                                    syre_core::project::Project::new(name.to_string_lossy());
                                let fs_resource = self.fs.find_file(&file).unwrap();
                                fs_resource
                                    .borrow_mut()
                                    .write(serde_json::to_string(&project).unwrap());
                            }
                            fs::ModifyKind::Corrupt
                            | fs::ModifyKind::Repair
                            | fs::ModifyKind::Other => {}
                            fs::ModifyKind::ManifestAdd(_) | fs::ModifyKind::ManifestRemove(_) => {
                                unreachable!()
                            }
                        },

                        app::FileResource::ProjectSettings(_) => match kind {
                            fs::ModifyKind::Initialize => {
                                let settings = syre_local::types::ProjectSettings::new();
                                let fs_resource = self.fs.find_file(&file).unwrap();
                                fs_resource
                                    .borrow_mut()
                                    .write(serde_json::to_string(&settings).unwrap());
                            }
                            fs::ModifyKind::Corrupt
                            | fs::ModifyKind::Repair
                            | fs::ModifyKind::Other => {}
                            fs::ModifyKind::ManifestAdd(_) | fs::ModifyKind::ManifestRemove(_) => {
                                unreachable!()
                            }
                        },

                        app::FileResource::ContainerProperties(_) => match kind {
                            fs::ModifyKind::Initialize => {
                                let name = file.parent().unwrap().parent().unwrap();
                                let container =
                                    syre_core::project::Container::new(name.to_string_lossy());
                                let properties: syre_local::types::StoredContainerProperties =
                                    container.into();
                                let fs_resource = self.fs.find_file(&file).unwrap();
                                fs_resource
                                    .borrow_mut()
                                    .write(serde_json::to_string(&properties).unwrap());
                            }
                            fs::ModifyKind::Corrupt
                            | fs::ModifyKind::Repair
                            | fs::ModifyKind::Other => {}
                            fs::ModifyKind::ManifestAdd(_) | fs::ModifyKind::ManifestRemove(_) => {
                                unreachable!()
                            }
                        },

                        app::FileResource::ContainerSettings(_) => match kind {
                            fs::ModifyKind::Initialize => {
                                let settings = syre_local::types::ContainerSettings::default();
                                let fs_resource = self.fs.find_file(&file).unwrap();
                                fs_resource
                                    .borrow_mut()
                                    .write(serde_json::to_string(&settings).unwrap());
                            }
                            fs::ModifyKind::Corrupt
                            | fs::ModifyKind::Repair
                            | fs::ModifyKind::Other => {}
                            fs::ModifyKind::ManifestAdd(_) | fs::ModifyKind::ManifestRemove(_) => {
                                unreachable!()
                            }
                        },

                        app::FileResource::Analysis(_) | app::FileResource::Asset(_) => {
                            match kind {
                                fs::ModifyKind::Corrupt
                                | fs::ModifyKind::Repair
                                | fs::ModifyKind::Other => {}
                                fs::ModifyKind::ManifestAdd(_)
                                | fs::ModifyKind::ManifestRemove(_)
                                | fs::ModifyKind::Initialize => {
                                    unreachable!()
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Clone for State {
    fn clone(&self) -> Self {
        let (fs_state, _fs_node_map) = self.fs.duplicate_with_app_references_and_map();
        let (app_state, _app_resource_map) = self.app.duplicate_with_fs_references_and_map();

        Self {
            fs: fs_state,
            app: app_state,
        }
    }
}

/// Finds `origin` in the map, and returns its image.
fn find_mapped_to<'a, T>(origin: &Ptr<T>, map: &'a graph::NodeMap<T>) -> Option<&'a Ptr<T>> {
    map.iter().find_map(|(from, to)| {
        if Ptr::ptr_eq(from, origin) {
            Some(to)
        } else {
            None
        }
    })
}

pub type Result<T = ()> = std::result::Result<T, error::Error>;
pub mod error {
    #[derive(Debug)]
    pub enum Error {
        NameCollision,
        DoesNotExist,
    }
}
