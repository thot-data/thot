use rayon::prelude::*;
use std::{assert_matches::assert_matches, fs, io, path::PathBuf};
use syre_core::{project::ScriptLang, types::ResourceId};
use syre_desktop_lib::{self as lib};
use syre_local as local;
use syre_local_database as db;

#[tauri::command]
pub async fn project_add_analyses(
    db: tauri::State<'_, db::Client>,
    project: ResourceId,
    resources: Vec<lib::types::AddFsAnalysisResourceData>,
) -> Result<(), Vec<lib::command::analyses::error::AddAnalyses>> {
    use lib::command::analyses::error::AddAnalyses as Error;
    use syre_local::types::FsResourceAction;

    let (project_path, project) = db.project().get_by_id(project).unwrap().unwrap();
    let analysis_root = project_path.clone().join(
        project
            .properties()
            .unwrap()
            .analysis_root
            .as_ref()
            .unwrap(),
    );

    let mut results = tokio::task::JoinSet::new();
    for resource in resources {
        assert!(resource.path.is_absolute());
        assert_matches!(
            resource.parent.components().next().unwrap(),
            std::path::Component::RootDir
        );

        let to = lib::utils::join_path_absolute(&analysis_root, &resource.parent);
        let to = to.join(resource.path.file_name().unwrap());

        let project_path = project_path.clone();
        let analysis_root = analysis_root.clone();
        results.spawn(async move {
            let Some(ext) = resource.path.extension() else {
                return Err(Error::FsResource {
                    path: resource.path.clone(),
                    error: io::ErrorKind::InvalidFilename.into(),
                });
            };

            let ext = ext.to_str().unwrap();
            if !ScriptLang::supported_extensions().contains(&ext) {
                return Err(Error::FsResource {
                    path: resource.path.clone(),
                    error: io::ErrorKind::InvalidFilename.into(),
                });
            }

            match resource.action {
                FsResourceAction::Copy => {
                    let resource_path = fs::canonicalize(resource.path).unwrap();
                    if resource_path != to {
                        tokio::fs::copy(&resource_path, to).await.map_err(|err| {
                            Error::FsResource {
                                path: resource_path.clone(),
                                error: err.into(),
                            }
                        })?;
                    } else {
                        let mut analyses =
                            local::project::resources::Analyses::load_from(&project_path)
                                .map_err(|err| Error::UpdateAnalyses(err))?;

                        let rel_path = to.strip_prefix(analysis_root).unwrap();
                        if !analyses.values().any(|analysis| match analysis {
                            local::types::AnalysisKind::Script(script) => script.path == rel_path,
                            local::types::AnalysisKind::ExcelTemplate(template) => {
                                template.template.path == rel_path
                            }
                        }) {
                            let script = syre_core::project::Script::from_path(rel_path).unwrap();
                            analyses.insert_script_unique_path(script).unwrap();
                            analyses
                                .save()
                                .map_err(|err| Error::UpdateAnalyses(err.into()))?;
                        }
                    }

                    Ok(())
                }
                FsResourceAction::Move => {
                    fs::rename(&resource.path, to).map_err(|err| Error::FsResource {
                        path: resource.path.clone(),
                        error: err.into(),
                    })
                }
                FsResourceAction::Reference => todo!(),
            }
        });
    }

    let results = results.join_all().await;
    let errors = results
        .into_iter()
        .filter_map(|result| result.err())
        .collect::<Vec<_>>();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Sets all associations within `root`'s subtree with `analysis` to `enable`.
#[tauri::command]
pub async fn analysis_toggle_associations(
    db: tauri::State<'_, db::Client>,
    project: PathBuf,
    root: PathBuf,
    analysis: ResourceId,
    enable: bool,
) -> Result<(), lib::command::analyses::error::ToggleSubtreeAssociations> {
    use lib::command::analyses::error::ToggleSubtreeAssociations;

    let Some(project) = db.project().get(project).unwrap() else {
        return Err(ToggleSubtreeAssociations::ProjectNotFound);
    };
    let db::state::FolderResource::Present(project_data) = project.fs_resource().as_ref() else {
        return Err(ToggleSubtreeAssociations::ProjectNotPresent);
    };
    let project_properties = match project_data.properties() {
        db::state::DataResource::Ok(properties) => properties,
        db::state::DataResource::Err(err) => {
            return Err(ToggleSubtreeAssociations::InvalidProject(err))
        }
    };

    let data_root = project.path().join(&project_properties.data_root);
    let subtree_root = db::common::container_system_path(data_root, root);

    let containers = local::common::ignore::WalkBuilder::new(subtree_root)
        .build()
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .collect::<Vec<_>>();

    let errors = containers
        .into_par_iter()
        .filter_map(|entry| {
            let mut container =
                match local::loader::container::Loader::load_from_only_properties(entry.path())
                    .map_err(|err| (entry.path().to_path_buf(), err))
                {
                    Ok(container) => container,
                    Err(err) => return Some(err),
                };

            let Some(association) = container
                .analyses
                .iter_mut()
                .find(|association| association.analysis() == &analysis)
            else {
                return None;
            };

            if association.autorun != enable {
                association.autorun = enable;
                container
                    .save(entry.path())
                    .map_err(|err| (entry.path().to_path_buf(), err.into()))
                    .err()
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ToggleSubtreeAssociations::Container(errors))
    }
}
