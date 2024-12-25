use crate::{settings, state};
use std::{
    fs, io,
    path::{self, PathBuf},
    sync::Arc,
};
use syre_core::{
    self as core,
    types::{ResourceId, UserId, UserPermissions},
};
use syre_desktop_lib as lib;
use syre_local::{
    self as local,
    file_resource::SystemResource,
    project::{
        project,
        resources::{
            Analyses as LocalAnalyses, Container as LocalContainer, Project as LocalProject,
        },
    },
    types::AnalysisKind,
};
use syre_local_database as db;
use syre_local_runner as runner;

#[tauri::command]
pub fn create_project(
    user: ResourceId,
    path: PathBuf,
) -> Result<(), lib::command::project::error::Initialize> {
    use lib::command::project::error;

    let project_manifest_path = match local::system::collections::ProjectManifest::default_path() {
        Ok(path) => path,
        Err(err) => return Err(error::Initialize::ProjectManifest(err.into())),
    };

    if !project_manifest_path.exists() {
        let project_manifest = match local::system::collections::ProjectManifest::load_or_default()
        {
            Ok(manifest) => manifest,
            Err(err) => return Err(error::Initialize::ProjectManifest(err)),
        };

        if let Err(err) = project_manifest.save() {
            return Err(error::Initialize::ProjectManifest(err.into()));
        }
    }

    project::init(&path).map_err(|err| match err {
        project::error::Init::InvalidRootPath => error::Initialize::InvalidRootPath,
        project::error::Init::ProjectManifest(err) => error::Initialize::ProjectManifest(err),
        project::error::Init::CreateAppDir(err) => {
            error::Initialize::Init(format!("Could not create app directory: {err:?}"))
        }
        project::error::Init::Properties(err) => {
            error::Initialize::Init(format!("Could not update properties: {err:?}"))
        }
        project::error::Init::Analyses(err) => {
            error::Initialize::Init(format!("Could not update analyses: {err:?}"))
        }
    })?;

    // create analysis folder
    let analysis_root = "analysis";
    let mut analysis = path.to_path_buf();
    analysis.push(analysis_root);
    fs::create_dir(&analysis).unwrap();

    let mut project = LocalProject::load_from(path)
        .map_err(|err| error::Initialize::Init(format!("Could not update settings: {err:?}")))?;
    let settings = project.settings_mut();
    settings.creator = Some(UserId::Id(user.clone()));
    settings
        .permissions
        .insert(user.clone(), UserPermissions::all());
    project.analysis_root = Some(PathBuf::from(analysis_root));
    project
        .save()
        .map_err(|err| error::Initialize::Init(format!("Could not update settings: {err:?}")))?;

    let mut root = LocalContainer::new(project.data_root_path());
    root.settings_mut().creator = Some(UserId::Id(user.clone()));
    root.save()
        .map_err(|err| error::Initialize::Init(format!("Could not update settings: {err:?}")))?;

    Ok(())
}

#[tauri::command]
pub fn initialize_project(
    user: ResourceId,
    path: PathBuf,
) -> Result<(), lib::command::project::error::Initialize> {
    use lib::command::project::error;
    use project::converter;

    if !local::project::project::is_valid_project_path(&path)
        .map_err(|err| error::Initialize::ProjectManifest(err))?
    {
        tracing::error!("invalid project root path");
        return Err(error::Initialize::InvalidRootPath);
    }

    let converter = local::project::project::converter::Converter::new();
    converter.convert(&path).map_err(|err| match err {
        converter::error::Convert::DoesNotExist => error::Initialize::InvalidRootPath,
        converter::error::Convert::Init(err) => {
            error::Initialize::Init(format!("Could not initialize the project: {err:?}"))
        }
        converter::error::Convert::Fs(err) => {
            error::Initialize::Init(format!("Could not initialize the project: {err:?}"))
        }
        converter::error::Convert::Build(err) => {
            error::Initialize::Init(format!("Could not convert the file to a project: {err:?}"))
        }
        converter::error::Convert::Analyses(err) => {
            error::Initialize::Init(format!("Could not update analyses: {err:?}"))
        }
    })?;

    local::system::project_manifest::register_project(path)
        .map_err(|err| error::Initialize::ProjectManifest(err))?;

    Ok(())
}

#[tauri::command]
pub fn import_project(
    user: ResourceId,
    path: PathBuf,
) -> Result<(), lib::command::project::error::Import> {
    use lib::command::project::error;

    let mut settings = local::project::resources::Project::load_from_settings_only(&path)
        .map_err(|err| error::Import::Settings(err))?;

    settings
        .permissions
        .entry(user)
        .or_insert(UserPermissions::all());

    settings
        .save(&path)
        .map_err(|err| error::Import::Settings(err.into()))?;

    local::system::project_manifest::register_project(&path)
        .map_err(|err| error::Import::ProjectManifest(err))?;

    Ok(())
}

