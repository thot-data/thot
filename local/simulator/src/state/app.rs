use super::{
    graph::{NodeMap, Tree},
    HasName, Ptr, Reducible, WPtr,
};
use has_id::HasId;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};
use syre_core::types::ResourceId;
use syre_local::common;

pub type ProjectMap = Vec<((Ptr<Project>, Ptr<Project>), ContainerMap)>;
pub type ContainerMap = NodeMap<Container>;

#[derive(Debug)]
pub struct State {
    app: AppState,
    projects: Vec<Ptr<Project>>,
}

impl State {
    pub fn new(user_manifest: impl Into<PathBuf>, project_manifest: impl Into<PathBuf>) -> Self {
        Self {
            app: AppState::new(user_manifest, project_manifest),
            projects: vec![],
        }
    }

    pub fn app_state(&self) -> &AppState {
        &self.app
    }

    pub fn projects(&self) -> &Vec<Ptr<Project>> {
        &self.projects
    }

    pub fn projects_mut(&mut self) -> &mut Vec<Ptr<Project>> {
        &mut self.projects
    }
}

impl State {
    pub fn find_path_resource(&self, path: impl AsRef<Path>) -> Option<AppResource> {
        let path = path.as_ref();
        let user_manifest = self.app.user_manifest();
        if path == user_manifest.borrow().path() {
            return Some(FileResource::UserManifest(Ptr::downgrade(user_manifest)).into());
        }

        let project_manifest = self.app.project_manifest();
        if path == project_manifest.borrow().path() {
            return Some(FileResource::ProjectManifest(Ptr::downgrade(project_manifest)).into());
        }

        self.projects.iter().find_map(|project_ptr| {
            let project = project_ptr.borrow();
            if project.path() == path {
                return Some(FolderResource::Project(Ptr::downgrade(project_ptr)).into());
            }

            if let Ok(rel_path) = path.strip_prefix(project.path()) {
                self.find_path_project_resource(rel_path, project_ptr)
            } else {
                None
            }
        })
    }

    /// # Arguments
    /// #. `path`: Path relative to project root.
    pub fn find_path_project_resource(
        &self,
        path: impl AsRef<Path>,
        project: &Ptr<Project>,
    ) -> Option<AppResource> {
        let path = path.as_ref();
        let project = project.borrow();
        assert!(path != project.path());
        if path == common::app_dir() {
            let Resource::Present(config) = project.config() else {
                panic!();
            };

            return Some(FolderResource::ProjectConfig(Ptr::downgrade(config)).into());
        }

        if path == common::project_file() {
            let Resource::Present(config) = project.config() else {
                panic!();
            };

            let config = config.borrow();
            return Some(
                FileResource::ProjectProperties(Ptr::downgrade(config.properties())).into(),
            );
        }

        if path == common::project_settings_file() {
            let Resource::Present(config) = project.config() else {
                panic!();
            };

            let config = config.borrow();
            return Some(FileResource::ProjectSettings(Ptr::downgrade(config.settings())).into());
        }

        if path == common::analyses_file() {
            let Resource::Present(config) = project.config() else {
                panic!();
            };

            let config = config.borrow();
            return Some(FileResource::AnalysisManifest(Ptr::downgrade(config.analyses())).into());
        }

        if let Some(analyses_ptr) = project.analyses() {
            let analyses = analyses_ptr.borrow();
            if path == analyses.path() {
                return Some(FolderResource::Analyses(Ptr::downgrade(analyses_ptr)).into());
            }

            if let Ok(rel_path) = path.strip_prefix(analyses.path()) {
                if let Resource::Present(config) = project.config() {
                    let config = config.borrow();
                    let analyses = config.analyses().borrow();
                    return analyses.manifest().iter().find_map(|analysis_ptr| {
                        let analysis = analysis_ptr.borrow();
                        if analysis.path() == rel_path {
                            Some(FileResource::Analysis(Ptr::downgrade(analysis_ptr)).into())
                        } else {
                            None
                        }
                    });
                }
            }
        }

        let data = project.data().borrow();
        if let Ok(rel_path) = path.strip_prefix(data.path()) {
            let Some(graph) = data.graph() else {
                return Some(FolderResource::Data(Ptr::downgrade(project.data())).into());
            };

            if rel_path.as_os_str() == "" {
                return Some(FolderResource::Container(Ptr::downgrade(&graph.root())).into());
            }

            if rel_path.file_name().unwrap() == common::app_dir() {
                let container_path = rel_path.parent().unwrap();
                let Some(container) = graph.find_by_path(container_path) else {
                    panic!();
                };

                let container = container.borrow();
                let data = container.data().as_ref().unwrap();
                return Some(FolderResource::ContainerConfig(Ptr::downgrade(data.config())).into());
            }

            if rel_path.ends_with(common::container_file()) {
                let container_path = rel_path.parent().unwrap().parent().unwrap();
                let Some(container) = graph.find_by_path(container_path) else {
                    panic!();
                };

                let container = container.borrow();
                let data = container.data().as_ref().unwrap();
                let config = data.config().borrow();
                return Some(
                    FileResource::ContainerProperties(Ptr::downgrade(config.properties())).into(),
                );
            }

            if rel_path.ends_with(common::container_settings_file()) {
                let container_path = rel_path.parent().unwrap().parent().unwrap();
                let Some(container) = graph.find_by_path(container_path) else {
                    panic!();
                };

                let container = container.borrow();
                let data = container.data().as_ref().unwrap();
                let config = data.config().borrow();
                return Some(
                    FileResource::ContainerSettings(Ptr::downgrade(config.settings())).into(),
                );
            }

            if rel_path.ends_with(common::assets_file()) {
                let container_path = rel_path.parent().unwrap().parent().unwrap();
                let Some(container) = graph.find_by_path(container_path) else {
                    panic!();
                };

                let container = container.borrow();
                let data = container.data().as_ref().unwrap();
                let config = data.config().borrow();
                return Some(FileResource::AssetManifest(Ptr::downgrade(config.assets())).into());
            }

            if let Some(container) = graph.find_by_path(rel_path) {
                return Some(FolderResource::Container(Ptr::downgrade(&container)).into());
            }

            let Some(container) = graph.find_by_path(rel_path.parent().unwrap()) else {
                return None;
            };

            let container = container.borrow();
            if let Some(data) = container.data().as_ref() {
                let config = data.config().borrow();
                let assets = config.assets().borrow();
                return assets.manifest().iter().find_map(|asset| {
                    if asset.borrow().name() == rel_path.file_name().unwrap() {
                        Some(FileResource::Asset(Ptr::downgrade(asset)).into())
                    } else {
                        None
                    }
                });
            }
        }

        None
    }
}

