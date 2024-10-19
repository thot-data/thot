use super::{
    state::{self, workspace_graph::SelectedResource},
    workspace,
};
use crate::types;
use leptos::*;
use std::fmt;
use syre_core::types::ResourceId;
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

#[derive(Clone)]
pub enum EditorKind {
    Project,
    Analyses,
    Container(state::Container),
    Asset(state::Asset),
    ContainerBulk(Signal<Vec<ResourceId>>),
    AssetBulk(Signal<Vec<ResourceId>>),
    MixedBulk(Memo<Vec<SelectedResource>>),
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
    let active_editor = expect_context::<RwSignal<workspace::PropertiesEditor>>();
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
        let graph = graph.clone();
        move |_| {
            let editor_kind =
                active_editor_from_selection(workspace_graph_state.selection(), &graph);
            active_editor.set(editor_kind.into())
        }
    });

    let widget = move || {
        active_editor.with(|active_editor| match &**active_editor {
            EditorKind::Project => view! { <Project /> }.into_view(),
            EditorKind::Analyses => view! {
                <div id=ANALYSES_ID class="h-full">
                    <Analyses />
                </div>
            }
            .into_view(),
            EditorKind::Container(container) => {
                view! { <Container container=container.clone() /> }.into_view()
            }
            EditorKind::Asset(asset) => view! { <Asset asset=asset.clone() /> }.into_view(),
            EditorKind::ContainerBulk(containers) => {
                view! { <ContainerBulk containers=containers.clone() /> }.into_view()
            }
            EditorKind::AssetBulk(assets) => {
                view! { <AssetBulk assets=assets.clone() /> }.into_view()
            }
            EditorKind::MixedBulk(resources) => {
                view! { <MixedBulk resources=resources.clone() /> }.into_view()
            }
        })
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
    selection: ReadSignal<Vec<SelectedResource>>,
    graph: &state::Graph,
) -> EditorKind {
    use state::workspace_graph::ResourceKind;

    selection.with(|selected| match &selected[..] {
        [] => EditorKind::Analyses,
        [resource] => match resource.kind() {
            ResourceKind::Container => {
                let container = graph.find_by_id(resource.rid()).unwrap();
                EditorKind::Container(container.state().clone())
            }
            ResourceKind::Asset => {
                let asset = graph.find_asset_by_id(resource.rid()).unwrap();
                EditorKind::Asset(asset)
            }
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
                    ResourceKind::Container => {
                        let containers = {
                            let selection = selected.clone();
                            move || {
                                selection
                                    .iter()
                                    .map(|resource| resource.rid().clone())
                                    .collect()
                            }
                        };

                        let containers = Signal::derive(containers);
                        EditorKind::ContainerBulk(containers)
                    }
                    ResourceKind::Asset => {
                        let assets = {
                            let selection = selected.clone();
                            move || {
                                selection
                                    .iter()
                                    .map(|resource| resource.rid().clone())
                                    .collect()
                            }
                        };

                        let assets = Signal::derive(assets);
                        EditorKind::AssetBulk(assets)
                    }
                },
                _ => {
                    let resources = create_memo(move |_| selection.get());
                    EditorKind::MixedBulk(resources)
                }
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
