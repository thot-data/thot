use crate::app::{ProjectsStateAction, ProjectsStateReducer};
use crate::hooks::{use_active_project, use_open_projects, use_user, use_user_projects};
use crate::routes::Route;
use indexmap::IndexMap;
use thot_core::project::Project;
use thot_core::types::ResourceId;
use thot_ui::components::navigation::{TabBar, TabCloseInfo};
use yew::prelude::*;
use yew_router::prelude::*;

#[function_component(MainNavigation)]
pub fn main_navigation() -> Html {
    let navigator = use_navigator().expect("navigator not found");
    let projects_state =
        use_context::<ProjectsStateReducer>().expect("`ProjectsStateReducer` context not found");

    let open_projects = use_open_projects();
    let user = use_user();
    let Some(user) = user.as_ref() else {
        panic!("user not set"); // @todo: Redirect to login.
    };

    let user_projects = use_user_projects(&user.rid);
    let active_project = use_active_project();
    let tabs = use_state(|| {
        let projects = user_projects
            .iter()
            .filter(|prj| open_projects.contains(&prj.rid))
            .collect::<Vec<&Project>>();

        projects_to_tabs(projects)
    });

    // update tabs when open projects change
    use_effect_with(open_projects.clone(), {
        let tabs = tabs.clone();
        let user_projects = user_projects.clone();

        move |open_projects| {
            let projects = user_projects
                .iter()
                .filter(|prj| open_projects.contains(&prj.rid))
                .collect::<Vec<&Project>>();

            tabs.set(projects_to_tabs(projects));
        }
    });

    // -----------------
    // --- callbacks ---
    // -----------------

    let activate_project = use_callback((), {
        let projects_state = projects_state.dispatcher();
        let navigator = navigator.clone();

        move |pid: ResourceId, _| {
            projects_state.dispatch(ProjectsStateAction::SetActiveProject(pid));
            navigator.push(&Route::Workspace);
        }
    });

    let close_project = use_callback(projects_state.clone(), {
        let navigator = navigator.clone();
        move |TabCloseInfo { closing, next }, projects_state| {
            projects_state.dispatch(ProjectsStateAction::RemoveOpenProject(closing, next));
            if projects_state.open_projects.len() == 1 {
                navigator.push(&Route::Dashboard);
            }
        }
    });

    html! {
        <div id={"main-navigation-tabs"}>
            <span
                id={"home-navigation"}
                class={classes!("inline-block")}>

                <Link<Route> to={Route::Dashboard}>
                    <img class={classes!("logo-tab-container")} src="/public/logos/logo-white-icon.svg" />
                </Link<Route>>
            </span>

            <TabBar<ResourceId>
                id={"project-navigation-tabs"}
                class={classes!("inline-block", "tab-horizontal")}
                tabs={(*tabs).clone()}
                active={(*active_project).clone()}
                onclick_tab={activate_project}
                onclick_tab_close={close_project} />
        </div>
    }
}

// ***************
// *** helpers ***
// ***************

/// Converts [`Project`]s to tabs for display.
fn projects_to_tabs(projects: Vec<&Project>) -> IndexMap<ResourceId, String> {
    projects
        .into_iter()
        .map(|p| (p.rid.clone(), p.name.clone()))
        .collect::<IndexMap<ResourceId, String>>()
}