impl State {
    pub fn find_path_project(&self, path: impl AsRef<Path>) -> Option<&Ptr<Project>> {
        let path = path.as_ref();
        self.projects
            .iter()
            .find(|project| path.starts_with(project.borrow().path()))
    }

    pub fn find_resource_project(&self, resource: AppResource) -> Option<&Ptr<Project>> {
        match resource {
            AppResource::File(resource) => match resource {
                FileResource::UserManifest(_) => {
                    return None;
                }
                FileResource::ProjectManifest(_) => {
                    return None;
                }
                FileResource::ProjectProperties(resource) => {
                    let resource = resource.upgrade()?;
                    self.find_project_properties_project(&resource)
                }
                FileResource::ProjectSettings(resource) => {
                    let resource = resource.upgrade()?;
                    self.find_project_settings_project(&resource)
                }
                FileResource::AnalysisManifest(resource) => {
                    let resource = resource.upgrade()?;
                    self.find_analysis_manifest_project(&resource)
                }
                FileResource::Analysis(resource) => {
                    let resource = resource.upgrade()?;
                    self.find_analysis_project(&resource)
                }
                FileResource::ContainerProperties(resource) => {
                    let resource = resource.upgrade()?;
                    self.find_container_properties_project(&resource)
                }
                FileResource::ContainerSettings(resource) => {
                    let resource = resource.upgrade()?;
                    self.find_container_settings_project(&resource)
                }
                FileResource::AssetManifest(resource) => {
                    let resource = resource.upgrade()?;
                    self.find_asset_manifest_project(&resource)
                }
                FileResource::Asset(resource) => {
                    let resource = resource.upgrade()?;
                    self.find_asset_project(&resource)
                }
            },

            AppResource::Folder(resource) => match resource {
                FolderResource::Project(resource) => {
                    let resource = resource.upgrade()?;
                    self.projects()
                        .iter()
                        .find(|prj| Ptr::ptr_eq(prj, &resource))
                }
                FolderResource::ProjectConfig(resource) => {
                    let resource = resource.upgrade()?;
                    self.find_project_config_project(&resource)
                }
                FolderResource::Analyses(resource) => {
                    let resource = resource.upgrade()?;
                    self.find_analyses_project(&resource)
                }
                FolderResource::Data(resource) => {
                    let resource = resource.upgrade()?;
                    self.find_data_project(&resource)
                }
                FolderResource::Container(resource) => {
                    let resource = resource.upgrade()?;
                    self.find_container_project(&resource)
                }
                FolderResource::ContainerConfig(resource) => {
                    let resource = resource.upgrade()?;
                    self.find_container_config_project(&resource)
                }
            },
        }
    }

    pub fn find_project_properties_project(
        &self,
        resource: &Ptr<ProjectProperties>,
    ) -> Option<&Ptr<Project>> {
        self.projects
            .iter()
            .find(|project| match project.borrow().config() {
                Resource::NotPresent => false,
                Resource::Present(config) => Ptr::ptr_eq(config.borrow().properties(), &resource),
            })
    }

    pub fn find_project_settings_project(
        &self,
        resource: &Ptr<ProjectSettings>,
    ) -> Option<&Ptr<Project>> {
        self.projects
            .iter()
            .find(|project| match project.borrow().config() {
                Resource::NotPresent => false,
                Resource::Present(config) => Ptr::ptr_eq(config.borrow().settings(), resource),
            })
    }

    pub fn find_analysis_manifest_project(
        &self,
        resource: &Ptr<AnalysisManifest>,
    ) -> Option<&Ptr<Project>> {
        self.projects
            .iter()
            .find(|project| match project.borrow().config() {
                Resource::NotPresent => false,
                Resource::Present(config) => Ptr::ptr_eq(config.borrow().analyses(), resource),
            })
    }

    pub fn find_analysis_project(&self, resource: &Ptr<Analysis>) -> Option<&Ptr<Project>> {
        self.projects.iter().find(|project| {
            let project = project.borrow();
            let Resource::Present(config) = project.config() else {
                return false;
            };

            let config = config.borrow();
            let analyses = config.analyses().borrow();
            analyses
                .manifest()
                .iter()
                .any(|a| Ptr::ptr_eq(&resource, a))
        })
    }

    pub fn find_container_properties_project(
        &self,
        resource: &Ptr<ContainerProperties>,
    ) -> Option<&Ptr<Project>> {
        self.projects
            .iter()
            .find(|project| match project.borrow().data().borrow().graph() {
                None => false,
                Some(graph) => graph.nodes().iter().any(|node| match node.borrow().data() {
                    None => false,
                    Some(data) => Ptr::ptr_eq(data.config().borrow().properties(), &resource),
                }),
            })
    }

    pub fn find_container_properties_project_and_container(
        &self,
        resource: &Ptr<ContainerProperties>,
    ) -> Option<(Ptr<Project>, Ptr<Container>)> {
        self.projects.iter().find_map(|project_ptr| {
            let project = project_ptr.borrow();
            let data = project.data().borrow();
            let Some(graph) = data.graph() else {
                return None;
            };

            if let Some(container) = graph
                .nodes()
                .iter()
                .find(|node| match node.borrow().data() {
                    None => false,
                    Some(data) => Ptr::ptr_eq(data.config().borrow().properties(), &resource),
                })
            {
                Some((project_ptr.clone(), container.clone()))
            } else {
                None
            }
        })
    }

    pub fn find_container_settings_project(
        &self,
        resource: &Ptr<ContainerSettings>,
    ) -> Option<&Ptr<Project>> {
        self.projects
            .iter()
            .find(|project| match project.borrow().data().borrow().graph() {
                None => false,
                Some(graph) => graph.nodes().iter().any(|node| match node.borrow().data() {
                    None => false,
                    Some(data) => Ptr::ptr_eq(data.config().borrow().settings(), &resource),
                }),
            })
    }

    pub fn find_container_settings_project_and_container(
        &self,
        resource: &Ptr<ContainerSettings>,
    ) -> Option<(Ptr<Project>, Ptr<Container>)> {
        self.projects.iter().find_map(|project_ptr| {
            let project = project_ptr.borrow();
            let data = project.data().borrow();
            let Some(graph) = data.graph() else {
                return None;
            };

            if let Some(container) = graph
                .nodes()
                .iter()
                .find(|node| match node.borrow().data() {
                    None => false,
                    Some(data) => Ptr::ptr_eq(data.config().borrow().settings(), &resource),
                })
            {
                Some((project_ptr.clone(), container.clone()))
            } else {
                None
            }
        })
    }

