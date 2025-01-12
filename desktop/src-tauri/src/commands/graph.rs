use crate::fs_action;
use std::{
    io,
    path::{Path, PathBuf},
};
use syre_core::{self as core, types::ResourceId};
use syre_desktop_lib as lib;
use syre_local as local;
use syre_local_database::{self as db, common::is_root_path};

#[tauri::command]
pub fn create_child_container(
    db: tauri::State<db::Client>,
    project: ResourceId,
    path: PathBuf,
) -> Result<ResourceId, local::project::container::error::Build> {
    assert!(is_root_path(&path));
    let (project_path, project_state) = db.project().get_by_id(project).unwrap().unwrap();
    let db::state::DataResource::Ok(properties) = project_state.properties() else {
        panic!("invalid state");
    };

    let container_path =
        db::common::container_system_path(project_path.join(&properties.data_root), path);
    local::project::container::new(container_path).map_err(|err| match err {
        local::project::container::error::Build::Load
        | local::project::container::error::Build::NotADirectory => {
            unreachable!("should not occure when creating a new container");
        }
        local::project::container::error::Build::Save(_)
        | local::project::container::error::Build::AlreadyResource => err,
    })
}

// TODO: Clean up return type.
/// Adds file system resources to the project.
///
/// # Returns
/// `Vec` of `Result`s corresponding to each resource.
#[tauri::command]
pub async fn add_file_system_resources(
    state: tauri::State<'_, crate::State>,
    db: tauri::State<'_, db::Client>,
    resources: Vec<lib::types::AddFsGraphResourceData>,
) -> Result<(), Vec<(PathBuf, lib::command::error::IoErrorKind)>> {
    let mut projects = resources
        .iter()
        .map(|resource| &resource.project)
        .collect::<Vec<_>>();
    projects.sort();
    projects.dedup();

    let project_paths = projects
        .into_iter()
        .cloned()
        .map(|project| {
            let (path, state) = db.project().get_by_id(project.clone()).unwrap().unwrap();
            let db::state::DataResource::Ok(properties) = state.properties() else {
                todo!();
            };

            (project, path, properties.data_root.clone())
        })
        .collect::<Vec<_>>();

    let user = state.user();
    let user = user.lock().unwrap().as_ref().map(|user| user.rid().clone());

    let mut results = tokio::task::JoinSet::new();
    for resource in resources {
        let (project_path, data_root) = project_paths
            .iter()
            .find_map(|(project_rid, project_path, data_root)| {
                if *project_rid == resource.project {
                    Some((project_path.clone(), data_root.clone()))
                } else {
                    None
                }
            })
            .unwrap();

        let actions = state.actions().clone();
        results.spawn({
            let user = user.clone();
            async move {
                add_file_system_resource(resource, project_path, data_root, user, actions).await
            }
        });
    }
    let results = results.join_all().await;

    let errors = results
        .into_iter()
        .filter_map(|result| result.err())
        .flat_map(|errors| {
            errors
                .into_iter()
                .map(|(path, err)| (path, err.into()))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

async fn add_file_system_resource(
    resource: lib::types::AddFsGraphResourceData,
    project: impl AsRef<Path>,
    data_root: impl AsRef<Path>,
    user: Option<ResourceId>,
    actions: crate::state::Slice<Vec<fs_action::Action>>,
) -> Result<(), Vec<(PathBuf, io::ErrorKind)>> {
    use syre_local::types::FsResourceAction;

    let project_path = project.as_ref();
    let data_root = data_root.as_ref();
    let to_name = resource.path.file_name().unwrap();
    let data_root_path = project_path.join(data_root);
    let parent_path = lib::utils::join_path_absolute(data_root_path, &resource.parent);
    let to_path = parent_path.join(to_name);
    match resource.action {
        FsResourceAction::Move => {
            if to_path == resource.path {
                return Err(vec![(resource.path.clone(), io::ErrorKind::AlreadyExists)]);
            }

            tokio::fs::rename(&resource.path, &resource.parent)
                .await
                .map_err(|err| vec![(resource.path.clone(), err.kind())])
        }
        FsResourceAction::Copy => {
            if to_path == resource.path {
                return Err(vec![(resource.path.clone(), io::ErrorKind::AlreadyExists)]);
            }

            let to_path = local::common::unique_file_name(&to_path)
                .map_err(|err| vec![(resource.path.clone(), err)])?;
            if resource.path.is_file() {
                let result = tokio::fs::copy(&resource.path, &to_path)
                    .await
                    .map(|_| ())
                    .map_err(|err| vec![(resource.path.clone(), err.kind())]);

                // TODO: What if already a resource and current creator differs from original?
                // TODO: If file is already a resource, copy info.
                if result.is_ok() {
                    let project_settings = crate::settings::project::Desktop::load(project_path);
                    let creator = user.map(|user| {
                        core::types::Creator::User(Some(core::types::UserId::Id(user)))
                    });
                    let asset_drag_drop_kind = project_settings
                        .as_ref()
                        .ok()
                        .map(|settings| settings.asset_drag_drop_kind.clone())
                        .flatten();

                    if creator.is_some() || asset_drag_drop_kind.is_some() {
                        let action = fs_action::Action::new(
                            Box::new({
                                let project_path = project_path.to_path_buf();
                                let container_path = resource.parent.clone();
                                let to_name = to_name.to_os_string();
                                move |event| {
                                    let db::event::UpdateKind::Project {
                                        path: event_project_path,
                                        update:
                                            db::event::Project::Container {
                                                path: event_container_path,
                                                update:
                                                    db::event::Container::Assets(
                                                        db::event::DataResource::Modified(assets),
                                                    ),
                                            },
                                        ..
                                    } = event.kind()
                                    else {
                                        return false;
                                    };

                                    if *event_project_path != project_path {
                                        return false;
                                    }

                                    if *event_container_path != container_path {
                                        return false;
                                    }

                                    assets.iter().any(|asset| asset.path == to_name)
                                }
                            }),
                            Box::new({
                                let to_name = to_name.to_os_string();
                                move |event| {
                                    let db::event::UpdateKind::Project {
                                        path: event_project_path,
                                        update:
                                            db::event::Project::Container {
                                                path: event_container_path,
                                                update:
                                                    db::event::Container::Assets(
                                                        db::event::DataResource::Modified(assets),
                                                    ),
                                            },
                                        ..
                                    } = event.kind()
                                    else {
                                        panic!("invalid event kind");
                                    };

                                    let Ok(mut assets) =
                                        local::loader::container::Loader::load_from_only_assets(
                                            &parent_path,
                                        )
                                    else {
                                        tracing::trace!("assets file could not be loaded");
                                        return;
                                    };

                                    let Some(asset) =
                                        assets.iter_mut().find(|asset| asset.path == to_name)
                                    else {
                                        tracing::trace!("asset not found");
                                        return;
                                    };

                                    if let Some(creator) = creator {
                                        asset.properties.creator = creator;
                                    }
                                    if let Some(asset_drag_drop_kind) = asset_drag_drop_kind {
                                        let _ = asset.properties.kind.insert(asset_drag_drop_kind);
                                    }

                                    if let Err(err) = assets.save(&parent_path) {
                                        tracing::trace!("could not save assets: {err:?}");
                                    }
                                }
                            }),
                        );
                        let mut actions = actions.lock().unwrap();
                        actions.push(action);
                    }
                }
                result
            } else if resource.path.is_dir() {
                copy_dir(&resource.path, &to_path).await
            } else {
                todo!();
            }
        }
        FsResourceAction::Reference => todo!(),
    }
}

#[tauri::command]
pub async fn container_duplicate(
    db: tauri::State<'_, db::Client>,
    project: ResourceId,
    container: PathBuf,
) -> Result<(), lib::command::graph::error::duplicate::Error> {
    assert!(is_root_path(&container));
    let (project_path, project_state) = db.project().get_by_id(project).unwrap().unwrap();
    let db::state::DataResource::Ok(properties) = project_state.properties() else {
        panic!("invalid state");
    };

    let root_path =
        db::common::container_system_path(project_path.join(&properties.data_root), &container);

    duplicate::duplicate_subgraph(root_path)
        .await
        .map(|_path| ())
        .map_err(|err| err.into())
}

#[tauri::command]
pub fn container_trash(
    db: tauri::State<db::Client>,
    project: ResourceId,
    container: PathBuf,
) -> Result<(), lib::command::error::IoErrorKind> {
    assert!(is_root_path(&container));
    let (project_path, project_state) = db.project().get_by_id(project).unwrap().unwrap();
    let db::state::DataResource::Ok(properties) = project_state.properties() else {
        panic!("invalid state");
    };

    let container_path =
        db::common::container_system_path(project_path.join(&properties.data_root), container);

    trash::delete(&container_path).map_err(|err| match err {
        _ => todo!("{container_path:?}: {err:?}"),
    })
}

/// # Returns
/// `Err` if any path fails to be copied.
pub async fn copy_dir(
    src: impl AsRef<Path>,
    dst: impl AsRef<Path>,
) -> Result<(), Vec<(PathBuf, io::ErrorKind)>> {
    let src: &Path = src.as_ref();
    let dst: &Path = dst.as_ref();
    let mut results = tokio::task::JoinSet::new();
    for entry in walkdir::WalkDir::new(src)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let rel_path = entry.path().strip_prefix(src).unwrap();
        let dst = dst.join(rel_path);

        results.spawn(async move {
            if entry.file_type().is_file() {
                match tokio::fs::copy(entry.path(), dst).await {
                    Ok(_) => Ok(()),
                    Err(err) => Err((entry.path().to_path_buf(), err.kind())),
                }
            } else if entry.file_type().is_dir() {
                match tokio::fs::create_dir(dst).await {
                    Ok(_) => Ok(()),
                    Err(err) => Err((entry.path().to_path_buf(), err.kind())),
                }
            } else {
                todo!();
            }
        });
    }
    let results = results.join_all().await;

    let errors = results
        .into_iter()
        .filter_map(|result| match result {
            Ok(_) => None,
            Err(err) => Some(err),
        })
        .collect::<Vec<_>>();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

mod duplicate {
    use std::{
        fs, io,
        path::{Path, PathBuf},
    };
    use syre_local as local;

    /// Duplicate a subgraph.
    ///
    /// # Returns
    /// Path to the duplicated root.
    pub async fn duplicate_subgraph(root: impl AsRef<Path>) -> Result<PathBuf, error::Error> {
        /// Name for directory within temporary directory, to place duplicated containers.
        const ROOT_DIR_NAME: &str = "data";
        /// How long to wait between moving duplicated tree folder.
        const MOVE_DUPLICATED_TREE_DELAY_MS: u64 = 50;
        /// Maximum number of attempts to move duplicated tree folder.
        const MOVE_DUPLICATED_TREE_ATTEMPTS: usize = 20;

        let dup_root =
            local::common::unique_file_name(&root).map_err(|err| error::Error::Filename(err))?;

        let parent = dup_root.parent().unwrap();
        let tmp_dir = local::common::fs::TempDir::hidden_in(parent)
            .map_err(|err| error::Error::Tmp(err.kind()))?;
        let tmp_root = tmp_dir.path().join(ROOT_DIR_NAME);
        let dir_walker = local::common::ignore::WalkBuilder::new(&root)
            .filter_entry(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
            .build();

        let containers = dir_walker
            .into_iter()
            .filter_map(|entry| entry.ok())
            .map(|entry| {
                let rel_path = entry.path().strip_prefix(&root).unwrap();
                let path = tmp_root.join(rel_path);

                let mut container = local::project::resources::Container::new(path);
                let (properties, analyses, settings) =
                    match local::loader::container::Loader::load(entry.path()) {
                        Ok(container) => (
                            container.properties.clone(),
                            container.analyses.clone(),
                            container.settings,
                        ),
                        Err(local::loader::container::State {
                            properties,
                            settings,
                            ..
                        }) if properties.is_ok() && settings.is_ok() => {
                            let properties = properties.unwrap();
                            (
                                properties.properties,
                                properties.analyses,
                                settings.unwrap(),
                            )
                        }
                        Err(state) => {
                            return Err((
                                entry.path().to_path_buf(),
                                error::Duplicate::Load(state),
                            ));
                        }
                    };

                container.properties = properties;
                container.analyses = analyses;
                container.settings = settings;

                if rel_path.as_os_str() == "" {
                    container.properties.name =
                        dup_root.file_name().unwrap().to_string_lossy().to_string();
                }

                container.save().map_err(|err| {
                    (
                        container.base_path().to_path_buf(),
                        error::Duplicate::Save(err),
                    )
                })
            })
            .collect::<Vec<_>>();

        let errors = containers
            .into_iter()
            .filter_map(|container| container.err())
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            return Err(error::Error::Duplicate(errors));
        }

        // NB: Files may not be fully created at this point.
        // When trying to rename a folder at this time a `PermissionDenied` error is raised.
        // This allows a delay for full creation before moving.
        for attempt in 1..=MOVE_DUPLICATED_TREE_ATTEMPTS {
            match fs::rename(&tmp_root, &dup_root) {
                Ok(_) => return Ok(dup_root),
                Err(err)
                    if matches!(err.kind(), io::ErrorKind::PermissionDenied)
                        && attempt < MOVE_DUPLICATED_TREE_ATTEMPTS =>
                {
                    continue
                }
                Err(err) => return Err(error::Error::Move(err.kind())),
            }

            tokio::time::sleep(std::time::Duration::from_millis(
                MOVE_DUPLICATED_TREE_DELAY_MS,
            ))
            .await;
        }
        unreachable!("fn terminated in loop above");
    }

    pub mod error {
        use std::{io, path::PathBuf};
        use syre_desktop_lib as lib;
        use syre_local as local;

        #[derive(Debug)]
        pub enum Error {
            /// Creating a unique file name for the duplicate root failed.
            Filename(io::ErrorKind),

            /// Creating a temporary directory in which to duplicate the tree failed.
            Tmp(io::ErrorKind),

            /// Duplicating the tree failed.
            Duplicate(Vec<(PathBuf, Duplicate)>),

            /// Relocating the duplicated tree to its final destination failed.
            Move(io::ErrorKind),
        }

        #[derive(Debug)]
        pub enum Duplicate {
            /// Loading the parent failed.
            Load(local::loader::container::State),

            /// Saving the child failed.
            Save(local::project::resources::container::error::Save),
        }

        impl Into<lib::command::graph::error::duplicate::Error> for Error {
            fn into(self) -> lib::command::graph::error::duplicate::Error {
                use lib::command::graph::error;

                match self {
                    Self::Filename(err) => error::duplicate::Error::Filename(err.into()),
                    Self::Tmp(err) => error::duplicate::Error::Tmp(err.into()),
                    Self::Move(err) => error::duplicate::Error::Move(err.into()),
                    Self::Duplicate(errors) => {
                        let errors = errors
                            .into_iter()
                            .map(|(path, err)| {
                                let err = match err {
                                    Duplicate::Load(local::loader::container::State {
                                        properties,
                                        settings,
                                        ..
                                    }) => error::duplicate::Duplicate::Load {
                                        properties: properties.err(),
                                        settings: settings.err(),
                                    },
                                    Duplicate::Save(err) => { match err {
                                        local::project::resources::container::error::Save::CreateDir(error) => error::duplicate::Duplicate::Save(error::duplicate::SaveContainer::CreateDir(error.into())),
                                        local::project::resources::container::error::Save::SaveFiles{properties, assets, settings} => error::duplicate::Duplicate::Save(error::duplicate::SaveContainer::SaveFiles{
                                            properties: properties.map(|err| err.into()) ,
                                            assets: assets.map(|err| err.into()),
                                            settings: settings.map(|err| err.into())
                                        }),
                                    }
                                    }
                                };

                                (path, err)
                            })
                            .collect();

                        error::duplicate::Error::Duplicate(errors)
                    }
                }
            }
        }
    }
}
