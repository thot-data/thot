//! Functionality and resources related to projects.
use super::resources::{Analyses, Project};
use crate::{
    common,
    system::{collections::ProjectManifest, project_manifest},
    Result,
};
use std::{
    fs, io,
    path::{Path, PathBuf},
    result::Result as StdResult,
};
use syre_core::types::ResourceId;

pub use duplicate::duplicate;

// ************
// *** Init ***
// ************

/// Initialize a new Syre project.
/// If the path is already initialized as a Syre resource -- i.e. has an app folder -- nothing is
/// done.
///
/// # Steps
/// 1. Create app folder to store data.
/// 2. Create [`Project`] for project info.
/// 3. Create `ProjectSettings` for project settings.
/// 4. Create `Script`s registry.
/// 5. Add [`Project`] to collections registry.
pub fn init(path: impl AsRef<Path>) -> StdResult<ResourceId, error::Init> {
    let path = path.as_ref();
    if !is_valid_project_path(&path).map_err(|err| error::Init::ProjectManifest(err))? {
        return Err(error::Init::InvalidRootPath);
    }

    // create directory
    let syre_dir = common::app_dir_of(path);
    fs::create_dir(&syre_dir).map_err(|err| error::Init::CreateAppDir(err))?;

    // create app files
    let project = Project::new(path).map_err(|err| error::Init::Properties(err.into()))?;
    project.save().map_err(|err| error::Init::Properties(err))?;

    let analyses = Analyses::new(path);
    analyses.save().map_err(|err| error::Init::Analyses(err))?;

    project_manifest::register_project(project.base_path())
        .map_err(|err| error::Init::ProjectManifest(err))?;

    Ok(project.rid().clone().into())
}

/// Creates a new Syre project.
///
/// # Errors
/// + If the folder already exists.
///
/// # See also
/// + [`init`]
pub fn new(root: &Path) -> StdResult<ResourceId, error::New> {
    if root.exists() {
        return Err(io::Error::new(io::ErrorKind::IsADirectory, "folder already exists").into());
    }

    fs::create_dir_all(root).map_err(|err| error::New::CreateRoot(err))?;
    Ok(init(root)?)
}

/// Move project to a new location.
pub fn mv(from: impl Into<PathBuf>, to: impl Into<PathBuf>) -> Result {
    let from = from.into();
    let mut projects = ProjectManifest::load()?;
    if !projects.contains(&from) {
        return Err(crate::error::Project::PathNotAProjectRoot(from).into());
    }

    // move folder
    let to = to.into();
    if let Err(err) = fs::rename(&from, &to) {
        return Err(err.into());
    }

    projects.remove(&from);
    projects.push(to);
    projects.save()?;
    Ok(())
}

/// Checks if the given path is within a registered project root,
/// by comparing it to registered project roots.
/// This does not check the state of the project it may be in.
///
/// # Returns
/// `Some` with the registered project's path if the path is contained within it.
/// `None` if the path is not within a registered project root.
///
/// # Errors
/// + If the project manifest can not be loaded.
pub fn path_in_registered_project(
    path: impl AsRef<Path>,
) -> StdResult<Option<PathBuf>, crate::error::IoSerde> {
    let project_manifest = ProjectManifest::load()?;
    let project = project_manifest
        .iter()
        .find(|project| path.as_ref().strip_prefix(project).is_ok())
        .map(|project| project.clone());

    Ok(project)
}

/// Checks if the given path contains a registered project root,
/// by comparing it to registered project roots.
/// This does not check the state of the project it may contain.
///
/// # Errors
/// + If the project manifest can not be loaded.
pub fn contains_registered_projects(
    path: impl AsRef<Path>,
) -> StdResult<Vec<PathBuf>, crate::error::IoSerde> {
    let project_manifest = ProjectManifest::load()?;
    let project = project_manifest
        .iter()
        .filter(|project| project.strip_prefix(path.as_ref()).is_ok())
        .map(|project| project.clone())
        .collect();

    Ok(project)
}

