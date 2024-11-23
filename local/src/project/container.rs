//! High level functionality related to Containers.
#[cfg(feature = "fs")]
pub use functions::*;

#[cfg(feature = "fs")]
pub mod functions {
    use super::{builder, error};
    use crate::common::container_file_of;
    use std::path::Path;
    use syre_core::types::ResourceId;

    /// Convenience function to create a new folder as a `Container`.
    ///
    /// Equivalent to
    /// ```
    /// let builder = InitOptions::new();
    /// builder.build(path)?;
    /// ```
    pub fn new(path: impl AsRef<Path>) -> Result<ResourceId, error::Build> {
        let builder = builder::InitOptions::new();
        builder.build(path.as_ref())
    }

    /// Returns whether or not the path is a Container.
    /// Checks if <path>/<APP_DIR>/<CONTAINER_FILE> exists.
    pub fn path_is_container(path: &Path) -> bool {
        let c_path = container_file_of(path);
        c_path.exists()
    }
}

#[cfg(feature = "fs")]
pub mod builder {
    //! Build containers.
    use super::{
        super::{project, resources::Container},
        error,
    };
    use crate::{
        common::app_dir, loader::container::Loader as ContainerLoader, project::resources,
    };
    use std::{
        fs,
        path::{self, Path, PathBuf},
    };
    use syre_core::{
        project::{Asset, ContainerProperties},
        types::ResourceId,
    };

    /// Initialize a new folder.
    #[derive(Default)]
    pub struct InitNew {
        properties: Option<ContainerProperties>,
    }

    impl InitNew {
        pub fn properties(&self) -> Option<&ContainerProperties> {
            self.properties.as_ref()
        }

        pub fn set_properties(&mut self, properties: ContainerProperties) {
            self.properties = Some(properties);
        }

        pub fn unset_properties(&mut self) {
            self.properties = None;
        }
    }

    /// Initialize an existing folder.
    #[derive(Default)]
    pub struct InitExisting {
        recursive: bool,

        /// Create new ids for resources.
        new_ids: bool,

        /// glob patterns to ignore.
        ignore: Vec<String>,
    }

    impl InitExisting {
        pub fn set_recursive(&mut self, recursive: bool) {
            self.recursive = recursive;
        }

        pub fn with_new_ids(&mut self, new_ids: bool) {
            self.new_ids = new_ids;
        }

        pub fn ignored(&self) -> &Vec<String> {
            &self.ignore
        }

        pub fn ignore(&mut self, pattern: impl Into<String>) {
            self.ignore.push(pattern.into());
        }
    }

    #[derive(Default)]
    pub struct InitOptions<I> {
        init: I,
        init_assets: bool,
    }

    impl<I> InitOptions<I> {
        /// Initialize files as `Asset`s.
        pub fn with_assets(&mut self) {
            self.init_assets = true;
        }

        /// Do not initialize files as `Asset`s.
        pub fn without_assets(&mut self) {
            self.init_assets = false;
        }
    }

    impl InitOptions<InitNew> {
        /// Create a new folder as a `Container`.
        pub fn new() -> Self {
            InitOptions::default()
        }

        /// Use the given properties to initialize the `Container`.
        ///
        /// # Notes
        /// + `name` is ignored and will be replaced by the folder's name.
        pub fn properties(&mut self, properties: ContainerProperties) {
            self.init.set_properties(properties);
        }

        /// Clears properties.
        pub fn unset_properties(&mut self) {
            self.init.unset_properties();
        }

        /// Run the intialization.
        ///
        /// # Returns
        /// [`ResourceId`] of the [`Container`](CoreContainer).
        pub fn build(&self, path: impl AsRef<Path>) -> Result<ResourceId, error::Build> {
            let path = path.as_ref();
            if path.exists() && project::path_is_resource(path) {
                return Err(error::Build::AlreadyResource);
            }

            let mut container = Container::new(path);
            if let Some(properties) = self.init.properties() {
                container.properties = properties.clone();
            }

            container.properties.name = container
                .base_path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();

            if self.init_assets {
                for entry in fs::read_dir(container.base_path()).unwrap() {
                    let entry = entry.unwrap();
                    let entry_path = entry.path();
                    if entry_path.is_file() {
                        let asset = Asset::new(entry_path);
                        container.assets.push(asset);
                    }
                }
            }

            if let Err(err) = container.save() {
                let err = match err {
                    resources::container::error::Save::CreateDir(error) => error,
                    resources::container::error::Save::SaveFiles {
                        properties,
                        assets,
                        settings,
                    } => {
                        if let Some(err) = properties {
                            err
                        } else if let Some(err) = assets {
                            err
                        } else {
                            settings.unwrap()
                        }
                    }
                };

                return Err(error::Build::Save(err.kind()));
            }

            Ok(container.rid().clone())
        }
    }

    impl InitOptions<InitExisting> {
        /// Initialize an existing folder or folder tree as a `Container`.
        pub fn init() -> Self {
            InitOptions::default()
        }

        /// Set whether to recurse into subfolders.
        pub fn recurse(&mut self, recursive: bool) {
            self.init.set_recursive(recursive);
        }

        /// Set whether new ids should be assigned to resources.
        pub fn with_new_ids(&mut self, new_ids: bool) {
            self.init.with_new_ids(new_ids);
        }

        /// Ignore a path and it's subfolders when recursing.
        /// Ignored if `recurse` is `false`.
        ///
        /// # Arguments
        /// + `pattern`: A glob pattern to ignore, relative to the
        /// `Container` graph root.
        pub fn ignore(&mut self, pattern: impl Into<String>) {
            self.init.ignore(pattern);
        }