    pub fn find_asset_manifest_project(
        &self,
        resource: &Ptr<AssetManifest>,
    ) -> Option<&Ptr<Project>> {
        self.projects
            .iter()
            .find(|project| match project.borrow().data().borrow().graph() {
                None => false,
                Some(graph) => graph.nodes().iter().any(|node| match node.borrow().data() {
                    None => false,
                    Some(data) => Ptr::ptr_eq(data.config().borrow().assets(), &resource),
                }),
            })
    }

    pub fn find_asset_manifest_project_and_container(
        &self,
        resource: &Ptr<AssetManifest>,
    ) -> Option<(Ptr<Project>, Ptr<Container>)> {
        self.projects.iter().find_map(|project_ptr| {
            let project = project_ptr.borrow();
            let data = project.data().borrow();
            let Some(graph) = data.graph() else {
                return None;
            };

            if let Some(container) = graph
                .nodes()
                .iter()
                .find(|node| match node.borrow().data() {
                    None => false,
                    Some(data) => Ptr::ptr_eq(data.config().borrow().assets(), &resource),
                })
            {
                Some((project_ptr.clone(), container.clone()))
            } else {
                None
            }
        })
    }

    pub fn find_asset_project(&self, resource: &Ptr<Asset>) -> Option<&Ptr<Project>> {
        self.projects
            .iter()
            .find(|project| match project.borrow().data().borrow().graph() {
                None => false,
                Some(graph) => graph.nodes().iter().any(|node| match node.borrow().data() {
                    None => false,
                    Some(data) => data
                        .config()
                        .borrow()
                        .assets()
                        .borrow()
                        .manifest()
                        .iter()
                        .any(|a| Ptr::ptr_eq(a, &resource)),
                }),
            })
    }

    pub fn find_asset_project_and_container(
        &self,
        resource: &Ptr<Asset>,
    ) -> Option<(Ptr<Project>, Ptr<Container>)> {
        self.projects.iter().find_map(|project_ptr| {
            let project = project_ptr.borrow();
            let data = project.data().borrow();
            let Some(graph) = data.graph() else {
                return None;
            };

            if let Some(container) = graph
                .nodes()
                .iter()
                .find(|node| match node.borrow().data() {
                    None => false,
                    Some(data) => data
                        .config()
                        .borrow()
                        .assets()
                        .borrow()
                        .manifest()
                        .iter()
                        .any(|a| Ptr::ptr_eq(a, &resource)),
                })
            {
                Some((project_ptr.clone(), container.clone()))
            } else {
                None
            }
        })
    }

    pub fn find_project_config_project(
        &self,
        resource: &Ptr<ProjectConfig>,
    ) -> Option<&Ptr<Project>> {
        self.projects
            .iter()
            .find(|project| match project.borrow().config() {
                Resource::NotPresent => false,
                Resource::Present(c) => Ptr::ptr_eq(c, &resource),
            })
    }

    pub fn find_analyses_project(&self, resource: &Ptr<Analyses>) -> Option<&Ptr<Project>> {
        self.projects.iter().find(|project| {
            if let Some(a) = project.borrow().analyses() {
                Ptr::ptr_eq(a, &resource)
            } else {
                false
            }
        })
    }

    pub fn find_data_project(&self, resource: &Ptr<Data>) -> Option<&Ptr<Project>> {
        self.projects
            .iter()
            .find(|project| Ptr::ptr_eq(project.borrow().data(), &resource))
    }

    pub fn find_container_project(&self, resource: &Ptr<Container>) -> Option<&Ptr<Project>> {
        self.projects
            .iter()
            .find(|project| match project.borrow().data().borrow().graph() {
                None => false,
                Some(graph) => graph
                    .nodes()
                    .iter()
                    .any(|node| Ptr::ptr_eq(node, &resource)),
            })
    }

    pub fn find_container_config_project(
        &self,
        resource: &Ptr<ContainerConfig>,
    ) -> Option<&Ptr<Project>> {
        self.projects
            .iter()
            .find(|project| match project.borrow().data().borrow().graph() {
                None => false,
                Some(graph) => graph.nodes().iter().any(|node| match node.borrow().data() {
                    None => false,
                    Some(data) => Ptr::ptr_eq(data.config(), &resource),
                }),
            })
    }

    pub fn find_container_config_container(
        &self,
        resource: &Ptr<ContainerConfig>,
    ) -> Option<Ptr<Container>> {
        let (_, container) = self.find_container_config_project_and_container(resource)?;
        Some(container)
    }

    pub fn find_container_config_project_and_container(
        &self,
        resource: &Ptr<ContainerConfig>,
    ) -> Option<(Ptr<Project>, Ptr<Container>)> {
        self.projects.iter().find_map(|project_ptr| {
            let project = project_ptr.borrow();
            let data = project.data().borrow();
            let Some(graph) = data.graph() else {
                return None;
            };

            if let Some(container) = graph
                .nodes()
                .iter()
                .find(|node| match node.borrow().data() {
                    None => false,
                    Some(data) => Ptr::ptr_eq(data.config(), &resource),
                })
            {
                Some((project_ptr.clone(), container.clone()))
            } else {
                None
            }
        })
    }

    pub fn find_asset_container(&self, resource: &Ptr<Asset>) -> Option<Ptr<Container>> {
        self.projects.iter().find_map(|project| {
            let project = project.borrow();
            let data = project.data().borrow();
            let Some(graph) = data.graph() else {
                return None;
            };

            graph
                .nodes()
                .iter()
                .find(|node| match node.borrow().data() {
                    None => false,
                    Some(data) => {
                        let config = data.config().borrow();
                        let asset_manifest = config.assets().borrow();
                        asset_manifest
                            .manifest()
                            .iter()
                            .any(|asset| Ptr::ptr_eq(asset, resource))
                    }
                })
                .cloned()
        })
    }
}

impl State {
    pub fn resource_path(&self, resource: AppResource) -> Option<PathBuf> {
        match resource {
            AppResource::File(resource) => self.file_resource_path(resource),
            AppResource::Folder(resource) => self.folder_resource_path(resource),
        }
    }