/// Checks if the given path is a valid project root,
/// by comparing it to registered project roots.
/// This does not check the state of any projects.
///
/// # Returns
/// `false` if the given path contains or is contained within any
/// registered project root paths, otherwise `true`.
///
/// # Errors
/// + If the project manifest can not be loaded.
pub fn is_valid_project_path(path: impl AsRef<Path>) -> StdResult<bool, crate::error::IoSerde> {
    let project_manifest = ProjectManifest::load()?;
    let valid = !project_manifest.iter().any(|project| {
        project.strip_prefix(path.as_ref()).is_ok() || path.as_ref().strip_prefix(project).is_ok()
    });

    Ok(valid)
}

/// Returns whether the given path is part of a Syre project.
///
/// # Returns
/// `true`` if the path has a <APP_DIR> folder in it.
///
/// # Note
/// + Only works with `Container`s and `Project`s, not `Asset`s.
pub fn path_is_resource(path: &Path) -> bool {
    let path = common::app_dir_of(path);
    path.exists()
}

/// Checks if the path has an app directory with a project's properties file.
/// i.e. The path has a <APP_DIR>/<PROJECT_FILE>.
/// Does not check if the project is regsitered.
///
/// # Returns
/// Whether the given path is a project root.
pub fn path_is_project_root(path: impl AsRef<Path>) -> bool {
    let path = common::project_file_of(path);
    path.exists()
}

/// Traverses up the directory tree to find a project root.
/// Does not check if the project is registered.
///
/// # Returns
/// Path to the project root.
///
/// # See also
/// + [`project_resource_root_path`]
pub fn project_root_path(path: impl AsRef<Path>) -> Option<PathBuf> {
    let mut path = path.as_ref().join("tmp"); // false join to pop off in loop
    while path.pop() {
        if path_is_project_root(&path) {
            return Some(path);
        }
    }

    None
}

// /// # Returns
// /// Path to the project root for a Syre resource.
// ///
// /// # See also
// /// + [`project_root_path`]
// pub fn project_resource_root_path(path: impl AsRef<Path>) -> StdResult<PathBuf, error::ProjectResource> {
//     let mut path = path.as_ref().join("tmp"); // false join to pop off in loop
//     while path.pop() {
//         let prj_path = common::project_file_of(&path);
//         if !prj_path.exists() {
//             // folder is not root
//             continue;
//         }

//         let file = fs::File::open(prj_path)?;
//         let reader = io::BufReader::new(file);
//         let prj: CoreProject = match serde_json::from_reader(reader) {
//             Ok(prj) => prj,
//             Err(err) => return Err(err.into()),
//         };

//         if prj.meta_level == 0 {
//             return Ok(fs::canonicalize(path)?);
//         }
//     }

//     Err(error::ProjectResource::NotInProject)
// }

// /// # Returns
// /// + [`ResourceId`] of the containing [`Project`] if it exists.
// /// + `None` if the path is not inside a `Project``.
// pub fn project_id(path: impl AsRef<Path>) -> StdResult<Option<ResourceId>, error::ProjectResource> {
//     let root = match project_resource_root_path(path.as_ref()) {
//         Ok(root) => root,
//         Err(Error::Project(crate::error::Project::PathNotInProject(_))) => return Ok(None),
//         Err(err) => return Err(err),
//     };

//     let project = Project::load_from(root)?;
//     Ok(Some(project.rid().clone()))
// }

pub mod converter {
    use super::super::{container, resources};
    use crate::{common, loader::container::Loader as ContainerLoader, system::config};
    use std::{
        collections::HashMap,
        fs, io,
        path::{Component, Path, PathBuf},
    };
    use syre_core::{
        project::{AnalysisAssociation, Script, ScriptLang},
        types::{ResourceId, UserId, UserPermissions},
    };

    pub struct Converter {
        data_root: PathBuf,
        analysis_root: Option<PathBuf>,
    }

    impl Converter {
        /// Creates a new converter.
        ///
        /// # Notes
        /// + `data_root` defaults to `data`.
        /// + `analysis_root` defaults to `analysis`.
        pub fn new() -> Self {
            Self {
                data_root: PathBuf::from("data"),
                analysis_root: Some(PathBuf::from("analysis")),
            }
        }