        /// Intialize the path as a `Container` tree.
        ///
        /// # Returns
        /// [`ResourceId`] of the root [`Container`](CoreContainer).
        ///
        /// # Errors
        /// + If a path is already a resource but can not be loaded as a Container.
        ///
        /// # Notes
        /// + If `path` is already initialized as a `Container` it is re-initialized,
        /// with all properties being maintained, but `Asset`s being updated.
        ///  - If `recurse` is `true`, this applies for folders within the subtree, too.
        ///
        /// + `Container` name will be updated to match the folder.
        /// + Hidden files (i.e. Files whose name starts with a period (.)) are ignored as `Asset`s.
        pub fn build(&self, path: impl AsRef<Path>) -> Result<ResourceId, error::Build> {
            /// Initialize a path as a Container.
            /// Used to recurse.
            ///
            /// # Arguments
            /// + `ignore`: Absolute paths to ignore.
            ///     No effect if `recurse` is `false`.
            ///
            /// # Notes
            /// + Hidden files are ignored as `Asset`s.
            fn init_container(
                path: impl Into<PathBuf>,
                init_assets: bool,
                new_ids: bool,
                recurse: bool,
                ignore: &Vec<PathBuf>,
            ) -> Result<ResourceId, error::Build> {
                let path: PathBuf = path.into();
                // TODO: What if path is a project?
                let mut container = if super::path_is_container(&path) {
                    let container_state = match ContainerLoader::load(&path) {
                        Ok(container) => container,
                        Err(_state) => return Err(error::Build::Load),
                    };

                    if new_ids {
                        let mut container = Container::new(&path);
                        container.container.properties = container_state.properties.clone();
                        container.container.assets = container_state
                            .assets
                            .clone()
                            .into_iter()
                            .map(|asset_state| {
                                let mut asset = Asset::new(asset_state.path.clone());
                                asset.properties = asset_state.properties;
                                asset
                            })
                            .collect();

                        container
                    } else {
                        container_state
                    }
                } else {
                    Container::new(&path)
                };

                container.properties.name = container
                    .base_path()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();

                let mut dirs = Vec::new();
                let mut files = Vec::new();
                for entry in fs::read_dir(container.base_path()).unwrap() {
                    let entry = entry.unwrap();
                    let entry_path = entry.path();
                    if entry_path.is_file() {
                        files.push(entry_path);
                    } else if entry_path.is_dir() {
                        if entry_path.components().any(|seg| match seg {
                            path::Component::Normal(seg) => seg == app_dir(),
                            _ => false,
                        }) {
                            continue;
                        }

                        dirs.push(entry_path);
                    }
                }

                let container_path = fs::canonicalize(container.base_path()).unwrap();
                let asset_paths = container
                    .assets
                    .iter()
                    .map(|asset| {
                        let asset_path = container.base_path().join(asset.path.as_path());
                        fs::canonicalize(asset_path).unwrap()
                    })
                    .collect::<Vec<_>>();

                if init_assets {
                    for file_path in files {
                        let file_path = fs::canonicalize(file_path).unwrap();
                        if asset_paths.contains(&file_path) {
                            continue;
                        }

                        // ignore hidden files as assets
                        if let Some(file_name) = file_path.file_name() {
                            if let Some(file_name) = file_name.to_str() {
                                if file_name.starts_with(".") {
                                    continue;
                                }
                            }
                        }

                        let rel_path = file_path
                            .strip_prefix(&container_path)
                            .unwrap()
                            .to_path_buf();

                        let asset = Asset::new(rel_path);
                        container.assets.push(asset);
                    }
                }

                if let Err(err) = container.save() {
                    let err = match err {
                        resources::container::error::Save::CreateDir(error) => error,
                        resources::container::error::Save::SaveFiles {
                            properties,
                            assets,
                            settings,
                        } => {
                            if let Some(err) = properties {
                                err
                            } else if let Some(err) = assets {
                                err
                            } else {
                                settings.unwrap()
                            }
                        }
                    };

                    return Err(error::Build::Save(err.kind()));
                }

                if recurse {
                    for dir_path in dirs.into_iter().filter(|path| !ignore.contains(path)) {
                        init_container(dir_path, init_assets, new_ids, recurse, ignore)?;
                    }
                }

                Ok(container.rid().clone())
            }

            // main
            let path = path.as_ref();
            if !path.is_dir() {
                return Err(error::Build::NotADirectory);
            }

            let ignore = self
                .init
                .ignored()
                .iter()
                .map(|pattern| {
                    let pattern = path.join(pattern).to_str().unwrap().to_string();
                    let mut match_options = glob::MatchOptions::new();
                    match_options.case_sensitive = false;

                    glob::glob_with(&pattern, match_options)
                        .unwrap()
                        .map(|path| PathBuf::from(path.unwrap()))
                        .collect::<Vec<_>>()
                })
                .flatten()
                .collect();

            init_container(
                path,
                self.init_assets,
                self.init.new_ids,
                self.init.recursive,
                &ignore,
            )
        }
    }
}

pub mod error {
    use serde::{Deserialize, Serialize};
    use std::io;

    #[derive(Serialize, Deserialize, Debug, derive_more::From)]
    pub enum Build {
        Load,
        Save(#[serde(with = "io_error_serde::ErrorKind")] io::ErrorKind),
        NotADirectory,

        /// The path is already a Syre resource.
        AlreadyResource,
    }
}

#[cfg(test)]
#[path = "./container_test.rs"]
mod container_test;