    pub fn file_resource_path(&self, resource: FileResource) -> Option<PathBuf> {
        match resource {
            FileResource::UserManifest(_) => Some(self.app.user_manifest.borrow().path.clone()),
            FileResource::ProjectManifest(_) => {
                Some(self.app.project_manifest.borrow().path.clone())
            }
            FileResource::ProjectProperties(resource) => {
                let resource = resource.upgrade()?;
                let project = self.find_project_properties_project(&resource)?;
                Some(common::project_file_of(project.borrow().path()))
            }
            FileResource::ProjectSettings(resource) => {
                let resource = resource.upgrade()?;
                let project = self.find_project_settings_project(&resource)?;
                Some(common::project_settings_file_of(project.borrow().path()))
            }
            FileResource::AnalysisManifest(resource) => {
                let resource = resource.upgrade()?;
                let project = self.find_analysis_manifest_project(&resource)?;
                Some(common::analyses_file_of(project.borrow().path()))
            }
            FileResource::Analysis(resource) => {
                let resource = resource.upgrade()?;
                let project = self.find_analysis_project(&resource)?;
                let project = project.borrow();
                let analyses = project.analyses()?;
                let analyses = analyses.borrow();
                let resource = resource.borrow();

                Some(project.path().join(analyses.path()).join(resource.path()))
            }
            FileResource::ContainerProperties(resource) => {
                let resource = resource.upgrade()?;
                self.container_properties_path(&resource)
            }
            FileResource::ContainerSettings(resource) => {
                let resource = resource.upgrade()?;
                self.container_settings_path(&resource)
            }
            FileResource::AssetManifest(resource) => {
                let resource = resource.upgrade()?;
                self.asset_manifest_path(&resource)
            }
            FileResource::Asset(resource) => {
                let resource = resource.upgrade()?;
                self.asset_path(&resource)
            }
        }
    }

    pub fn folder_resource_path(&self, resource: FolderResource) -> Option<PathBuf> {
        match resource {
            FolderResource::Project(resource) => {
                let resource = resource.upgrade()?;
                self.projects().iter().find_map(|prj| {
                    if Ptr::ptr_eq(prj, &resource) {
                        Some(prj.borrow().path().clone())
                    } else {
                        None
                    }
                })
            }
            FolderResource::ProjectConfig(resource) => {
                let resource = resource.upgrade()?;
                self.project_config_path(&resource)
            }
            FolderResource::Analyses(resource) => {
                let resource = resource.upgrade()?;
                let project = self.find_analyses_project(&resource)?;
                let path = project.borrow().path().join(resource.borrow().path());
                Some(path)
            }
            FolderResource::Data(resource) => {
                let resource = resource.upgrade()?;
                self.data_path(&resource)
            }
            FolderResource::Container(resource) => {
                let resource = resource.upgrade()?;
                self.container_path(&resource)
            }
            FolderResource::ContainerConfig(resource) => {
                let resource = resource.upgrade()?;
                self.container_config_path(&resource)
            }
        }
    }

    /// # Returns
    /// Full path to the container if it exists within the project.
    fn container_path_with_project(
        &self,
        project: &Ptr<Project>,
        container: &Ptr<Container>,
    ) -> Option<PathBuf> {
        let project = project.borrow();
        let data = project.data().borrow();
        let graph = data.graph()?;
        let container_path = graph.path(&container).unwrap();

        Some(project.path().join(data.path()).join(container_path))
    }

    pub fn container_properties_path(
        &self,
        resource: &Ptr<ContainerProperties>,
    ) -> Option<PathBuf> {
        let (project, container) =
            self.find_container_properties_project_and_container(resource)?;

        let container_path = self
            .container_path_with_project(&project, &container)
            .unwrap();

        Some(common::container_file_of(container_path))
    }

    pub fn container_settings_path(&self, resource: &Ptr<ContainerSettings>) -> Option<PathBuf> {
        let (project, container) = self.find_container_settings_project_and_container(resource)?;
        let container_path = self
            .container_path_with_project(&project, &container)
            .unwrap();

        Some(common::container_file_of(container_path))
    }

    pub fn asset_manifest_path(&self, resource: &Ptr<AssetManifest>) -> Option<PathBuf> {
        let (project, container) = self.find_asset_manifest_project_and_container(resource)?;
        let container_path = self
            .container_path_with_project(&project, &container)
            .unwrap();

        Some(common::assets_file_of(container_path))
    }

    pub fn asset_path(&self, resource: &Ptr<Asset>) -> Option<PathBuf> {
        let (project, container) = self.find_asset_project_and_container(resource)?;
        let container_path = self
            .container_path_with_project(&project, &container)
            .unwrap();

        Some(common::assets_file_of(container_path))
    }

    pub fn project_config_path(&self, resource: &Ptr<ProjectConfig>) -> Option<PathBuf> {
        let project = self.find_project_config_project(resource)?;
        Some(common::app_dir_of(project.borrow().path()))
    }

    pub fn data_path(&self, resource: &Ptr<Data>) -> Option<PathBuf> {
        let project = self.find_data_project(&resource)?;
        let project = project.borrow();
        let data = project.data().borrow();
        Some(project.path().join(data.path()))
    }

    pub fn container_path(&self, resource: &Ptr<Container>) -> Option<PathBuf> {
        let project = self.find_container_project(&resource)?;
        let project = project.borrow();
        let data = project.data().borrow();
        let graph = data.graph()?;
        let container_path = graph.path(&resource).unwrap();

        let path = project.path().join(data.path());
        let path = path.parent().unwrap(); // account for doubling of data root
        Some(path.join(container_path))
    }

    pub fn container_config_path(&self, resource: &Ptr<ContainerConfig>) -> Option<PathBuf> {
        let (project, container) = self.find_container_config_project_and_container(resource)?;
        let container_path = self
            .container_path_with_project(&project, &container)
            .unwrap();

        Some(common::app_dir_of(container_path))
    }
}

impl State {
    /// Duplicate the state.
    /// All `fs` references point to the original resource.
    pub fn duplicate_with_fs_references_and_map(&self) -> (Self, ProjectMap) {
        let mut project_map = Vec::with_capacity(self.projects.len());
        let projects = self
            .projects
            .iter()
            .map(|project| {
                let (dup, container_map) = project.borrow().duplicate_with_fs_references_and_map();
                let dup = Ptr::new(dup);
                project_map.push(((project.clone(), dup.clone()), container_map));
                dup
            })
            .collect();

        (
            Self {
                app: self.app.clone(),
                projects,
            },
            project_map,
        )
    }
}

#[derive(Debug, derive_more::From)]
pub enum Action {
    App(AppAction),
    Project(ProjectAction),
}

#[derive(Debug)]
pub enum AppAction {
    UserManifest(ManifestAction),
    ProjectManifest(ManifestAction),
}