        pub fn set_data_root(&mut self, path: impl Into<PathBuf>) -> io::Result<()> {
            let path = path.into();
            Self::check_path(&path)?;
            if let Some(analysis_root) = self.analysis_root.as_ref() {
                if path.starts_with(analysis_root) || analysis_root.starts_with(&path) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidFilename,
                        "`data_root` and `analysis_root` must be distinct",
                    ));
                }
            }

            self.data_root = path;
            Ok(())
        }

        /// Indicates analysis scripts should be moved into the given folder and processed.
        pub fn set_analysis_root(&mut self, path: impl Into<PathBuf>) -> io::Result<()> {
            let path = path.into();
            Self::check_path(&path)?;
            if path.starts_with(&self.data_root) || self.data_root.starts_with(&path) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidFilename,
                    "`data_root` and `analysis_root` must be distinct",
                ));
            }

            self.analysis_root = Some(path);
            Ok(())
        }

        /// Do not initialize analysis scripts.
        pub fn without_scripts(&mut self) {
            self.analysis_root = None;
        }

        /// Converts an existing folder of data and scripts into a project.
        ///
        /// # Returns
        /// Project's id.
        pub fn convert(&self, root: impl AsRef<Path>) -> Result<ResourceId, error::Convert> {
            let Ok(root) = fs::canonicalize(root.as_ref()) else {
                return Err(error::Convert::DoesNotExist);
            };

            let pid = super::init(root.as_path())?;
            let mut project = resources::Project::load_from(root.as_path()).unwrap();
            project.data_root = self.data_root.clone();
            project.analysis_root = self.analysis_root.clone();

            if let Ok(config) = config::Config::load() {
                let user = config.user.clone().map(|user| UserId::Id(user));
                project.settings_mut().creator = user;

                if let Some(user) = config.user.as_ref() {
                    project
                        .settings_mut()
                        .permissions
                        .insert(user.clone(), UserPermissions::all());
                }
            }
            project.save().unwrap();

            // create data and analysis roots
            // move contents into data root
            let tmp_dir = common::unique_file_name(root.join("__tmp_data__"))
                .map_err(|err| io::Error::new(err, "could not create unique file name"))?;
            fs::create_dir(&tmp_dir)?;
            for entry in fs::read_dir(&root)? {
                let entry = entry?;
                let path = entry.path();
                if path == tmp_dir || path == common::app_dir_of(&root) {
                    continue;
                }

                let rel_path = path.strip_prefix(&root).unwrap();
                fs::rename(entry.path(), tmp_dir.join(rel_path))?;
            }

            let data_root = root.join(&self.data_root);
            fs::rename(tmp_dir, &data_root)?;

            if let Some(analysis_root) = self.analysis_root.as_ref() {
                // performed before intializing graph so scripts don't get registered as assets
                let analysis_root = root.join(analysis_root);
                fs::create_dir_all(&analysis_root)?;

                // move scripts
                #[cfg(target_os = "windows")]
                let data_root = common::strip_windows_unc(&data_root);

                let mut ext_pattern = data_root.join("**").join("*");
                let mut match_options = glob::MatchOptions::new();
                match_options.case_sensitive = false;

                let mut script_paths = Vec::new();
                for lang_ext in ScriptLang::supported_extensions() {
                    ext_pattern.set_extension(lang_ext);

                    for entry in
                        glob::glob_with(ext_pattern.to_str().unwrap(), match_options).unwrap()
                    {
                        let script_path = match entry {
                            Ok(path) => path,
                            Err(err) => return Err(err.into_error().into()),
                        };

                        let rel_path = script_path.strip_prefix(&data_root).unwrap().to_path_buf();
                        let to = analysis_root.join(&rel_path);
                        fs::create_dir_all(to.parent().unwrap())?;
                        fs::rename(script_path, to)?;
                        script_paths.push(rel_path);
                    }
                }

                // initialize scripts
                let mut scripts = resources::Analyses::load_from(&root)
                    .map_err(|err| error::Convert::Analyses(err))?;
                for script_path in script_paths {
                    let Ok(script) = Script::from_path(script_path) else {
                        continue;
                    };

                    scripts.insert_script_unique_path(script).unwrap();
                }

                scripts.save()?;
            }

            // initialize container graph
            let mut builder = container::builder::InitOptions::init();
            builder.recurse(true);
            builder.with_new_ids(true);
            builder.with_assets();
            builder.build(&data_root)?;

            if self.analysis_root.is_some() {
                // assign scripts
                let analyses = resources::Analyses::load_from(&root)?;
                let mut container_scripts = HashMap::new();
                for script in analyses.scripts() {
                    let entry = container_scripts
                        .entry(script.path.parent().unwrap())
                        .or_insert(Vec::new());

                    entry.push(script.rid().clone());
                }

                for (container, scripts) in container_scripts {
                    let container = data_root.join(container);
                    let Ok(mut container) = ContainerLoader::load(container) else {
                        continue;
                    };

                    for script in scripts {
                        container.set_analysis_association(AnalysisAssociation::new(script));
                    }

                    container.save().map_err(|err| match err {
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
                    })?;
                }
            }

            Ok(pid)
        }

        fn check_path(path: impl AsRef<Path>) -> io::Result<()> {
            let path = path.as_ref();
            if !path.is_relative() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidFilename,
                    "path must be relative",
                ));
            }

            if path.components().any(|comp| comp == Component::ParentDir) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidFilename,
                    "path may not contain parent directory references (i.e. `..`)",
                ));
            }

            Ok(())
        }
    }

    pub mod error {
        use crate::{error::IoSerde, project::container};
        use std::io;

        #[derive(Debug, derive_more::From)]
        pub enum Convert {
            /// Folder does not exist.
            DoesNotExist,

            /// Could not initialize the project.
            Init(super::super::error::Init),

            /// An issue occurred when manipulating the files system.
            Fs(io::Error),

            /// An issue occurred when building the container tree.
            Build(container::error::Build),

            /// An issue occurred when manipulating analyses.
            Analyses(IoSerde),
        }
    }
}

