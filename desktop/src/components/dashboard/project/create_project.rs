//! Create a new [`Project`].
use crate::app::{AppStateAction, AppStateReducer, ProjectsStateAction, ProjectsStateReducer};
use crate::commands::graph::init_project_graph;
use crate::commands::project::{init_project, load_project, update_project};
use crate::hooks::use_user;
use crate::routes::Route;
use std::path::{Path, PathBuf};
use thot_core::types::{Creator, UserId};
use thot_ui::components::{file_selector::FileSelectorProps, FileSelector, FileSelectorAction};
use thot_ui::types::Message;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew::props;
use yew_router::prelude::*;

// ********************************
// *** Create Project Component ***
// ********************************

/// New project component.
/// Consists of three steps:
/// 1. Select folder
///     May select an existing folder, or create a new one.
/// 2. Assign properties with optional advanced properties.
/// 3. Build the project tree.
#[tracing::instrument]
#[function_component(CreateProject)]
pub fn create_project() -> Html {
    let navigator = use_navigator().expect("navigator not found");
    let app_state = use_context::<AppStateReducer>().expect("`AppStateReducer` context not found");
    let projects_state =
        use_context::<ProjectsStateReducer>().expect("`ProjectsStateReducer` context not found");

    let user = use_user();
    let Some(user) = user.as_ref() else {
        navigator.push(&Route::SignIn);
        app_state.dispatch(AppStateAction::AddMessage(Message::error(
            "Could not get user.",
        )));
        return html! {};
    };

    let onsuccess = {
        let app_state = app_state.clone();
        let projects_state = projects_state.clone();
        let navigator = navigator.clone();
        let user = user.rid.clone();

        Callback::from(move |path: PathBuf| {
            let app_state = app_state.clone();
            let projects_state = projects_state.clone();
            let navigator = navigator.clone();
            let user = user.clone();

            app_state.dispatch(AppStateAction::SetActiveWidget(None)); // close self

            // create and go to project
            spawn_local(async move {
                // TODO[m]: Validate path is not already a project.
                // init project
                match init_project(path.clone()).await {
                    Ok(_rid) => {}
                    Err(err) => {
                        let mut msg = Message::error("Could not create project");
                        msg.set_details(err);
                        app_state.dispatch(AppStateAction::AddMessage(msg));
                        return;
                    }
                };

                let (mut project, settings) = match load_project(path.clone()).await {
                    Ok(project) => project,
                    Err(err) => {
                        let mut msg = Message::error("Could not load project");
                        msg.set_details(format!("{err:?}"));
                        app_state.dispatch(AppStateAction::AddMessage(msg));
                        return;
                    }
                };

                // initialize data root as container
                let mut data_root_abs = path.clone();
                let data_root_rel = Path::new("data");
                data_root_abs.push(data_root_rel);

                // TODO[l]: Could load graph here.
                let _graph =
                    match init_project_graph(project.rid.clone(), data_root_abs.clone()).await {
                        Ok(graph) => graph,
                        Err(err) => {
                            let mut msg = Message::error("Could not create project graph.");
                            msg.set_details(err);
                            app_state.dispatch(AppStateAction::AddMessage(msg));
                            return;
                        }
                    };

                // save project
                project.data_root = Some(data_root_rel.to_path_buf());
                project.creator = Creator::User(Some(UserId::Id(user)));
                match update_project(project.clone()).await {
                    Ok(_) => {}
                    Err(err) => {
                        let mut msg = Message::error("Could not update project");
                        msg.set_details(format!("{err:?}"));
                        app_state.dispatch(AppStateAction::AddMessage(msg));
                        return;
                    }
                };

                // update ui
                let rid = project.rid.clone();
                projects_state.dispatch(ProjectsStateAction::InsertProject((project, settings)));
                projects_state.dispatch(ProjectsStateAction::AddOpenProject(rid.clone()));
                projects_state.dispatch(ProjectsStateAction::SetActiveProject(rid));

                navigator.push(&Route::Workspace);
            });
        })
    };

    let oncancel = {
        let app_state = app_state.clone();

        Callback::from(move |_| {
            app_state.dispatch(AppStateAction::SetActiveWidget(None));
        })
    };

    let props = props! {
        FileSelectorProps {
            title: "Select project directory",
            action: FileSelectorAction::PickFolder,
            show_cancel: false,
            onsuccess,
            oncancel,
        }
    };

    html! {
        <FileSelector ..props />
    }
}