#[derive(Debug)]
pub enum ProjectAction {
    Create(PathBuf),
    Remove(PathBuf),
    SetPath {
        project: PathBuf,
        to: PathBuf,
    },
    Config {
        project: PathBuf,
        action: ConfigAction,
    },
    Analyses {
        project: PathBuf,
        action: ManifestAction,
    },
    Data {
        project: PathBuf,
        action: DataAction,
    },
}

#[derive(Debug)]
pub enum DataAction {
    InitializeGraph,
    RemoveGraph,
    InsertContainer {
        /// Parent relative to data root.
        parent: PathBuf,
        name: OsString,
    },

    RemoveContainer(
        /// Path relative to data root.
        PathBuf,
    ),

    ContainerConfig {
        /// Path relative to data root.
        container: PathBuf,
        action: ConfigAction,
    },
}

#[derive(Debug, derive_more::From)]
pub enum ConfigAction {
    Insert,
    Remove,
    Manifest(ManifestAction),
}

#[derive(Debug)]
pub enum ManifestAction {
    AddItem(String),
    RemoveItem(usize),
}

impl Reducible for State {
    type Action = Action;
    fn reduce(&mut self, action: Self::Action) {
        match action {
            Action::App(action) => self.reduce_app(action),
            Action::Project(action) => self.reduce_project(action),
        }
    }
}

impl State {
    fn reduce_app(&mut self, action: AppAction) {
        match action {
            AppAction::UserManifest(action) => self.reduce_app_user_manifest(action),
            AppAction::ProjectManifest(action) => self.reduce_app_project_manifest(action),
        }
    }

    fn reduce_app_user_manifest(&mut self, action: ManifestAction) {
        match action {
            ManifestAction::AddItem(_) => todo!(),
            ManifestAction::RemoveItem(index) => {
                self.app.user_manifest.borrow_mut().remove(index);
            }
        }
    }

    fn reduce_app_project_manifest(&mut self, action: ManifestAction) {
        match action {
            ManifestAction::AddItem(path) => {
                let path = PathBuf::from(path);
                self.app.project_manifest.borrow_mut().push(path.clone());
                let project = Project::new(path, "data");
                self.projects.push(Ptr::new(project));
            }
            ManifestAction::RemoveItem(index) => {
                self.app.project_manifest.borrow_mut().remove(index);
            }
        }
    }

    fn reduce_project(&mut self, action: ProjectAction) {
        match action {
            ProjectAction::Create(path) => {
                if !self
                    .projects
                    .iter()
                    .any(|project| project.borrow().path() == &path)
                {
                    let project = Project::new(path, "data");
                    self.projects.push(Ptr::new(project));
                }
            }
            ProjectAction::Remove(path) => self
                .projects
                .retain(|project| project.borrow().path() != &path),
            ProjectAction::SetPath { project, to } => {
                let project = self
                    .projects
                    .iter()
                    .find(|prj| prj.borrow().path() == &project)
                    .unwrap();

                project.borrow_mut().set_path(to);
            }
            ProjectAction::Config { project, action } => {
                let project = self.find_path_project(project).unwrap().clone();
                self.reduce_project_config(project, action);
            }
            ProjectAction::Analyses { project, action } => unreachable!(),
            ProjectAction::Data { project, action } => {
                let project = self.find_path_project(project).unwrap().clone();
                self.reduce_project_data(project, action)
            }
        }
    }

    fn reduce_project_config(&mut self, project: Ptr<Project>, action: ConfigAction) {
        match action {
            ConfigAction::Insert => {
                project.borrow_mut().insert_config();
            }
            ConfigAction::Remove => {
                project.borrow_mut().remove_config();
            }
            ConfigAction::Manifest(_) => todo!(),
        }
    }

