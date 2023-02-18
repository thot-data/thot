//! UI for a `Container` preview within a [`ContainerTree`](super::ContainerTree).
//! Acts as a wrapper around a [`thot_ui::widgets::container::container_tree::Container`].
use crate::app::{AppStateAction, AppStateReducer};
use crate::components::asset::CreateAssets;
use crate::components::canvas::{
    CanvasStateAction, CanvasStateReducer, ContainerTreeStateAction, ContainerTreeStateReducer,
};
use crate::components::details_bar::DetailsBarWidget;
use crate::hooks::use_container;
use thot_core::types::ResourceId;
use thot_ui::components::ShadowBox;
use thot_ui::types::Message;
use thot_ui::widgets::container::container_tree::{
    container::ContainerProps as ContainerUiProps, container::ContainerSettingsMenuEvent,
    Container as ContainerUi,
};
use yew::prelude::*;
use yew::props;

#[derive(Properties, PartialEq)]
pub struct ContainerProps {
    pub rid: ResourceId,

    /// Callback to run when the add child button is clicked.
    #[prop_or_default]
    pub onadd_child: Option<Callback<ResourceId>>,
}

#[function_component(Container)]
pub fn container(props: &ContainerProps) -> HtmlResult {
    // -------------
    // --- setup ---
    // -------------
    let app_state = use_context::<AppStateReducer>().expect("`AppStateReducer` context not found");
    let canvas_state =
        use_context::<CanvasStateReducer>().expect("`CanvasStateReducer` context not found");

    let tree_state = use_context::<ContainerTreeStateReducer>()
        .expect("`ContainerTreeReducer` context not found");

    let show_create_asset = use_state(|| false);
    let container = use_container(props.rid.clone());
    let Some(container) = container.as_ref() else {
        panic!("`Container` not loaded");
    };

    let container_id = {
        let container = container.lock().expect("could not lock `Container`");
        container.rid.clone()
    };

    let selected = canvas_state.selected.contains(&container_id);
    let multiple_selected = canvas_state.selected.len() > 1;

    // -------------------
    // --- interaction ---
    // -------------------

    let onclick = {
        let canvas_state = canvas_state.clone();
        let container_id = container_id.clone();
        let selected = selected.clone();
        let multiple_selected = multiple_selected.clone();

        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            let container_id = container_id.clone();
            match selection_action(selected, multiple_selected, e) {
                SelectionAction::SelectOnly => {
                    canvas_state.dispatch(CanvasStateAction::ClearSelected);
                    canvas_state.dispatch(CanvasStateAction::SelectContainer(container_id));
                }

                SelectionAction::Select => {
                    canvas_state.dispatch(CanvasStateAction::SelectContainer(container_id));
                }

                SelectionAction::Unselect => {
                    canvas_state.dispatch(CanvasStateAction::Unselect(container_id));
                }
            }
        })
    };

    // ----------------------------
    // --- settings menu events ---
    // ----------------------------

    let on_settings_event = {
        let show_create_asset = show_create_asset.clone();

        Callback::from(move |event: ContainerSettingsMenuEvent| match event {
            ContainerSettingsMenuEvent::AddAsset => show_create_asset.set(true),
            ContainerSettingsMenuEvent::Analyze => {}
        })
    };

    let close_create_asset = {
        let show_create_asset = show_create_asset.clone();

        Callback::from(move |_: MouseEvent| {
            show_create_asset.set(false);
        })
    };

    // --------------
    // --- assets ---
    // --------------

    let onclick_asset = {
        // let app_state = app_state.clone();
        let canvas_state = canvas_state.clone();
        let tree_state = tree_state.clone();
        let selected = selected.clone();
        let multiple_selected = multiple_selected.clone();

        Callback::from(move |(asset, e): (ResourceId, MouseEvent)| {
            let container = tree_state
                .asset_map
                .get(&asset)
                .expect("`Asset`'s `Container` not found");

            let container = tree_state
                .containers
                .get(container)
                .expect("`Container` not found")
                .as_ref()
                .expect("`Container` not set")
                .lock()
                .expect("could not lock `Container`");

            let asset = container.assets.get(&asset).expect("`Asset` not found");
            let rid = asset.rid.clone();
            match selection_action(selected, multiple_selected, e) {
                SelectionAction::SelectOnly => {
                    canvas_state.dispatch(CanvasStateAction::ClearSelected);
                    canvas_state.dispatch(CanvasStateAction::SelectAsset(rid));
                }

                SelectionAction::Select => {
                    canvas_state.dispatch(CanvasStateAction::SelectAsset(rid));
                }

                SelectionAction::Unselect => {
                    canvas_state.dispatch(CanvasStateAction::Unselect(rid));
                }
            }
        })
    };

    let onadd_assets = {
        let show_create_asset = show_create_asset.clone();

        Callback::from(move |_: ()| {
            show_create_asset.set(false);
        })
    };

    // ---------------
    // --- scripts ---
    // ---------------

    let onclick_edit_scripts = {
        let app_state = app_state.clone();
        let canvas_state = canvas_state.clone();

        Callback::from(move |container: ResourceId| {
            let onsave = {
                let app_state = app_state.clone();
                let canvas_state = canvas_state.clone();

                Callback::from(move |_: ()| {
                    canvas_state.dispatch(CanvasStateAction::ClearDetailsBar);
                    app_state.dispatch(AppStateAction::AddMessage(Message::success(
                        "Resource saved".to_string(),
                    )));
                })
            };

            canvas_state.dispatch(CanvasStateAction::SetDetailsBarWidget(
                DetailsBarWidget::ScriptsAssociationsEditor(container.clone(), Some(onsave)),
            ));
        })
    };

    // ----------------------
    // --- on drop events ---
    // ----------------------

    let ondragenter = {
        let tree_state = tree_state.clone();
        let container_id = container_id.clone();

        Callback::from(move |_: web_sys::DragEvent| {
            tree_state.dispatch(ContainerTreeStateAction::SetDragOverContainer(
                container_id.clone(),
            ));
        })
    };

    let ondragleave = {
        let tree_state = tree_state.clone();

        Callback::from(move |_: web_sys::DragEvent| {
            tree_state.dispatch(ContainerTreeStateAction::ClearDragOverContainer);
        })
    };

    // ----------
    // --- ui ---
    // ----------

    // props
    let mut class = Classes::new();
    if selected {
        class.push("selected");
    }

    let container_val = container.lock().expect("could not lock container").clone();
    let c_props = {
        let c = container_val.clone();

        props! {
            ContainerUiProps {
                class,
                rid: c.rid,
                properties: c.properties,
                assets: c.assets,
                active_assets: canvas_state.selected.clone(),
                scripts: c.scripts,
                preview: tree_state.preview.clone(),
                onclick,
                onclick_asset,
                onadd_child: props.onadd_child.clone(),
                on_settings_event,
                ondragenter,
                ondragleave,
                onclick_edit_scripts,
            }
        }
    };

    let container_name = match container_val.properties.name.clone() {
        None => "(no name)".to_string(),
        Some(name) => name,
    };

    Ok(html! {
        <>
        <ContainerUi ..c_props />
        if *show_create_asset {
            <ShadowBox
                title={format!("Add Asset to {container_name}")}
                onclose={close_create_asset}>

                <CreateAssets
                    container={container_val.rid.clone()}
                    onsuccess={onadd_assets} />
            </ShadowBox>
        }
        </>
    })
}

// ***************
// *** helpers ***
// ***************

enum SelectionAction {
    SelectOnly,
    Select,
    Unselect,
}

/// Determines the selection action from the current action and state.
///
/// # Arguments
/// 1. If the clicked resource is currently selected.
/// 2. If at least one other resource is currently selected.
/// 3. The [`MouseEvent`].
fn selection_action(selected: bool, multiple: bool, e: MouseEvent) -> SelectionAction {
    if e.ctrl_key() {
        if selected {
            return SelectionAction::Unselect;
        } else {
            return SelectionAction::Select;
        }
    }

    if selected {
        if multiple {
            return SelectionAction::SelectOnly;
        }

        return SelectionAction::Unselect;
    }

    SelectionAction::SelectOnly
}

#[cfg(test)]
#[path = "./container_test.rs"]
mod container_test;