pub mod duplicate {
    use crate::{common, loader, project::resources, types};
    use std::{
        collections::HashMap,
        fs,
        path::{Path, PathBuf},
        sync::Arc,
    };
    use syre_core::{self as core, graph::ResourceTree, types::ResourceId};

    pub use error::Error;

    /// Duplicate a project.
    ///
    /// # Arguments
    /// + `src`: Base path of the source project.
    /// + `dst`: Location of duplicated project. Must be a non-existant path.
    ///
    /// # Notes
    /// + Dupicated project will have name of source project with ` - Copy` appended.
    /// + Analyses and graph are duplicated without any assets.
    pub fn duplicate(src: impl AsRef<Path>, dst: impl Into<PathBuf>) -> Result<(), Error> {
        let src = src.as_ref();
        let dst: PathBuf = dst.into();
        if !src.exists() {
            return Err(Error::SourceDoesNotExist);
        }

        let project_src = resources::Project::load_from(src).map_err(|err| {
            let resources::project::LoadError {
                properties,
                settings,
            } = err;
            if let Err(err) = properties {
                Error::InvalidSourceProperties(err)
            } else if let Err(err) = settings {
                Error::InvalidSourceSettings(err)
            } else {
                unreachable!();
            }
        })?;

        if dst.exists() {
            return Err(Error::DestinationAlreadyExists);
        }
        let mut tmp = dst.clone();
        tmp.set_file_name(format!(
            ".{}.tmp",
            tmp.file_name().unwrap().to_string_lossy()
        ));
        fs::create_dir_all(&tmp).map_err(|err| Error::CreateDestinationFolder(err.kind()))?;

        #[cfg(target_os = "windows")]
        let hidden = common::fs::hide_folder(&tmp).is_ok();

        let mut project = resources::project::Builder::new(tmp);
        let mut properties = project_src.properties().clone();
        let mut project_name = project_src.properties().name.clone();
        project_name.push_str(" - Copy");
        properties.name = project_name;
        project.with_properties(properties);
        let project = project.build().unwrap();

        project
            .save()
            .map_err(|err| Error::SaveProject(err.kind()))?;

        let analyses_src = resources::Analyses::load_from(project_src.base_path())
            .map_err(Error::LoadAnalyses)?
            .to_vec();
        let mut analyses_map = Vec::with_capacity(analyses_src.len());
        let analyses = analyses_src
            .into_iter()
            .map(|analysis_src| match analysis_src {
                types::AnalysisKind::Script(script_src) => {
                    let rid_src = script_src.rid().clone();
                    let core::project::Script {
                        path,
                        name,
                        description,
                        env,
                        creator,
                        ..
                    } = script_src;

                    let mut script = core::project::Script::new(path, env);
                    script.name = name;
                    script.description = description;
                    script.creator = creator;

                    analyses_map.push((rid_src, script.rid().clone()));
                    types::AnalysisKind::Script(script)
                }
                types::AnalysisKind::ExcelTemplate(_excel_template) => todo!(),
            })
            .collect::<Vec<_>>();
        let analyses = resources::Analyses::new_with(project.base_path().to_path_buf(), analyses);
        analyses
            .save()
            .map_err(|err| Error::SaveAnalyses(err.kind()))?;

        if let Some(analysis_root) = project.analysis_root_path() {
            common::copy_dir(project_src.analysis_root_path().unwrap(), analysis_root).map_err(
                |errors| {
                    tracing::error!(?errors);
                    let errors = errors
                        .into_iter()
                        .map(|(path, err)| error::File { path, error: err })
                        .collect();
                    Error::DuplicateAnalyses(errors)
                },
            )?;
        }

        let graph_src =
            loader::tree::Loader::load(project_src.data_root_path()).map_err(|err| match err {
                loader::tree::error::Error::Root(_) => panic!("can not access graph root"),
                loader::tree::error::Error::Ignore { .. } => todo!("invalid ignore file"),
                loader::tree::error::Error::State(tree) => {
                    let (nodes, _, _) = tree.to_parts();
                    let errors = nodes
                        .into_iter()
                        .filter_map(|node| {
                            let node = Arc::into_inner(node).unwrap();
                            let container = node.into_inner().unwrap();
                            container.err().map(|container| {
                                let error = container.data().as_ref().err().unwrap();
                                error::File {
                                    path: container.path().clone(),
                                    error: *error,
                                }
                            })
                        })
                        .collect::<Vec<_>>();

                    Error::LoadGraph(errors)
                }
            })?;

        let (nodes_src, edges_src) = graph_src.into_components();
        let mut node_map = Vec::with_capacity(nodes_src.len());
        let data_root_src = project_src.data_root_path();
        let data_root = project.data_root_path();
        let nodes = nodes_src
            .values()
            .map(|node_src| {
                let container_src = node_src.data();
                let container_graph_path = container_src.base_path().to_path_buf();
                let container_graph_path =
                    container_graph_path.strip_prefix(&data_root_src).unwrap();

                let mut container =
                    resources::container::Builder::new(data_root.join(container_graph_path));
                container.with_properties(container_src.properties.clone());
                container.with_analyses(container_src.analyses.clone());
                container.with_settings(container_src.settings.clone());
                let mut container = container.build();
                node_map.push((container_src.rid().clone(), container.rid().clone()));

                let analyses = container
                    .analyses
                    .iter()
                    .map(|assoc_src| {
                        let analysis = analyses_map
                            .iter()
                            .find_map(|(src, dst)| {
                                (assoc_src.analysis() == src).then_some(dst.clone())
                            })
                            .unwrap();

                        let mut assoc = core::project::AnalysisAssociation::new(analysis);
                        assoc.priority = assoc_src.priority;
                        assoc.autorun = assoc_src.autorun;
                        assoc
                    })
                    .collect::<Vec<_>>();

                container.analyses = analyses;
                container
            })
            .collect::<Vec<_>>();

        let edges = edges_src
            .into_iter()
            .map(|(parent_src, children_src)| {
                let parent = node_map
                    .iter()
                    .find_map(|(src, dst)| (*src == parent_src).then_some(dst.clone()))
                    .unwrap();

                let children = children_src
                    .iter()
                    .map(|child_src| {
                        node_map
                            .iter()
                            .find_map(|(src, dst)| (child_src == src).then_some(dst.clone()))
                            .unwrap()
                    })
                    .collect::<indexmap::IndexSet<_>>();

                (parent, children)
            })
            .collect::<HashMap<_, _>>();

        let nodes = nodes
            .into_iter()
            .map(|container| {
                (
                    container.rid().clone(),
                    core::graph::ResourceNode::new(container),
                )
            })
            .collect();
        let graph = core::graph::ResourceTree::from_parts(nodes, edges).unwrap();

        save_graph(&graph).map_err(|(path, err)| {
            let (path, error) = match err {
                resources::container::error::Save::CreateDir(err) => (path, err.kind()),
                resources::container::error::Save::SaveFiles {
                    properties,
                    assets,
                    settings,
                } => {
                    if let Some(err) = properties {
                        (crate::common::container_file_of(path), err.kind())
                    } else if let Some(err) = assets {
                        (crate::common::assets_file_of(path), err.kind())
                    } else if let Some(err) = settings {
                        (crate::common::container_settings_file_of(path), err.kind())
                    } else {
                        unreachable!();
                    }
                }
            };

            Error::SaveGraph(error::File { path, error })
        })?;

        fs::rename(project.base_path(), &dst)
            .map_err(|err| Error::RenameDestination(err.kind()))?;
        #[cfg(target_os = "windows")]
        if hidden {
            common::fs::unhide_folder(&dst).unwrap();
        }

        Ok(())
    }

