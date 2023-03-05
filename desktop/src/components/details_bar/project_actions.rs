//! Project actions detail widget bar.
use super::project_scripts::ProjectScripts;
use crate::app::{ProjectsStateAction, ProjectsStateReducer};
use crate::commands::container::{
    UpdateScriptAssociationsArgs, UpdateScriptAssociationsStringArgs,
};
use crate::commands::script::{AddScriptArgs, RemoveScriptArgs};
use crate::common::invoke;
use crate::components::canvas::{
    CanvasStateReducer, ContainerTreeStateAction, ContainerTreeStateReducer,
};
use crate::hooks::{use_project, use_project_scripts};
use crate::Result;
use serde_wasm_bindgen as swb;
use std::collections::HashSet;
use std::path::PathBuf;
use thot_core::project::Script as CoreScript;
use thot_core::types::ResourceId;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[function_component(ProjectActions)]
pub fn project_actions() -> Html {
    let projects_state =
        use_context::<ProjectsStateReducer>().expect("`ProjectsStateReducer` context not found");

    let canvas_state =
        use_context::<CanvasStateReducer>().expect("`CanvasStateReducer` context not found");

    let tree_state = use_context::<ContainerTreeStateReducer>()
        .expect("`ContainerTreeStateReducer` context not found");

    let project = use_project(&canvas_state.project);
    let Some(project) = project.as_ref() else {
        panic!("`Project` not loaded");
    };

    let project_scripts = use_project_scripts(canvas_state.project.clone());

    let onadd_scripts = {
        let projects_state = projects_state.clone();
        let project_scripts = project_scripts.clone();
        let project = project.rid.clone();

        Callback::from(move |paths: HashSet<PathBuf>| {
            let projects_state = projects_state.clone();
            let project_scripts = project_scripts.clone();
            let project = project.clone();

            spawn_local(async move {
                let Some(mut scripts) = (*project_scripts).clone() else {
                    panic!("`Project` `Script`s not loaded");
                };

                for path in paths {
                    let project = project.clone();
                    let script = invoke(
                        "add_script",
                        AddScriptArgs {
                            project: project.clone(),
                            path,
                        },
                    )
                    .await
                    .expect("could not invoke `add_script`");

                    let script: CoreScript = swb::from_value(script)
                        .expect("could not convert result of `add_script` to `Script`");

                    scripts.insert(script.rid.clone(), script);
                }

                projects_state
                    .dispatch(ProjectsStateAction::InsertProjectScripts(project, scripts));
            });
        })
    };

    let onremove_script = {
        let projects_state = projects_state.clone();
        let tree_state = tree_state.clone();
        let project_scripts = project_scripts.clone();
        let project = project.rid.clone();

        Callback::from(move |rid: ResourceId| {
            let projects_state = projects_state.clone();
            let tree_state = tree_state.clone();
            let project_scripts = project_scripts.clone();
            let project = project.clone();

            spawn_local(async move {
                let Some(mut scripts) = (*project_scripts).clone() else {
                    panic!("`Project` `Script`s not loaded");
                };

                let project = project.clone();
                let res = invoke(
                    "remove_script",
                    RemoveScriptArgs {
                        project: project.clone(),
                        script: rid.clone(),
                    },
                )
                .await
                .expect("could not invoke `remove_script`");

                // @todo[2]: Process result display error to user
                // let res: Result = swb::from_value(res)
                //     .expect("could not convert result of `remove_script` to `Result`");

                // Remove from containers
                for container in tree_state.containers.values() {
                    let Some(container) = container else { panic!("`Container` not loaded") };
                    let container = container.lock().expect("could not lock `Container`");
                    let rid = container.rid.clone();
                    let mut associations = container.scripts.clone();
                    web_sys::console::log_1(&format!("{:#?}", associations).into());
                    drop(container);
                    associations.remove(&rid);
                    tree_state.dispatch(
                        ContainerTreeStateAction::UpdateContainerScriptAssociations(
                            UpdateScriptAssociationsArgs {
                                rid: rid.clone(),
                                associations,
                            },
                        ),
                    );
                    // @remove
                    let containers = tree_state.containers.get(&rid).unwrap().clone().unwrap();
                    let containers = containers.lock().unwrap();
                    web_sys::console::log_1(&format!("{:#?}", containers.scripts).into());
                }

                // Remove from scripts
                scripts.remove(&rid);

                projects_state
                    .dispatch(ProjectsStateAction::InsertProjectScripts(project, scripts));
            });
        })
    };

    html! {
        <div>
            <ProjectScripts onadd={onadd_scripts} onremove={onremove_script} />
        </div>
    }
}

#[cfg(test)]
#[path = "./project_actions_test.rs"]
mod project_actions_test;
