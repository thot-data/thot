//! Main application.
use super::{
    app_state::AppState, auth_state::AuthState, projects_state::ProjectsState, AppStateAction,
    AppStateReducer, AuthStateReducer, ProjectsStateAction, ProjectsStateReducer,
};
use crate::commands::project::load_user_projects;
use crate::components::messages::Messages;
use crate::routes::{routes::switch, Route};
use crate::widgets::GlobalWidgets;
use thot_local_database::error::server::LoadUserProjects as LoadUserProjectsError;
use thot_ui::types::Message;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

// *********************
// *** App Component ***
// *********************

#[cfg_attr(doc, aquamarine::aquamarine)]
/// App initialization
///
/// ```mermaid
/// flowchart TD
///      start(Initialize app) --> get_active_user(Get active user)
///      get_active_user -- Set --> set_state(Set state)
///      get_active_user -- Not set --> sign_in(Sign in)
///      set_state --> finish(App initialized)
///      sign_in -- Has account --> set_state
///      sign_in -- New user --> create_account(Create account)
///      create_account --> set_state
/// ```
#[function_component(App)]
pub fn app() -> Html {
    let auth_state = use_reducer(|| AuthState::default());
    let app_state = use_reducer(|| AppState::default());
    let projects_state = use_reducer(|| ProjectsState::default());
    let project_manifest_state = use_state(|| Ok(()));

    // load user projects
    use_effect_with(auth_state.clone(), {
        let app_state = app_state.clone();
        let projects_state = projects_state.clone();
        let project_manifest_state = project_manifest_state.setter();

        move |auth_state| {
            let Some(user) = auth_state.user.as_ref() else {
                return;
            };

            let user_id = user.rid.clone();
            let projects_state = projects_state.clone();

            spawn_local(async move {
                match load_user_projects(user_id).await {
                    Ok(projects) => {
                        project_manifest_state.set(Ok(()));
                        projects_state.dispatch(ProjectsStateAction::InsertProjects(projects));
                    }

                    Err(LoadUserProjectsError::LoadProjectsManifest(err)) => {
                        project_manifest_state.set(Err(err));
                    }

                    Err(LoadUserProjectsError::LoadProjects { projects, errors }) => {
                        let details = errors
                            .iter()
                            .map(|(path, err)| format!("{path:?}: {err}"))
                            .collect::<Vec<_>>()
                            .join(", ");

                        let mut msg = Message::error("Some projects could not be loaded.");
                        msg.set_details(details);
                        app_state.dispatch(AppStateAction::AddMessage(msg));
                        project_manifest_state.set(Ok(()));
                        projects_state.dispatch(ProjectsStateAction::InsertProjects(projects));
                    }
                };
            });
        }
    });

    // TODO Respond to `open_settings` event.
    // use futures::stream::StreamExt;
    // use thot_core::project::Project;
    // use thot_local::types::ProjectSettings;
    // use_effect_with((), move |_| {
    //     spawn_local(async move {
    //         let mut events =
    //             tauri_sys::event::listen::<thot_local_database::Update>("thot://settings")
    //                 .await
    //                 .expect("could not create `thot://settings` listener");

    //         while let Some(event) = events.next().await {
    //             tracing::debug!(?event);
    //             handle_event(event.payload).unwrap();
    //         }
    //     });
    // });

    if let Err(err) = (*project_manifest_state).as_ref() {
        return html! {
            <div>
                <h1>{"Could not load project manifest"}</h1>
                <div>{ format!("Error: {err}") }</div>
            </div>
        };
    }

    html! {
        <BrowserRouter>
        <ContextProvider<AuthStateReducer> context={auth_state.clone()}>
        <ContextProvider<AppStateReducer> context={app_state}>
            if auth_state.is_authenticated() {
                <ContextProvider<ProjectsStateReducer> context={projects_state}>
                    <div id={"content"}>
                        <main>
                            <Switch<Route> render={switch} />
                        </main>
                        <Messages />
                        <GlobalWidgets />
                        <div id={"app-main-shadow-box"}></div>
                    </div>
                </ContextProvider<ProjectsStateReducer>>
            } else {
                <main>
                    <Switch<Route> render={switch} />
                </main>
            }
        </ContextProvider<AppStateReducer>>
        </ContextProvider<AuthStateReducer>>
        </BrowserRouter>
    }
}