#[tauri::command]
pub fn deregister_project(project: PathBuf) -> Result<(), local::error::IoSerde> {
    local::system::project_manifest::deregister_project(&project)
}

/// # Returns
/// Tuple of (project path, project data, project graph).
#[tauri::command]
pub fn project_resources(
    db: tauri::State<db::Client>,
    project: ResourceId,
) -> Option<(
    PathBuf,
    db::state::ProjectData,
    db::state::FolderResource<db::state::Graph>,
)> {
    let resources = db.project().resources(project).unwrap();
    assert!(if let Some((_, data, _)) = resources.as_ref() {
        data.properties().is_ok()
    } else {
        true
    });

    resources
}

#[tauri::command]
pub fn project_properties_update(
    db: tauri::State<db::Client>,
    update: core::project::Project,
) -> Result<(), local::error::IoSerde> {
    let path = db.project().path(update.rid().clone()).unwrap().unwrap();
    let mut properties = local::project::resources::Project::load_from_properties_only(&path)?;
    assert_eq!(properties.rid(), update.rid());
    if properties == update {
        return Ok(());
    }

    let core::project::Project {
        name,
        description,
        data_root,
        analysis_root,
        meta_level,
        ..
    } = update;

    properties.name = name;
    properties.description = description;
    properties.data_root = data_root;
    properties.analysis_root = analysis_root;
    properties.meta_level = meta_level;

    local::project::resources::Project::save_properties_only(&path, &properties)
        .map_err(|err| err.kind())?;

    Ok(())
}