    fn reduce_project_data(&mut self, project: Ptr<Project>, action: DataAction) {
        match action {
            DataAction::InitializeGraph => {
                let project = project.borrow();
                let mut data = project.data().borrow_mut();
                data.initialize_graph();
            }
            DataAction::RemoveGraph => {
                let project = project.borrow();
                let mut data = project.data().borrow_mut();
                data.remove_graph();
            }
            DataAction::InsertContainer { parent, name } => todo!(),
            DataAction::RemoveContainer(_) => todo!(),
            DataAction::ContainerConfig { container, action } => {
                let project = project.borrow();
                let data = project.data.borrow();
                let graph = data.graph().unwrap();
                let container = graph.find_by_path(container).unwrap();
                let mut container = container.borrow_mut();
                match action {
                    ConfigAction::Insert => {
                        container.set_data(ContainerData::new());
                    }
                    ConfigAction::Remove => {
                        container.remove_data();
                    }
                    ConfigAction::Manifest(_) => todo!(),
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct AppState {
    user_manifest: Ptr<UserManifest>,
    project_manifest: Ptr<ProjectManifest>,
}

impl AppState {
    pub fn new(user_manifest: impl Into<PathBuf>, project_manifest: impl Into<PathBuf>) -> Self {
        Self {
            user_manifest: Ptr::new(UserManifest::new(user_manifest)),
            project_manifest: Ptr::new(ProjectManifest::new(project_manifest)),
        }
    }

    pub fn user_manifest(&self) -> &Ptr<UserManifest> {
        &self.user_manifest
    }

    pub fn project_manifest(&self) -> &Ptr<ProjectManifest> {
        &self.project_manifest
    }
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        let user_manifest = Ptr::new(self.user_manifest.borrow().clone());
        let project_manifest = Ptr::new(self.project_manifest.borrow().clone());
        Self {
            user_manifest,
            project_manifest,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserManifest {
    path: PathBuf,
    manifest: Vec<String>,
}

impl UserManifest {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            manifest: vec![],
        }
    }
}

impl HasPath for UserManifest {
    fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Manifest for UserManifest {
    type Item = String;
    fn manifest(&self) -> &Vec<Self::Item> {
        &self.manifest
    }

    fn push(&mut self, value: Self::Item) {
        self.manifest.push(value);
    }

    fn remove(&mut self, index: usize) -> Self::Item {
        self.manifest.swap_remove(index)
    }
}

#[derive(Debug, Clone)]
pub struct ProjectManifest {
    path: PathBuf,
    manifest: Vec<PathBuf>,
}

impl ProjectManifest {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            manifest: vec![],
        }
    }
}

impl HasPath for ProjectManifest {
    fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Manifest for ProjectManifest {
    type Item = PathBuf;
    fn manifest(&self) -> &Vec<Self::Item> {
        &self.manifest
    }

    fn push(&mut self, value: Self::Item) {
        self.manifest.push(value);
    }

    fn remove(&mut self, index: usize) -> Self::Item {
        self.manifest.swap_remove(index)
    }
}

#[derive(Debug, HasId)]
pub struct Project {
    #[id]
    rid: ResourceId,

    /// Path to the project's base folder.
    path: PathBuf,

    config: Resource<ProjectConfig>,

    /// Analyses folder.
    analyses: Option<Ptr<Analyses>>,

    /// Data folder.
    data: Ptr<Data>,
}

impl Project {
    pub fn new(path: impl Into<PathBuf>, data_path: impl Into<PathBuf>) -> Self {
        Self {
            rid: ResourceId::new(),
            path: path.into(),
            config: Resource::NotPresent,
            data: Ptr::new(Data::new(data_path)),
            analyses: None,
        }
    }

    pub fn rid(&self) -> &ResourceId {
        &self.rid
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn set_path(&mut self, path: impl Into<PathBuf>) {
        self.path = path.into();
    }

    pub fn config(&self) -> &Resource<ProjectConfig> {
        &self.config
    }

    pub fn insert_config(&mut self) {
        assert!(!self.config.is_present());
        let config = ProjectConfig::new();
        self.config = Resource::Present(Ptr::new(config));
    }

    pub fn remove_config(&mut self) {
        self.config = Resource::NotPresent;
    }

    pub fn analyses(&self) -> Option<&Ptr<Analyses>> {
        self.analyses.as_ref()
    }

    pub fn data(&self) -> &Ptr<Data> {
        &self.data
    }

    pub fn set_data_root(&mut self, name: impl Into<OsString>) {
        self.data.borrow_mut().set_graph_root(name);
    }

    pub fn remove_data_root(&mut self) {
        self.data.borrow_mut().remove_graph();
    }
}

impl Project {
    pub fn sync_with_fs(&mut self, fs: &super::fs::State) {
        if let Some(_root) = fs.find_folder(&self.path) {
            if let Some(_config) = fs.find_folder(common::app_dir_of(&self.path)) {
                if !self.config.is_present() {
                    self.insert_config();
                }
            } else {
                self.remove_config();
            }

            let data_path = self.data.borrow().path().clone();
            if let Some(root) = fs.find_folder(self.path.join(data_path)) {
                let mut data = self.data.borrow_mut();
                if data.graph().is_none() {
                    data.initialize_graph();
                }

                data.sync_with_fs(&root, fs)
            } else {
                if self.data.borrow().graph().is_some() {
                    self.remove_data_root();
                }
            }
        } else {
            self.remove_config();
            self.remove_data_root();
        }
    }
}

impl Project {
    /// Duplicates the project.
    /// All `fs` references point to their original resource.
    ///
    /// # Returns
    /// Tuple of (duplicate, [(orginal container, duplicate container)]).
    pub fn duplicate_with_fs_references_and_map(&self) -> (Self, ContainerMap) {
        let config = if let Resource::Present(config) = &self.config {
            Resource::Present(Ptr::new(config.borrow().duplicate_with_fs_references()))
        } else {
            Resource::NotPresent
        };

        let (data, data_map) = self.data.borrow().duplicate_with_app_resource_and_map();
        let analyses = self
            .analyses
            .clone()
            .map(|analyses| Ptr::new(analyses.borrow().clone()));

        (
            Self {
                rid: self.rid.clone(),
                path: self.path.clone(),
                config,
                analyses,
                data: Ptr::new(data),
            },
            data_map,
        )
    }
}

#[derive(Debug)]
pub struct ProjectConfig {
    properties: Ptr<ProjectProperties>,
    settings: Ptr<ProjectSettings>,
    analyses: Ptr<AnalysisManifest>,
}

impl ProjectConfig {
    pub fn new() -> Self {
        Self {
            properties: Ptr::new(ProjectProperties),
            settings: Ptr::new(ProjectSettings),
            analyses: Ptr::new(AnalysisManifest::new()),
        }
    }

    pub fn properties(&self) -> &Ptr<ProjectProperties> {
        &self.properties
    }

    pub fn settings(&self) -> &Ptr<ProjectSettings> {
        &self.settings
    }

    pub fn analyses(&self) -> &Ptr<AnalysisManifest> {
        &self.analyses
    }
}

impl ProjectConfig {
    pub fn duplicate_with_fs_references(&self) -> Self {
        Self {
            properties: Ptr::new(self.properties.borrow().clone()),
            settings: Ptr::new(self.settings.borrow().clone()),
            analyses: Ptr::new(self.analyses.borrow().clone()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProjectProperties;

#[derive(Debug, Clone)]
pub struct ProjectSettings;

#[derive(Debug, Clone)]
pub struct AnalysisManifest {
    manifest: Vec<Ptr<Analysis>>,
}

impl AnalysisManifest {
    pub fn new() -> Self {
        Self { manifest: vec![] }
    }
}

impl Manifest for AnalysisManifest {
    type Item = Ptr<Analysis>;
    fn manifest(&self) -> &Vec<Self::Item> {
        &self.manifest
    }

    fn push(&mut self, value: Self::Item) {
        self.manifest.push(value);
    }

    fn remove(&mut self, index: usize) -> Self::Item {
        self.manifest.swap_remove(index)
    }
}

/// Project analysis folder.
#[derive(Debug, Clone)]
pub struct Analyses {
    /// Path to the analysis folder.
    path: PathBuf,
}

impl Analyses {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[derive(Debug, HasId)]
pub struct Analysis {
    #[id]
    rid: ResourceId,

    /// Path relative to the analysis root.
    path: PathBuf,
}

impl Analysis {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            rid: ResourceId::new(),
            path: path.into(),
        }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

/// Project data folder.
#[derive(Debug)]
pub struct Data {
    /// Path to the data root.
    ///
    /// # Notes
    /// + Includes the data root's name,
    /// so must aware when `join`ing paths
    /// or it will be doubled.
    path: PathBuf,

    /// Data graph.
    /// `None` if a folder is not at the root path.
    graph: Option<Tree<Container>>,
}

impl Data {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            graph: None,
        }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn set_path(&mut self, path: impl Into<PathBuf>) {
        self.path = path.into();
    }

    pub fn graph(&self) -> Option<&Tree<Container>> {
        self.graph.as_ref()
    }

    pub fn graph_mut(&mut self) -> &mut Option<Tree<Container>> {
        &mut self.graph
    }

    pub fn initialize_graph(&mut self) {
        assert!(self.graph.is_none());
        let root = Container::new(self.path().file_name().unwrap());
        let graph = Tree::new(root);
        let _ = self.graph.insert(graph);
    }

    pub fn remove_graph(&mut self) {
        assert!(self.graph.is_some());
        let _ = self.graph.take();
    }

    /// Sets the graph's root.
    /// Removes previous graph.
    pub fn set_graph_root(&mut self, name: impl Into<OsString>) {
        let root = Container::new(name);
        let graph = Tree::new(root);
        self.graph = Some(graph);
    }

    /// Remove a container from the graph.
    ///
    /// # Panics
    /// + If `graph` is `None`.
    pub fn remove_container(&mut self, container: &Ptr<Container>) {
        let graph = self.graph.as_mut().unwrap();
        if Ptr::ptr_eq(&graph.root(), &container) {
            self.graph = None;
        } else {
            graph.remove(container).unwrap();
        }
    }
}

impl Data {
    pub fn sync_with_fs(&mut self, root: &Ptr<super::fs::Folder>, fs: &super::fs::State) {
        fn create_container(root: &Ptr<super::fs::Folder>, fs: &super::fs::State) -> Container {
            let mut container = Container::new(root.borrow().name());
            if fs
                .graph()
                .children(root)
                .unwrap()
                .iter()
                .any(|child| child.borrow().name() == common::app_dir())
            {
                let data = ContainerData::new();
                let config = data.config().borrow();
                let mut assets = config.assets().borrow_mut();
                for file in root.borrow().files() {
                    let asset = Asset::new(file.borrow().name());
                    let asset = Ptr::new(asset);
                    assets.push(asset);
                }

                drop(assets);
                drop(config);
                container.set_data(data);
            }

            container
        }

        fn build_tree(
            folder: &Ptr<super::fs::Folder>,
            container: &Ptr<Container>,
            fs: &super::fs::State,
            graph: &mut Tree<Container>,
        ) {
            for child_folder in fs.graph().children(&folder).unwrap() {
                let child = create_container(&child_folder, fs);
                let child = graph.insert(child, container).unwrap();
                build_tree(&child_folder, &child, fs, graph)
            }
        }

        assert_eq!(root.borrow().name(), self.path);
        let container = create_container(root, fs);
        let mut graph = Tree::new(container);
        build_tree(root, &graph.root(), fs, &mut graph);
    }
}

impl Data {
    /// Duplicates the the `Data` struct.
    /// The `project` reference and all app references in the `graph`
    /// point to the original resource.
    ///
    /// # Returns
    /// Tuple of (duplicate, container map) where `container map` is `[(original, duplicate)]`.
    /// `(None, [])` if `graph` is `None`.
    pub fn duplicate_with_app_resource_and_map(&self) -> (Self, ContainerMap) {
        let (graph, node_map) = if let Some(graph) = &self.graph {
            let (graph, node_map) = graph.duplicate_with_map();
            (Some(graph), node_map)
        } else {
            (None, vec![])
        };

        (
            Self {
                path: self.path.clone(),
                graph,
            },
            node_map,
        )
    }
}

/// Any folder in a Project's data directory.
/// It may or may not have Container data.
/// This is represented in the `data` field.
#[derive(Debug, Clone)]
pub struct Container {
    name: OsString,
    data: Option<ContainerData>,
}

impl Container {
    pub fn new(name: impl Into<OsString>) -> Self {
        Self {
            name: name.into(),
            data: None,
        }
    }

    pub fn rid(&self) -> Option<ResourceId> {
        if let Some(data) = &self.data {
            Some(data.rid().clone())
        } else {
            None
        }
    }

    pub fn data(&self) -> &Option<ContainerData> {
        &self.data
    }

    pub fn set_data(&mut self, data: ContainerData) {
        assert!(self.data.is_none());
        let _ = self.data.insert(data);
    }

    pub fn remove_data(&mut self) -> Option<ContainerData> {
        assert!(self.data.is_some());
        self.data.take()
    }
}

#[derive(Debug, HasId, Clone)]
pub struct ContainerData {
    #[id]
    rid: ResourceId,
    config: Ptr<ContainerConfig>,
}

impl ContainerData {
    pub fn new() -> Self {
        Self {
            rid: ResourceId::new(),
            config: Ptr::new(ContainerConfig::new()),
        }
    }

    pub fn rid(&self) -> &ResourceId {
        &self.rid
    }

    pub fn config(&self) -> &Ptr<ContainerConfig> {
        &self.config
    }
}

#[derive(Debug, Clone)]
pub struct ContainerConfig {
    properties: Ptr<ContainerProperties>,
    settings: Ptr<ContainerSettings>,
    assets: Ptr<AssetManifest>,
}

impl ContainerConfig {
    pub fn new() -> Self {
        Self {
            properties: Ptr::new(ContainerProperties),
            settings: Ptr::new(ContainerSettings),
            assets: Ptr::new(AssetManifest::new()),
        }
    }

    pub fn properties(&self) -> &Ptr<ContainerProperties> {
        &self.properties
    }

    pub fn settings(&self) -> &Ptr<ContainerSettings> {
        &self.settings
    }

    pub fn assets(&self) -> &Ptr<AssetManifest> {
        &self.assets
    }
}

#[derive(Debug)]
pub struct ContainerProperties;

#[derive(Debug)]
pub struct ContainerSettings;

#[derive(Debug)]
pub struct AssetManifest {
    manifest: Vec<Ptr<Asset>>,
}

impl AssetManifest {
    pub fn new() -> Self {
        Self { manifest: vec![] }
    }
}

impl Manifest for AssetManifest {
    type Item = Ptr<Asset>;
    fn manifest(&self) -> &Vec<Self::Item> {
        &self.manifest
    }

    fn push(&mut self, value: Self::Item) {
        self.manifest.push(value);
    }

    fn remove(&mut self, index: usize) -> Self::Item {
        self.manifest.swap_remove(index)
    }
}

#[derive(Debug, HasId)]
pub struct Asset {
    #[id]
    rid: ResourceId,
    name: OsString,
}

impl Asset {
    pub fn new(name: impl Into<OsString>) -> Self {
        Self {
            rid: ResourceId::new(),
            name: name.into(),
        }
    }
}

#[derive(Debug)]
pub enum Resource<T> {
    NotPresent,
    Present(Ptr<T>),
}

impl<T> Resource<T> {
    pub fn is_present(&self) -> bool {
        match self {
            Self::Present { .. } => true,
            Self::NotPresent => false,
        }
    }
}

#[derive(Debug)]
pub enum DataResource<T> {
    NotPresent,
    Present {
        resource: Ptr<T>,
        state: DataResourceState,
    },
}

impl<T> DataResource<T> {
    pub fn is_present(&self) -> bool {
        match self {
            Self::Present { .. } => true,
            Self::NotPresent => false,
        }
    }
}

#[derive(Debug)]
pub enum FsResource<T> {
    NotPresent,
    Present(WPtr<T>),
}

impl<T> FsResource<T> {
    pub fn is_present(&self) -> bool {
        match self {
            Self::Present { .. } => true,
            Self::NotPresent => false,
        }
    }
}

#[derive(Debug)]
pub enum FsDataResource<T> {
    NotPresent,
    Present {
        resource: WPtr<T>,
        state: DataResourceState,
    },
}

impl<T> FsDataResource<T> {
    pub fn is_present(&self) -> bool {
        match self {
            Self::Present { .. } => true,
            Self::NotPresent => false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum DataResourceState {
    Valid,
    Invalid,
}

#[derive(Clone)]
pub enum FileResource {
    UserManifest(WPtr<UserManifest>),
    ProjectManifest(WPtr<ProjectManifest>),
    ProjectProperties(WPtr<ProjectProperties>),
    ProjectSettings(WPtr<ProjectSettings>),
    AnalysisManifest(WPtr<AnalysisManifest>),
    Analysis(WPtr<Analysis>),
    ContainerProperties(WPtr<ContainerProperties>),
    ContainerSettings(WPtr<ContainerSettings>),
    AssetManifest(WPtr<AssetManifest>),
    Asset(WPtr<Asset>),
}

impl std::fmt::Debug for FileResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileResource::UserManifest(ptr) => {
                f.write_fmt(format_args!("UserManifest [{:?}]", ptr.as_ptr()))
            }
            FileResource::ProjectManifest(ptr) => {
                f.write_fmt(format_args!("ProjectManifest [{:?}]", ptr.as_ptr()))
            }
            FileResource::ProjectProperties(ptr) => {
                f.write_fmt(format_args!("ProjectProperties [{:?}]", ptr.as_ptr()))
            }
            FileResource::ProjectSettings(ptr) => {
                f.write_fmt(format_args!("ProjectSettings [{:?}]", ptr.as_ptr()))
            }
            FileResource::AnalysisManifest(ptr) => {
                f.write_fmt(format_args!("AnalysisManifest [{:?}]", ptr.as_ptr()))
            }
            FileResource::Analysis(ptr) => {
                f.write_fmt(format_args!("Analysis [{:?}]", ptr.as_ptr()))
            }
            FileResource::ContainerProperties(ptr) => {
                f.write_fmt(format_args!("ContainerProperties [{:?}]", ptr.as_ptr()))
            }
            FileResource::ContainerSettings(ptr) => {
                f.write_fmt(format_args!("ContainerSettings [{:?}]", ptr.as_ptr()))
            }
            FileResource::AssetManifest(ptr) => {
                f.write_fmt(format_args!("AssetManifest [{:?}]", ptr.as_ptr()))
            }
            FileResource::Asset(ptr) => f.write_fmt(format_args!("Asset [{:?}]", ptr.as_ptr())),
        }
    }
}

#[derive(Clone)]
pub enum FolderResource {
    Project(WPtr<Project>),
    ProjectConfig(WPtr<ProjectConfig>),
    Analyses(WPtr<Analyses>),
    /// Indicates the resource points to the data root,
    /// but the data root has not yet been created.
    Data(WPtr<Data>),
    Container(WPtr<Container>),
    ContainerConfig(WPtr<ContainerConfig>),
}

impl std::fmt::Debug for FolderResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FolderResource::Project(ptr) => {
                f.write_fmt(format_args!("Project [{:?}]", ptr.as_ptr()))
            }
            FolderResource::ProjectConfig(ptr) => {
                f.write_fmt(format_args!("ProjectConfig [{:?}]", ptr.as_ptr()))
            }
            FolderResource::Analyses(ptr) => {
                f.write_fmt(format_args!("Analyses [{:?}]", ptr.as_ptr()))
            }
            FolderResource::Data(ptr) => f.write_fmt(format_args!("Data [{:?}]", ptr.as_ptr())),
            FolderResource::Container(ptr) => {
                f.write_fmt(format_args!("Container [{:?}]", ptr.as_ptr()))
            }
            FolderResource::ContainerConfig(ptr) => {
                f.write_fmt(format_args!("ContainerConfig [{:?}]", ptr.as_ptr()))
            }
        }
    }
}

#[derive(Clone, Debug, derive_more::From)]
pub enum AppResource {
    File(FileResource),
    Folder(FolderResource),
}

impl HasName for Container {
    fn name(&self) -> &std::ffi::OsStr {
        &self.name
    }

    fn set_name(&mut self, name: impl Into<std::ffi::OsString>) {
        self.name = name.into()
    }
}

impl HasName for Asset {
    fn name(&self) -> &std::ffi::OsStr {
        &self.name
    }

    fn set_name(&mut self, name: impl Into<std::ffi::OsString>) {
        self.name = name.into()
    }
}

impl<T> Clone for Resource<T> {
    fn clone(&self) -> Self {
        match self {
            Self::NotPresent => Self::NotPresent,
            Self::Present(ptr) => Self::Present(ptr.clone()),
        }
    }
}

impl<T> Clone for DataResource<T> {
    fn clone(&self) -> Self {
        match self {
            Self::NotPresent => Self::NotPresent,
            Self::Present { resource, state } => Self::Present {
                resource: resource.clone(),
                state: state.clone(),
            },
        }
    }
}

impl<T> Clone for FsResource<T> {
    fn clone(&self) -> Self {
        match self {
            Self::NotPresent => Self::NotPresent,
            Self::Present(ptr) => Self::Present(ptr.clone()),
        }
    }
}

impl<T> Clone for FsDataResource<T> {
    fn clone(&self) -> Self {
        match self {
            Self::NotPresent => Self::NotPresent,
            Self::Present { resource, state } => Self::Present {
                resource: resource.clone(),
                state: state.clone(),
            },
        }
    }
}

pub trait HasPath {
    fn path(&self) -> &PathBuf;
}

pub trait Manifest {
    type Item;
    fn manifest(&self) -> &Vec<Self::Item>;
    fn push(&mut self, value: Self::Item);
    fn remove(&mut self, index: usize) -> Self::Item;
}