    fn save_graph(
        graph: &ResourceTree<resources::Container>,
    ) -> Result<(), (PathBuf, resources::container::error::Save)> {
        fn inner(
            root: &ResourceId,
            graph: &ResourceTree<resources::Container>,
        ) -> Result<(), (PathBuf, resources::container::error::Save)> {
            let node = graph.get(root).unwrap();
            node.save()
                .map_err(|err| (node.base_path().to_path_buf(), err))?;

            let children = graph.children(root).unwrap();
            for child in children {
                inner(child, graph)?;
            }

            Ok(())
        }

        inner(graph.root(), graph)
    }

    pub mod error {
        use crate::error::IoSerde;
        use serde::{Deserialize, Serialize};
        use std::{io, path::PathBuf};

        #[derive(Serialize, Deserialize, Debug)]
        pub enum Error {
            SourceDoesNotExist,
            InvalidSourceProperties(IoSerde),
            InvalidSourceSettings(IoSerde),
            DestinationAlreadyExists,
            CreateDestinationFolder(#[serde(with = "io_error_serde::ErrorKind")] io::ErrorKind),
            SaveProject(#[serde(with = "io_error_serde::ErrorKind")] io::ErrorKind),
            LoadAnalyses(IoSerde),
            SaveAnalyses(#[serde(with = "io_error_serde::ErrorKind")] io::ErrorKind),
            DuplicateAnalyses(Vec<File>),
            LoadGraph(Vec<File>),
            SaveGraph(File),
            RenameDestination(#[serde(with = "io_error_serde::ErrorKind")] io::ErrorKind),
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct File {
            pub path: PathBuf,

            #[serde(with = "io_error_serde::ErrorKind")]
            pub error: io::ErrorKind,
        }
    }
}

pub mod error {
    use crate::error::IoSerde;
    use std::io;

    #[derive(Debug, derive_more::From)]
    pub enum New {
        CreateRoot(io::Error),
        Init(Init),
    }

    #[derive(Debug)]
    pub enum Init {
        /// The path is not a valid project root path.
        /// This is likely because it contains other or is contained within another project root path(s).
        InvalidRootPath,

        /// Could not register the project in the project manifest.
        ProjectManifest(IoSerde),
        CreateAppDir(io::Error),
        Properties(io::Error),
        Analyses(io::Error),
    }

    #[derive(Debug)]
    pub enum ProjectResource {
        /// Resource is not in a project.
        NotInProject,

        /// Resource does not exist.
        DoesNotExist,
    }
}

#[cfg(test)]
#[path = "./project_test.rs"]
mod project_test;