/// # Arguments
/// + `path`: Relative path from the analysis root.
#[tauri::command]
pub fn project_analysis_remove(
    db: tauri::State<db::Client>,
    project: ResourceId,
    path: PathBuf,
) -> Result<(), lib::command::project::error::AnalysesUpdate> {
    use lib::command::project::error::AnalysesUpdate;

    let (project_path, project) = db.project().get_by_id(project).unwrap().unwrap();
    let mut analyses = match LocalAnalyses::load_from(&project_path) {
        Ok(analyses) => analyses,
        Err(err) => return Err(AnalysesUpdate::AnalysesFile(err)),
    };

    analyses.retain(|_, analysis| match analysis {
        AnalysisKind::Script(script) => script.path != path,
        AnalysisKind::ExcelTemplate(template) => template.template.path != path,
    });

    if let Err(err) = analyses.save() {
        return Err(AnalysesUpdate::AnalysesFile(err.kind().into()));
    }

    if let db::state::DataResource::Ok(properties) = project.properties() {
        let path = project_path
            .join(properties.analysis_root.as_ref().unwrap())
            .join(path);

        if let Err(err) = trash::delete(&path) {
            let err = match err {
                trash::Error::TargetedRoot => io::ErrorKind::InvalidFilename,
                trash::Error::CouldNotAccess { .. } => io::ErrorKind::PermissionDenied,
                trash::Error::CanonicalizePath { .. } => io::ErrorKind::NotFound,
                _ => {
                    tracing::error!(?err);
                    todo!();
                }
            };

            return Err(AnalysesUpdate::RemoveFile(err));
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn trigger_analysis(
    db: tauri::State<'_, db::Client>,
    state: tauri::State<'_, crate::State>,
    analyzer_action: tauri::State<'_, state::Slice<Option<state::AnalyzerAction>>>,
    rx: tauri::ipc::Channel<lib::event::analysis::Update>,
    project: ResourceId,
    root: PathBuf,
    max_tasks: Option<usize>,
) -> Result<(), lib::command::project::error::TriggerAnalysis> {
    use crate::state;
    use lib::command::project::error;
    const PROGRESS_DELAY_MS: u64 = 100;

    let (project_path, project_data, graph) =
        db.project().resources(project.clone()).unwrap().unwrap();
    let db::state::FolderResource::Present(graph) = graph else {
        return Err(error::TriggerAnalysis::GraphAbsent);
    };

    let graph = graph_state_to_runner_tree(graph)
        .map_err(|err| Into::<error::TriggerAnalysis>::into(err))?;
    let mut root_path = root.components();
    assert_eq!(root_path.next().unwrap(), path::Component::RootDir);
    let mut root = graph.root();
    while let Some(component) = root_path.next() {
        let path::Component::Normal(name) = component else {
            panic!("invalid path");
        };
        let name = name.to_str().unwrap();

        let children = graph.children(root).unwrap();
        root = children
            .iter()
            .find(|child| child.properties.name == name)
            .unwrap();
    }
    let root = root.rid().clone();

    let mut runner_settings = state
        .user()
        .lock()
        .unwrap()
        .as_ref()
        .map(|user| settings::user::Runner::load(user.rid()).ok())
        .flatten()
        .unwrap_or_default();

    if let Ok(runner_settings_project) = settings::project::Runner::load(&project_path) {
        let local::project::config::RunnerSettings {
            python_path,
            r_path,
            continue_on_error,
        } = runner_settings_project;

        if let Some(python_path) = python_path {
            let _ = runner_settings.python_path.insert(python_path);
        }
        if let Some(r_path) = r_path {
            let _ = runner_settings.r_path.insert(r_path);
        }
        if let Some(continue_on_error) = continue_on_error {
            runner_settings.continue_on_error = continue_on_error;
        }
    }

    let mut runner_hooks = runner::Builder::new(&project_path, &project_data);
    runner_hooks.settings(&runner_settings);
    let runner_hooks = match runner_hooks.build() {
        Ok(hooks) => hooks,
        Err(err) => return Err(err.into()),
    };
    let mut runner = core::runner::Builder::new(runner_hooks);
    if let Some(max_tasks) = max_tasks {
        runner.num_threads(max_tasks);
    }
    let runner = runner.build();

    let handle = runner.from(project, graph, &root).unwrap();
    tauri::async_runtime::spawn({
        let analyzer_action = (*analyzer_action).clone();
        async move {
            while !handle.done() {
                let status = handle.status();
                let completed = status.iter().filter(|state| state.complete()).count();
                let remaining = status.len() - completed;
                rx.send(lib::event::analysis::Update::Progress {
                    completed,
                    remaining,
                })
                .unwrap();

                // NOTE: Scope needed to isolate `Send`-ness of data
                // See https://github.com/rust-lang/rust/issues/63768.
                {
                    let mut action_guard = analyzer_action.lock().unwrap();
                    if let Some(action) = action_guard.as_ref() {
                        match action {
                            state::AnalyzerAction::Cancel => {
                                handle.cancel();
                            }
                            state::AnalyzerAction::Kill => {
                                handle.kill();
                            }
                        }
                        let _ = action_guard.take();
                    }
                }

                tokio::time::sleep(std::time::Duration::from_millis(PROGRESS_DELAY_MS)).await;
            }

            let status = handle.status();
            rx.send(lib::event::analysis::Update::Done(status)).unwrap();
        }
    });

    Ok(())
}

#[tauri::command]
pub fn cancel_analysis(action: tauri::State<'_, state::Slice<Option<state::AnalyzerAction>>>) {
    let mut action = action.lock().unwrap();
    let _ = action.insert(state::AnalyzerAction::Cancel);
}

#[tauri::command]
pub fn kill_analysis(action: tauri::State<'_, state::Slice<Option<state::AnalyzerAction>>>) {
    let mut action = action.lock().unwrap();
    let _ = action.insert(state::AnalyzerAction::Kill);
}

#[tauri::command]
pub fn delete_project(project: PathBuf) -> Result<(), lib::command::error::Trash> {
    trash::delete(&project).map_err(|err| err.into())
}

fn graph_state_to_runner_tree(
    graph: db::state::Graph,
) -> Result<core::runner::Tree, error::InvalidGraph> {
    let db::state::Graph { nodes, children } = graph;
    let (nodes, errors): (Vec<_>, Vec<_>) = nodes
        .into_iter()
        .map(|node| node.as_container())
        .partition(|node| node.is_some());
    if !errors.is_empty() {
        return Err(error::InvalidGraph::InvalidContainer);
    }
    let nodes = nodes
        .into_iter()
        .map(|node| Arc::new(node.unwrap()))
        .collect::<Vec<_>>();

    let graph = children
        .into_iter()
        .enumerate()
        .map(|(idx, children)| {
            let children = children.into_iter().map(|idx| nodes[idx].clone()).collect();
            (nodes[idx].clone(), children)
        })
        .collect::<Vec<_>>();

    Ok(core::runner::Tree::from_graph(graph)?)
}

mod error {
    use syre_core as core;
    use syre_desktop_lib as lib;

    pub enum InvalidGraph {
        InvalidContainer,
        InvalidTree,
        NoRoot,
    }

    impl From<core::runner::tree::error::InvalidGraph> for InvalidGraph {
        fn from(value: core::runner::tree::error::InvalidGraph) -> Self {
            match value {
                core::runner::tree::error::InvalidGraph::NoRoot => Self::NoRoot,
                core::runner::tree::error::InvalidGraph::InvalidTree => Self::InvalidTree,
            }
        }
    }

    impl Into<lib::command::project::error::TriggerAnalysis> for InvalidGraph {
        fn into(self) -> lib::command::project::error::TriggerAnalysis {
            use lib::command::project::error::TriggerAnalysis;

            match self {
                InvalidGraph::InvalidContainer => TriggerAnalysis::InvalidContainer,
                InvalidGraph::InvalidTree => TriggerAnalysis::InvalidTree,
                InvalidGraph::NoRoot => TriggerAnalysis::NoRoot,
            }
        }
    }
}
