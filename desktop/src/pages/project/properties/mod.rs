use super::state::{self, workspace_graph};
use crate::types;
use leptos::*;
use std::fmt;
use syre_desktop_lib as lib;

mod analyses;
mod asset;
mod asset_bulk;
mod common;
mod container;
mod container_bulk;
mod mixed_bulk;
mod project;

use analyses::Editor as Analyses;
use asset::Editor as Asset;
use asset_bulk::Editor as AssetBulk;
use container::Editor as Container;
use container_bulk::Editor as ContainerBulk;
use mixed_bulk::Editor as MixedBulk;
use project::Editor as Project;

/// Id for the analyses properties bar.
pub const ANALYSES_ID: &'static str = "analyses";

#[derive(Clone, Copy, derive_more::Deref)]
struct PopoutPortal(NodeRef<html::Div>);

#[derive(Clone, Copy)]
pub enum EditorKind {
    Project,
    Analyses,
    Container,
    Asset,
    ContainerBulk,
    AssetBulk,
    MixedBulk,
}

impl Default for EditorKind {
    fn default() -> Self {
        Self::Analyses
    }
}

#[derive(derive_more::Deref, Clone, Copy)]
pub struct InputDebounce(Signal<f64>);

#[component]
pub fn PropertiesBar() -> impl IntoView {
    let user_settings = expect_context::<types::settings::User>();
    let graph = expect_context::<state::Graph>();
    let workspace_graph_state = expect_context::<state::WorkspaceGraph>();
    let active_editor = expect_context::<RwSignal<EditorKind>>();
    let popout_portal = NodeRef::<html::Div>::new();
    provide_context(PopoutPortal(popout_portal));
    provide_context(InputDebounce(Signal::derive(move || {
        user_settings.with(|settings| {
            let debounce = match &settings.desktop {
                Ok(settings) => settings.input_debounce_ms,
                Err(_) => lib::settings::Desktop::default().input_debounce_ms,
            };

            debounce as f64
        })
    })));

    create_effect({
        let selected = workspace_graph_state.selection_resources().selected();
        move |_| active_editor.set(active_editor_from_selection(selected).into())
    });

    let widget = {
        let selected = workspace_graph_state.selection_resources().selected();
        move || {
            active_editor.with(|active_editor| match active_editor {
                EditorKind::Project => view! { <Project /> }.into_view(),
                EditorKind::Analyses => view! {
                    <div id=ANALYSES_ID class="h-full">
                        <Analyses />
                    </div>
                }
                .into_view(),
                EditorKind::Container => {
                    let container = selected.with(|selected| {
                        let [resource] = &selected[..] else {
                            panic!("invalid state");
                        };

                        resource
                            .rid()
                            .with_untracked(|rid| graph.find_by_id(rid).unwrap())
                    });

                    view! { <Container container=container.state().clone() /> }.into_view()
                }
                EditorKind::Asset => {
                    let asset = selected.with(|selected| {
                        let [resource] = &selected[..] else {
                            panic!("invalid state");
                        };

                        resource
                            .rid()
                            .with(|rid| graph.find_asset_by_id(rid).unwrap())
                    });

                    view! { <Asset asset /> }.into_view()
                }
                EditorKind::ContainerBulk => {
                    let containers = Signal::derive(move || {
                        selected.with(|selected| {
                            selected
                                .iter()
                                .map(|resource| resource.rid().get())
                                .collect::<Vec<_>>()
                        })
                    });

                    view! { <ContainerBulk containers /> }.into_view()
                }
                EditorKind::AssetBulk => {
                    let assets = Signal::derive(move || {
                        selected.with(|selected| {
                            selected
                                .iter()
                                .map(|resource| resource.rid().get())
                                .collect::<Vec<_>>()
                        })
                    });

                    view! { <AssetBulk assets /> }.into_view()
                }
                EditorKind::MixedBulk => view! { <MixedBulk resources=selected /> }.into_view(),
            })
        }
    };

    view! {
        <div class="h-full relative">
            {widget}
            <div ref=popout_portal class="absolute top-1/3 -left-[105%] right-[105%]"></div>
        </div>
    }
}

fn sort_resource_kind(
    a: &state::workspace_graph::ResourceKind,
    b: &state::workspace_graph::ResourceKind,
) -> std::cmp::Ordering {
    use state::workspace_graph::ResourceKind;
    use std::cmp::Ordering;

    match (a, b) {
        (ResourceKind::Container, ResourceKind::Asset) => Ordering::Less,
        (ResourceKind::Asset, ResourceKind::Container) => Ordering::Greater,
        (ResourceKind::Container, ResourceKind::Container)
        | (ResourceKind::Asset, ResourceKind::Asset) => Ordering::Equal,
    }
}

fn active_editor_from_selection(
    selection: ReadSignal<Vec<workspace_graph::Resource>>,
) -> EditorKind {
    use state::workspace_graph::ResourceKind;
    selection.with(|selected| match &selected[..] {
        [] => EditorKind::Analyses,
        [resource] => match resource.kind() {
            ResourceKind::Container => EditorKind::Container,
            ResourceKind::Asset => EditorKind::Asset,
        },
        _ => {
            let mut kinds = selected
                .iter()
                .map(|resource| resource.kind())
                .collect::<Vec<_>>();
            kinds.sort_by(|a, b| sort_resource_kind(a, b));
            kinds.dedup();

            match kinds[..] {
                [] => panic!("invalid state"),
                [kind] => match kind {
                    ResourceKind::Container => EditorKind::ContainerBulk,
                    ResourceKind::Asset => EditorKind::AssetBulk,
                },
                _ => EditorKind::MixedBulk,
            }
        }
    })
}

/// Calculates the y-coordinate the details popout should appear at.
///
/// # Returns
/// y-coordinate of the base relative to the parent, clamped to be within the viewport.
pub fn detail_popout_top(
    popout: &HtmlElement<html::Div>,
    base: &HtmlElement<html::Div>,
    parent: &HtmlElement<html::Div>,
) -> i32 {
    const MARGIN: i32 = 5;

    let popout_rect = popout.get_bounding_client_rect();
    let base_rect = base.get_bounding_client_rect();
    let parent_rect = parent.get_bounding_client_rect();
    let y_max = (parent_rect.height() - popout_rect.height()) as i32 - MARGIN;
    let top = (base_rect.top() - parent_rect.top()) as i32;
    crate::common::clamp(top, MARGIN, y_max)
}

/// Intended to take in a list of errors and produce a `<ul>`.
fn errors_to_list_view(errors: Vec<impl fmt::Debug>) -> impl IntoView {
    view! {
        <ul>
            {errors
                .into_iter()
                .map(|err| {
                    view! { <li>{format!("{err:?}")}</li> }
                })
                .collect::<Vec<_>>()}
        </ul>
    }
}
