//! Gets the active `Project`'s id.
use crate::app::ProjectsStateReducer;
use thot_core::types::ResourceId;
use yew::prelude::*;

#[hook]
pub fn use_active_project() -> UseStateHandle<Option<ResourceId>> {
    let projects_state =
        use_context::<ProjectsStateReducer>().expect("`ProjectsStateReducer` context not found");

    let active_project = use_state(|| projects_state.active_project.clone());

    {
        let projects_state = projects_state.clone();
        let active_project = active_project.clone();

        use_effect_with(projects_state, move |projects_state| {
            active_project.set(projects_state.active_project.clone());
        });
    };

    active_project
}
