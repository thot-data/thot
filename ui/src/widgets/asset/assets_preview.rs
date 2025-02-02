//! Assets preview.
use crate::widgets::common::asset as common;
use std::collections::HashSet;
use thot_core::project::Asset;
use thot_core::types::ResourceId;
use yew::prelude::*;
use yew_icons::Icon;

#[derive(Properties, PartialEq, Debug)]
pub struct AssetsPreviewProps {
    /// [`Asset`]s to display.
    pub assets: Vec<Asset>,

    /// Selected.
    #[prop_or_default]
    pub active: HashSet<ResourceId>,

    /// Callback when an [`Asset`](Asset) is clicked.
    #[prop_or_default]
    pub onclick_asset: Option<Callback<(ResourceId, MouseEvent)>>,

    /// Callback when an [`Asset`](Asset) is double clicked.
    #[prop_or_default]
    pub ondblclick_asset: Option<Callback<(ResourceId, MouseEvent)>>,

    /// Callback when an [`Asset`](Asset) is to be deleted.
    #[prop_or_default]
    pub onclick_asset_remove: Option<Callback<ResourceId>>,
}

#[function_component(AssetsPreview)]
#[tracing::instrument(level = "debug")]
pub fn assets_preview(props: &AssetsPreviewProps) -> Html {
    // NOTE: Check double click was for same asset,
    // otherwise removing an asset may trigger double click.
    let clicked_asset = use_state(|| None);
    let mut assets = props.assets.clone();
    assets.sort_by(|a, b| a.path.as_path().cmp(b.path.as_path()));

    html! {
        <div class={classes!("assets-preview")}>
            if assets.len() == 0 {
             { "(no data)" }
            } else {
                <ol class={classes!("thot-ui-assets-list")}>
                    { assets.iter().map(|asset| {
                        let mut class = classes!("thot-ui-asset-preview", "clickable");
                        if props.active.contains(&asset.rid) {
                            class.push("active");
                        }

                        let display_name = common::asset_display_name(&asset);
                        html! {
                            <li key={asset.rid.clone()}
                                {class}
                                onclick={onclick_asset(
                                    asset.rid.clone(),
                                    props.onclick_asset.clone(),
                                    clicked_asset.clone()
                                )}
                                ondblclick={ondblclick_asset(
                                    asset.rid.clone(),
                                    props.ondblclick_asset.clone(),
                                    clicked_asset.clone(),
                                )} >

                                <div class={classes!("thot-ui-asset")}>
                                    <div style={ common::asset_icon_color(&asset) }>
                                        <Icon class={classes!("thot-ui-asset-icon")} icon_id={common::asset_icon_id(&asset)} />
                                    </div>

                                    <div class={classes!("thot-ui-asset-name")}
                                        title={display_name.clone()}>
                                        { display_name }
                                    </div>
                                    if props.onclick_asset_remove.is_some() {
                                        <button onclick={onclick_asset_remove(
                                            asset.rid.clone(),
                                            props.onclick_asset_remove.clone(),
                                            clicked_asset.clone(),
                                        )} class={classes!("thot-ui-asset-remove")}>
                                            { "X" }
                                        </button>
                                    }
                                </div>
                            </li>
                        }
                    }).collect::<Html>() }
                </ol>
            }
        </div>
    }
}

// ***************
// *** helpers ***
// ***************

/// Creates a [`Callback`] that passes the [`ResourceId`] through as the only parameter, and sets
/// the asset click state.
#[tracing::instrument]
fn onclick_asset(
    rid: ResourceId,
    cb: Option<Callback<(ResourceId, MouseEvent)>>,
    clicked_asset_state: UseStateHandle<Option<ResourceId>>,
) -> Callback<MouseEvent> {
    Callback::from(move |e: MouseEvent| {
        if e.detail() == 1 {
            // only set on first click
            clicked_asset_state.set(Some(rid.clone()));
        }

        if let Some(cb) = cb.as_ref() {
            e.stop_propagation();
            cb.emit((rid.clone(), e));
        }
    })
}

/// Creates a [`Callback`] that passes the [`ResourceId`] through as the only parameter.
/// Reads the asset click state to ensure the same asset is being clicked.
#[tracing::instrument]
fn ondblclick_asset(
    rid: ResourceId,
    cb: Option<Callback<(ResourceId, MouseEvent)>>,
    clicked_asset_state: UseStateHandle<Option<ResourceId>>,
) -> Callback<MouseEvent> {
    Callback::from(move |e: MouseEvent| {
        if let Some(prev_rid) = clicked_asset_state.as_ref() {
            clicked_asset_state.set(Some(rid.clone()));

            if prev_rid != &rid {
                return;
            }
        } else {
            panic!("double click triggered without asset click state set");
        }

        if let Some(cb) = cb.as_ref() {
            e.stop_propagation();
            cb.emit((rid.clone(), e));
        }
    })
}

/// Creates a [`Callback`] that passes the [`ResourceId`] through as the only parameter.
#[tracing::instrument]
fn onclick_asset_remove(
    rid: ResourceId,
    cb: Option<Callback<ResourceId>>,
    clicked_asset_state: UseStateHandle<Option<ResourceId>>,
) -> Callback<MouseEvent> {
    Callback::from(move |e: MouseEvent| {
        if e.detail() == 1 {
            // only set on first click
            clicked_asset_state.set(Some(rid.clone()));
        }

        if let Some(cb) = cb.as_ref() {
            e.stop_propagation();
            cb.emit(rid.clone());
        }
    })
}
