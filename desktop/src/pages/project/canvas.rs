use super::{
    common::{asset_title_closure, interpret_resource_selection_action, SelectionAction},
    state,
    workspace::ViewboxState,
};
use crate::{
    commands, common,
    components::{self, ModalDialog, ToggleExpand, TruncateLeft},
    pages::project::actions,
    types,
};
use futures::StreamExt;
use has_id::HasId;
use leptos::{
    either::Either,
    ev::{DragEvent, MouseEvent, WheelEvent},
    html,
    portal::Portal,
    prelude::*,
    svg,
    task::spawn_local,
};
use leptos_icons::*;
use serde::Serialize;
use std::{cmp, io, num::NonZeroUsize, path::PathBuf, sync::Arc};
use syre_core::{project::AnalysisAssociation, types::ResourceId};
use syre_desktop_lib as lib;
use syre_local as local;
use syre_local_database as db;
use tauri_sys::{core::Channel, menu};
use wasm_bindgen::JsCast;

pub const CONTAINER_WIDTH: usize = 300;
const MAX_CONTAINER_HEIGHT: usize = 400;
const CONTAINER_HEADER_HEIGHT: usize = 50;
const CONTAINER_PREVIEW_LINE_HEIGHT: usize = 24;
const PADDING_X_SIBLING: usize = 20;
const PADDING_Y_CHILDREN: usize = 60;
const CANVAS_BUTTON_RADIUS: usize = 10;
const CANVAS_BUTTON_STROKE: usize = 2; // ensure this aligns with the actual stroke width defined in svg elements
const TOGGLE_VIEW_INDICATOR_RADIUS: usize = 3;
const ICON_SCALE: f64 = 0.9;
const VB_SCALE_ENLARGE: f32 = 0.9; // zoom in should reduce viewport.
const VB_SCALE_REDUCE: f32 = 1.1;
pub const VB_BASE: usize = 1000;
const VB_WIDTH_MIN: usize = 500;
const VB_WIDTH_MAX: usize = 10_000;
const VB_HEIGHT_MIN: usize = 500;
const VB_HEIGHT_MAX: usize = 10_000;
pub const DATA_KEY_CONTAINER: &str = "container";
pub const DATA_KEY_ASSET: &str = "asset";

/// Context menu for root container.
#[derive(derive_more::Deref, Clone)]
struct ContextMenuContainerRoot(Arc<menu::Menu>);
impl ContextMenuContainerRoot {
    pub fn new(menu: Arc<menu::Menu>) -> Self {
        Self(menu)
    }
}

/// Context menu for containers that are `Ok`.
#[derive(derive_more::Deref, Clone)]
struct ContextMenuContainerOk(Arc<menu::Menu>);
impl ContextMenuContainerOk {
    pub fn new(menu: Arc<menu::Menu>) -> Self {
        Self(menu)
    }
}

/// Context menu for containers that are `Err`.
#[derive(derive_more::Deref, Clone)]
struct ContextMenuContainerErr(Arc<menu::Menu>);
impl ContextMenuContainerErr {
    pub fn new(menu: Arc<menu::Menu>) -> Self {
        Self(menu)
    }
}

/// Context menu for assets.
#[derive(derive_more::Deref, Clone)]
struct ContextMenuAsset(Arc<menu::Menu>);
impl ContextMenuAsset {
    pub fn new(menu: Arc<menu::Menu>) -> Self {
        Self(menu)
    }
}

/// Active container for the container context menu.
#[derive(derive_more::Deref, derive_more::From, Clone)]
struct ContextMenuActiveContainer(state::graph::Node);

/// Active asset for the asset context menu.
#[derive(derive_more::Deref, derive_more::From, Clone)]
struct ContextMenuActiveAsset(ResourceId);

#[derive(derive_more::Deref, derive_more::From, Clone, Copy)]
struct ContainerPreviewHeight(ReadSignal<usize>);

#[derive(derive_more::Deref, derive_more::From, Clone)]
struct Container(state::graph::Node);

/// Node ref to the modal portal.
#[derive(Clone, derive_more::Deref)]
pub struct PortalRef(NodeRef<html::Div>);

#[component]
pub fn Canvas() -> impl IntoView {
    let project = expect_context::<state::Project>();
    let graph = expect_context::<state::Graph>();
    let messages = expect_context::<types::Messages>();

    let context_menu_active_container: RwSignal<Option<ContextMenuActiveContainer>> =
        RwSignal::<Option<ContextMenuActiveContainer>>::new(None);

    let context_menu_active_asset = RwSignal::<Option<ContextMenuActiveAsset>>::new(None);
    provide_context(context_menu_active_container.clone());
    provide_context(context_menu_active_asset);

    let context_menu_container_root = LocalResource::new({
        let project = project.clone();
        let graph = graph.clone();
        let messages = messages.clone();
        move || {
            let project = project.clone();
            let graph = graph.clone();
            let messages = messages.clone();
            async move {
                let mut container_open = tauri_sys::menu::item::MenuItemOptions::new("Open");
                container_open.set_id("canvas:container-open");

                let (menu, mut listeners) = menu::Menu::with_id_and_items(
                    "canvas:container-ok-context_menu",
                    vec![container_open.into()],
                )
                .await;

                spawn_local({
                    let container_open = listeners.pop().unwrap().unwrap();
                    handle_context_menu_container_root_events(
                        project,
                        graph,
                        messages,
                        container_open,
                    )
                });

                Arc::new(menu)
            }
        }
    });

    let context_menu_container_ok = LocalResource::new({
        let project = project.clone();
        let graph = graph.clone();
        let messages = messages.clone();
        move || {
            let project = project.clone();
            let graph = graph.clone();
            let messages = messages.clone();
            async move {
                let mut container_open = tauri_sys::menu::item::MenuItemOptions::new("Open");
                container_open.set_id("canvas:container-open");

                let mut container_duplicate =
                    tauri_sys::menu::item::MenuItemOptions::new("Duplicate");
                container_duplicate.set_id("canvas:container-duplicate");

                let mut container_trash = tauri_sys::menu::item::MenuItemOptions::new("Trash");
                container_trash.set_id("canvas:container-trash");

                let (menu, mut listeners) = menu::Menu::with_id_and_items(
                    "canvas:container-ok-context_menu",
                    vec![
                        container_open.into(),
                        container_duplicate.into(),
                        container_trash.into(),
                    ],
                )
                .await;

                spawn_local({
                    // pop from end to beginning
                    let container_trash = listeners.pop().unwrap().unwrap();
                    let container_duplicate = listeners.pop().unwrap().unwrap();
                    let container_open = listeners.pop().unwrap().unwrap();
                    handle_context_menu_container_ok_events(
                        project,
                        graph,
                        messages,
                        context_menu_active_container.read_only(),
                        container_open,
                        container_duplicate,
                        container_trash,
                    )
                });

                Arc::new(menu)
            }
        }
    });

    let context_menu_container_err = LocalResource::new({
        let project = project.clone();
        let graph = graph.clone();
        let messages = messages.clone();
        move || {
            let project = project.clone();
            let graph = graph.clone();
            let messages = messages.clone();
            async move {
                let mut container_open = tauri_sys::menu::item::MenuItemOptions::new("Open");
                container_open.set_id("canvas:container-open");

                let mut container_trash = tauri_sys::menu::item::MenuItemOptions::new("Trash");
                container_trash.set_id("canvas:container-trash");

                let (menu, mut listeners) = menu::Menu::with_id_and_items(
                    "canvas:container-err-context_menu",
                    vec![container_open.into(), container_trash.into()],
                )
                .await;

                spawn_local({
                    // pop from end to beginning
                    let container_trash = listeners.pop().unwrap().unwrap();
                    let container_open = listeners.pop().unwrap().unwrap();
                    handle_context_menu_container_err_events(
                        project,
                        graph,
                        messages,
                        context_menu_active_container.read_only(),
                        container_open,
                        container_trash,
                    )
                });

                Arc::new(menu)
            }
        }
    });

    let context_menu_asset = LocalResource::new({
        let project = project.clone();
        let graph = graph.clone();
        let messages = messages.clone();
        move || {
            let project = project.clone();
            let graph = graph.clone();
            let messages = messages.clone();
            async move {
                let mut asset_open = tauri_sys::menu::item::MenuItemOptions::new("Open");
                asset_open.set_id("canvas:asset-open");

                let (menu, mut listeners) = menu::Menu::with_id_and_items(
                    "canvas:asset-context_menu",
                    vec![asset_open.into()],
                )
                .await;

                spawn_local({
                    let asset_open = listeners.pop().unwrap().unwrap();
                    handle_context_menu_asset_events(
                        project,
                        graph,
                        messages,
                        context_menu_active_asset.read_only(),
                        asset_open,
                    )
                });

                Arc::new(menu)
            }
        }
    });

    view! {
        <Suspense fallback=move || {
            view! { <CanvasLoading /> }
        }>

            {move || Suspend::new(async move {
                let context_menu_container_root = context_menu_container_root.await;
                let context_menu_container_ok = context_menu_container_ok.await;
                let context_menu_container_err = context_menu_container_err.await;
                let context_menu_asset = context_menu_asset.await;
                Some(
                    view! {
                        <CanvasView
                            context_menu_container_root
                            context_menu_container_ok
                            context_menu_container_err
                            context_menu_asset
                        />
                    },
                )
            })}

        </Suspense>
    }
}

#[component]
fn CanvasLoading() -> impl IntoView {
    view! { <div class="text-center pt-4">"Setting up canvas"</div> }
}

#[component]
fn CanvasView(
    /// Context menu for the root container.
    context_menu_container_root: Arc<menu::Menu>,
    /// Context menu for `Ok` non-root containers.
    context_menu_container_ok: Arc<menu::Menu>,
    /// Context menu for `Err` non-root containers.
    context_menu_container_err: Arc<menu::Menu>,
    context_menu_asset: Arc<menu::Menu>,
) -> impl IntoView {
    let graph = expect_context::<state::Graph>();
    let workspace_graph_state = expect_context::<state::WorkspaceGraph>();
    let workspace_state = expect_context::<state::Workspace>();
    let display_state = expect_context::<state::Display>();
    let viewbox = expect_context::<ViewboxState>();

    let portal_ref = NodeRef::new();
    let (container_preview_height, set_container_preview_height) = signal(0);

    provide_context(ContextMenuContainerRoot::new(context_menu_container_root));
    provide_context(ContextMenuContainerOk::new(context_menu_container_ok));
    provide_context(ContextMenuContainerErr::new(context_menu_container_err));
    provide_context(ContextMenuAsset::new(context_menu_asset));
    provide_context(PortalRef(portal_ref));
    provide_context(ContainerPreviewHeight(container_preview_height));

    Effect::new(move |_| {
        let height = workspace_state.preview().with(|preview| {
            let mut height: usize = 0;
            if preview.assets {
                height += 3;
            }
            if preview.analyses {
                height += 3;
            }
            if preview.kind {
                height += 1;
            }
            if preview.description {
                height += 3;
            }
            if preview.tags {
                height += 1;
            }
            if preview.metadata {
                height += 5;
            }

            height * CONTAINER_PREVIEW_LINE_HEIGHT
        });

        set_container_preview_height(common::clamp(height, 0, MAX_CONTAINER_HEIGHT));
    });

    let (pan_drag, set_pan_drag) = signal(None);
    let (was_dragged, set_was_dragged) = signal(false);
    let vb_scale = {
        let width = viewbox.width().read_only();
        move || width.with(|width| VB_BASE as f64 / *width as f64)
    };

    let mousedown = move |e: MouseEvent| {
        if e.button() == types::MouseButton::Primary {
            set_pan_drag(Some((e.client_x(), e.client_y())));
        }
    };

    let mouseup = move |e: MouseEvent| {
        if e.button() == types::MouseButton::Primary && pan_drag.with(|c| c.is_some()) {
            if !was_dragged() {
                workspace_graph_state.selection_resources().clear();
            }

            set_pan_drag(None);
            set_was_dragged(false);
        }
    };

    let mousemove = {
        let root = display_state.find(graph.root()).unwrap();
        let viewbox = viewbox.clone();
        move |e: MouseEvent| {
            if pan_drag.with(|c| c.is_some()) {
                assert_eq!(e.button(), types::MouseButton::Primary);
                let (dx, dy) = pan_drag.with(|c| {
                    let (x, y) = c.unwrap();
                    (e.client_x() - x, e.client_y() - y)
                });

                if dx > 0 || dy > 0 {
                    set_was_dragged(true);
                }

                let x = viewbox.x().get() - (dx as f64 / vb_scale()) as isize;
                let y = viewbox.y().get() - (dy as f64 / vb_scale()) as isize;
                let x_max = (root.width().get_untracked().get()
                    * (CONTAINER_WIDTH + PADDING_X_SIBLING)) as isize
                    - viewbox.width().get() as isize / 2;
                let y_max = cmp::max(
                    (root.height().get_untracked().get()
                        * (MAX_CONTAINER_HEIGHT + PADDING_Y_CHILDREN)) as isize
                        - viewbox.height().get() as isize / 2,
                    0,
                );
                viewbox.x().set(common::clamp(
                    x,
                    -TryInto::<isize>::try_into(viewbox.width().get() / 2).unwrap(),
                    x_max.try_into().unwrap(),
                ));
                viewbox.y().set(common::clamp(
                    y,
                    -TryInto::<isize>::try_into(viewbox.height().get() / 2).unwrap(),
                    y_max.try_into().unwrap(),
                ));
                set_pan_drag(Some((e.client_x(), e.client_y())));
            }
        }
    };

    let mouseleave = move |e: MouseEvent| {
        if pan_drag.with(|c| c.is_some()) {
            assert_eq!(e.button(), types::MouseButton::Primary as i16);
            set_pan_drag(None);
        }
    };

    let wheel = {
        let root = display_state.find(graph.root()).unwrap();
        let viewbox = viewbox.clone();
        move |e: WheelEvent| {
            if e.ctrl_key() {
                let ViewboxDimensions {
                    x,
                    y,
                    width,
                    height,
                } = calculate_canvas_viewbox_scaling(
                    e,
                    viewbox.x().get_untracked(),
                    viewbox.y().get_untracked(),
                    viewbox.width().get_untracked(),
                    viewbox.height().get_untracked(),
                );

                viewbox.x().set(x);
                viewbox.y().set(y);
                viewbox.width().set(width);
                viewbox.height().set(height);
            } else if e.shift_key() {
                let (x, y) = calculate_canvas_position_from_wheel_event(
                    e.delta_y(),
                    e.delta_x(),
                    viewbox.x().get(),
                    viewbox.y().get(),
                    viewbox.width().get(),
                    viewbox.height().get(),
                    vb_scale(),
                    root.width().get_untracked().get(),
                    root.height().get_untracked().get(),
                );

                viewbox.x().set(x);
                viewbox.y().set(y);
            } else {
                let (x, y) = calculate_canvas_position_from_wheel_event(
                    e.delta_x(),
                    e.delta_y(),
                    viewbox.x().get(),
                    viewbox.y().get(),
                    viewbox.width().get(),
                    viewbox.height().get(),
                    vb_scale(),
                    root.width().get_untracked().get(),
                    root.height().get_untracked().get(),
                );

                viewbox.x().set(x);
                viewbox.y().set(y);
            }
        }
    };

    view! {
        <div id="canvas">
            <svg
                on:mousedown=mousedown
                on:mouseup=mouseup
                on:mousemove=mousemove
                on:mouseleave=mouseleave
                on:wheel=wheel
                viewBox=move || {
                    format!(
                        "{} {} {} {}",
                        viewbox.x().get(),
                        viewbox.y().get(),
                        viewbox.width().get(),
                        viewbox.height().get(),
                    )
                }
                class=("cursor-grabbing", move || pan_drag.with(|c| c.is_some()))
            >
                <Graph />
            </svg>

            <div node_ref=portal_ref></div>
        </div>
    }
}

#[component]
fn Graph() -> impl IntoView {
    let graph = expect_context::<state::Graph>();
    let workspace_graph_state = expect_context::<state::workspace_graph::State>();

    let visibilities = workspace_graph_state.container_visiblity().read_only();
    let display_state = display::State::from(graph.root().clone(), graph.edges(), visibilities);
    provide_context(display_state.clone());

    view! { <GraphView root=graph.root().clone() /> }
}

#[component]
fn GraphView(root: state::graph::Node) -> impl IntoView {
    let graph = expect_context::<state::Graph>();
    let workspace_graph_state = expect_context::<state::WorkspaceGraph>();
    let display_state = expect_context::<state::Display>();
    let container_preview_height = expect_context::<ContainerPreviewHeight>();
    let viewbox = expect_context::<ViewboxState>();
    let portal_ref = expect_context::<PortalRef>();
    let create_child_ref = NodeRef::<html::Dialog>::new();
    let wrapper_node = NodeRef::<svg::Svg>::new();
    let container_node = NodeRef::<html::Div>::new();

    fn child_key(child: &state::graph::Node, graph: &state::Graph) -> String {
        child.properties().with_untracked(|properties| {
            properties
                .as_ref()
                .map(|properties| properties.rid().with_untracked(|rid| rid.to_string()))
                .unwrap_or_else(|_| graph.path(child).unwrap().to_string_lossy().to_string())
        })
    }

    let create_child_dialog_show = move |e: MouseEvent| {
        if e.button() != types::MouseButton::Primary {
            return;
        }

        let dialog = create_child_ref.get().unwrap();
        dialog.show_modal().unwrap();
    };

    let container_visibility = workspace_graph_state
        .container_visibility_get(&root)
        .unwrap();

    let children = graph.children(&root).unwrap().read_only();
    let siblings = {
        let graph = graph.clone();
        let root = root.clone();
        move || {
            graph
                .parent(&root)
                .map(|parent| parent.with(|parent| graph.children(parent).unwrap().read_only()))
        }
    };

    let display_data = display_state.find(&root).unwrap();

    let container_height = Signal::derive(move || {
        container_preview_height.with(|preview_height| CONTAINER_HEADER_HEIGHT + preview_height)
    });

    let width = Signal::derive({
        let width = display_data.width();
        move || {
            width.with(|width| {
                width.get() * (CONTAINER_WIDTH + PADDING_X_SIBLING) - PADDING_X_SIBLING
            })
        }
    });

    let height = {
        let root = root.clone();
        let root_height = display_data.height();
        let container_visibility = container_visibility.read_only();
        move || {
            let height = if container_visibility() {
                let height = root_height.with(|height| height.get());
                height * (container_height.get() + PADDING_Y_CHILDREN) - PADDING_Y_CHILDREN
                    + CANVAS_BUTTON_RADIUS
                    + CANVAS_BUTTON_STROKE
            } else {
                container_height.get() + PADDING_Y_CHILDREN - PADDING_Y_CHILDREN / 2
                    + CANVAS_BUTTON_RADIUS
                    + CANVAS_BUTTON_STROKE
            };

            cmp::max(height, 0)
        }
    };

    let x = {
        let sibling_width_until = display_state.sibling_width_until(&root).unwrap();
        Signal::derive(move || *sibling_width_until.read() * (CONTAINER_WIDTH + PADDING_X_SIBLING))
    };

    let y = {
        let graph = graph.clone();
        let root = root.clone();
        move || {
            if state::graph::Node::ptr_eq(&root, graph.root()) {
                0
            } else {
                container_height.get() + PADDING_Y_CHILDREN
            }
        }
    };

    let x_node = Signal::derive(move || (width.with(|width| (width - CONTAINER_WIDTH) / 2)));

    let _ = Effect::watch(
        container_visibility.read_only(),
        {
            let subtree_width = display_state.find(&root).unwrap().width();
            move |visible, visible_prev, _| {
                if let Some(visible_prev) = visible_prev {
                    if visible == visible_prev {
                        return;
                    }
                }
                let Some(node) = wrapper_node.get_untracked() else {
                    return;
                };

                let elm = node.dyn_ref::<web_sys::SvgGraphicsElement>().unwrap();
                let transform = elm.get_ctm().unwrap();
                let x0 = transform.e() as isize;
                let vb_x0 = viewbox.x().get_untracked();
                if *visible {
                    if vb_x0 > x0 {
                        let width = subtree_width.get_untracked().get();
                        let shift = (width - 1) * (CONTAINER_WIDTH + PADDING_X_SIBLING);

                        viewbox.x().set(vb_x0 + shift as isize);
                    }
                } else {
                    if vb_x0 > x0 {
                        let width = subtree_width.get_untracked().get();
                        let shift = (width - 1) * (CONTAINER_WIDTH + PADDING_X_SIBLING);

                        viewbox.x().set(vb_x0 - shift as isize);
                    }
                }
            }
        },
        false,
    );

    let children_widths = {
        let children = display_state.children(&root).unwrap();
        Signal::derive(move || {
            children
                .read()
                .iter()
                .map(|child| child.width())
                .collect::<Vec<_>>()
        })
    };

    view! {
        <svg node_ref=wrapper_node width=width height=height x=x y=y>
            <GraphEdges x_node children_widths container_visibility=container_visibility.clone() />
            <g class="group">
                <foreignObject width=CONTAINER_WIDTH height=container_height x=x_node y=0>
                    <ContainerView node_ref=container_node container=root.clone() />
                </foreignObject>
                <svg
                    x=move || {
                        x_node
                            .with(|x| {
                                x + CONTAINER_WIDTH / 2 - CANVAS_BUTTON_RADIUS
                                    - CANVAS_BUTTON_STROKE
                            })
                    }
                    y=move || container_height.get() - CANVAS_BUTTON_RADIUS - CANVAS_BUTTON_STROKE
                    width=(CANVAS_BUTTON_RADIUS + CANVAS_BUTTON_STROKE) * 2
                    height=(CANVAS_BUTTON_RADIUS + CANVAS_BUTTON_STROKE) * 2
                    on:mousedown=create_child_dialog_show
                    class="group-[:not(:hover)]:hidden cursor-pointer"
                >
                    <circle
                        r=CANVAS_BUTTON_RADIUS
                        cx=CANVAS_BUTTON_RADIUS + CANVAS_BUTTON_STROKE
                        cy=CANVAS_BUTTON_RADIUS + CANVAS_BUTTON_STROKE
                        class="stroke-black dark:stroke-white fill-white dark:fill-secondary-700 stroke-2"
                    ></circle>
                    <svg
                        x=CANVAS_BUTTON_STROKE as f64
                            + CANVAS_BUTTON_RADIUS as f64 * (1.0 - ICON_SCALE)
                        y=CANVAS_BUTTON_STROKE as f64
                            + CANVAS_BUTTON_RADIUS as f64 * (1.0 - ICON_SCALE)
                        width=CANVAS_BUTTON_RADIUS * 2
                        height=CANVAS_BUTTON_RADIUS * 2
                    >
                        <Icon
                            icon=components::icon::Add
                            width=(CANVAS_BUTTON_RADIUS as f64 * 2.0 * ICON_SCALE).to_string()
                            height=(CANVAS_BUTTON_RADIUS as f64 * 2.0 * ICON_SCALE).to_string()
                            attr:class="stroke-black dark:stroke-white stroke-2 linecap-round"
                        />
                    </svg>
                </svg>
            </g>
            <g class:hidden={
                let container_visibility = container_visibility.clone();
                move || !container_visibility()
            }>
                <For
                    each=children
                    key={
                        let graph = graph.clone();
                        move |child| child_key(child, &graph)
                    }
                    let:child
                >
                    <GraphView root=child />
                </For>
            </g>
        </svg>

        {move || {
            if let Some(mount) = portal_ref.get() {
                let mount = (*mount).clone();
                Either::Left(
                    view! {
                        <Portal mount clone:root>
                            <ModalDialog node_ref=create_child_ref clone:root>
                                <CreateChildContainer
                                    parent=root.clone()
                                    parent_ref=create_child_ref.clone()
                                />
                            </ModalDialog>
                        </Portal>
                    },
                )
            } else {
                Either::Right(())
            }
        }}
    }.into_any()
}

#[component]
fn GraphEdges(
    x_node: Signal<usize>,
    children_widths: Signal<Vec<ArcReadSignal<NonZeroUsize>>>,
    container_visibility: ArcRwSignal<bool>,
) -> impl IntoView {
    let container_preview_height = expect_context::<ContainerPreviewHeight>();

    let container_height = Signal::derive(move || {
        container_preview_height.with(|preview_height| CONTAINER_HEADER_HEIGHT + preview_height)
    });

    let line_points = {
        let x_node = x_node.clone();
        move |(start, end): (usize, usize)| {
            let parent_x = x_node() + CONTAINER_WIDTH / 2;
            let parent_y = container_height.get();
            let midway_y = cmp::max(
                container_height.get() as i32 + (PADDING_Y_CHILDREN / 2) as i32,
                0,
            );
            let child_y = container_height
                .get()
                .checked_add(PADDING_Y_CHILDREN)
                .unwrap();

            let child_x =
                ((start + end - 1) * (CONTAINER_WIDTH + PADDING_X_SIBLING) + CONTAINER_WIDTH) / 2;

            format!(
                "{},{} {},{} {},{} {},{}",
                parent_x, parent_y, parent_x, midway_y, child_x, midway_y, child_x, child_y,
            )
        }
    };

    let visibility_toggle_line_coordiantes = move || {
        let x = x_node.get() + CONTAINER_WIDTH / 2;
        let y1 = container_height.get();
        let y2 = cmp::max(container_height.get() + (PADDING_Y_CHILDREN / 2), 0);

        (x.to_string(), y1.to_string(), x.to_string(), y2.to_string())
    };

    let connector_lines_center = Signal::derive(move || {
        let x = x_node.get() + CONTAINER_WIDTH / 2;
        let y = cmp::max(container_height.get() + (PADDING_Y_CHILDREN / 2), 0);

        (x, y)
    });

    let toggle_container_visibility = {
        let container_visibility = container_visibility.clone();
        move |e: MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }
            e.stop_propagation();

            container_visibility.set(!container_visibility());
        }
    };

    let x_children = Signal::derive({
        let children_widths = children_widths.clone();
        move || {
            children_widths.with(|widths| {
                widths
                    .iter()
                    .scan((0_usize, 0_usize), |(start, end), width| {
                        *start = *end;
                        *end += width.get().get();
                        Some((*start, *end))
                    })
                    .collect::<Vec<_>>()
            })
        }
    });

    view! {
        <g>
            <g class:hidden={
                let container_visibility = container_visibility.clone();
                move || { !container_visibility() }
            }>
                {move || {
                    x_children
                        .with(|x_children| {
                            x_children
                                .iter()
                                .cloned()
                                .map(|x| {
                                    view! {
                                        <polyline
                                            fill="none"
                                            class="stroke-secondary-400 dark:stroke-secondary-500"
                                            points=move || line_points(x)
                                        ></polyline>
                                    }
                                })
                                .collect::<Vec<_>>()
                        })
                }}
            </g>
            <g>
                {
                    let container_visibility = container_visibility.clone();
                    let toggle_container_visibility = toggle_container_visibility.clone();
                    move || {
                        if children_widths.with(|children| children.len()) > 0 {
                            let (x1, y1, x2, y2) = visibility_toggle_line_coordiantes();
                            Either::Left(
                                view! {
                                    <line
                                        x1=x1
                                        y1=y1
                                        x2=x2
                                        y2=y2
                                        class="stroke-secondary-400 dark:stroke-secondary-500"
                                    ></line>

                                    {
                                        let container_visibility = container_visibility.clone();
                                        let toggle_container_visibility = toggle_container_visibility
                                            .clone();
                                        move || {
                                            let (cx, cy) = connector_lines_center.get();
                                            view! {
                                                <svg
                                                    x=move || cx - CANVAS_BUTTON_RADIUS - CANVAS_BUTTON_STROKE
                                                    y=move || cy - CANVAS_BUTTON_RADIUS - CANVAS_BUTTON_STROKE
                                                    width=(CANVAS_BUTTON_RADIUS + CANVAS_BUTTON_STROKE) * 2
                                                    height=(CANVAS_BUTTON_RADIUS + CANVAS_BUTTON_STROKE) * 2
                                                    on:mousedown=toggle_container_visibility.clone()
                                                    class="group cursor-pointer"
                                                >
                                                    <circle
                                                        r=TOGGLE_VIEW_INDICATOR_RADIUS
                                                        cx=CANVAS_BUTTON_RADIUS + CANVAS_BUTTON_STROKE
                                                        cy=CANVAS_BUTTON_RADIUS + CANVAS_BUTTON_STROKE
                                                        class="stroke-secondary-400 fill-secondary-400 dark:stroke-secondary-500 \
                                                        dark:fill-secondary-500 transition-opacity transition-delay-200 hover:opacity-0"
                                                    ></circle>
                                                    <g class="group-[:not(:hover)]:hidden">
                                                        <circle
                                                            r=CANVAS_BUTTON_RADIUS
                                                            cx=CANVAS_BUTTON_RADIUS + CANVAS_BUTTON_STROKE
                                                            cy=CANVAS_BUTTON_RADIUS + CANVAS_BUTTON_STROKE
                                                            class="stroke-black dark:stroke-white fill-white \
                                                            dark:fill-secondary-700 stroke-2 transition-opacity transition-delay-200 \
                                                            opacity:0 hover:opacity-1"
                                                        ></circle>

                                                        <svg
                                                            x=CANVAS_BUTTON_STROKE
                                                            y=CANVAS_BUTTON_STROKE
                                                            width=CANVAS_BUTTON_RADIUS * 2
                                                            height=CANVAS_BUTTON_RADIUS * 2
                                                        >
                                                            <Icon
                                                                icon=Signal::derive({
                                                                    let container_visibility = container_visibility.clone();
                                                                    move || {
                                                                        if container_visibility() {
                                                                            components::icon::Eye
                                                                        } else {
                                                                            components::icon::EyeClosed
                                                                        }
                                                                    }
                                                                })
                                                                width=(CANVAS_BUTTON_RADIUS * 2).to_string()
                                                                height=(CANVAS_BUTTON_RADIUS * 2).to_string()
                                                            />
                                                        </svg>
                                                    </g>
                                                </svg>
                                            }
                                        }
                                    }
                                },
                            )
                        } else {
                            Either::Right(())
                        }
                    }
                }
            </g>
        </g>
    }
}

#[component]
fn CreateChildContainer(
    parent: state::graph::Node,
    parent_ref: NodeRef<html::Dialog>,
) -> impl IntoView {
    use syre_local::project::container;

    let project = expect_context::<state::Project>();
    let graph = expect_context::<state::Graph>();
    let (name, set_name) = signal("".to_string());

    let create_child: Action<_, _> = Action::new_unsync({
        move |name: &String| {
            let graph = graph.clone();
            let project = project.rid().clone();
            let parent = parent.clone();
            let name = name.clone();
            async move {
                let parent_path = graph.path(&parent).unwrap();
                let path = parent_path.join(name);
                match commands::graph::create_child(project.get_untracked(), path).await {
                    Ok(_id) => {
                        // TODO: Buffer id to ensure it is published in an update.
                        let dialog = parent_ref.get_untracked().unwrap();
                        dialog.close();
                        set_name("".to_string());
                        Ok(())
                    }
                    Err(err) => match err {
                        container::error::Build::Load | container::error::Build::NotADirectory => {
                            unreachable!()
                        }
                        container::error::Build::Save(err) => {
                            tracing::error!(?err);
                            Err("Could not save the container.")
                        }
                        container::error::Build::AlreadyResource => {
                            Err("Folder is already a resource.")
                        }
                    },
                }
            }
        }
    });

    let close = move |_| {
        let dialog = parent_ref.get().unwrap();
        dialog.close();
        set_name("".to_string());
    };

    view! {
        <div class="px-4 py-2 rounded bg-white dark:bg-secondary-900">
            <h1 class="text-center text-lg pb-2 dark:text-white">"Create a new child"</h1>
            <form on:submit=move |e| {
                e.prevent_default();
                create_child.dispatch(name());
            }>
                <div class="pb-2">
                    <input
                        placeholder="Name"
                        on:input=move |e| set_name(event_target_value(&e))
                        prop:value=name
                        class="input-simple"
                        minlength="1"
                        autofocus
                        required
                    />
                    {move || {
                        create_child
                            .value()
                            .with(|value| {
                                if let Some(Err(error)) = value {
                                    tracing::debug!(?error);
                                    let msg = "Something went wrong.";
                                    Either::Left(view! { <div>{msg}</div> })
                                } else {
                                    Either::Right(())
                                }
                            })
                    }}

                </div>
                <div class="flex gap-2">
                    <button disabled=create_child.pending() class="btn btn-primary">
                        "Create"
                    </button>
                    <button
                        type="button"
                        on:mousedown=close
                        disabled=create_child.pending()
                        class="btn btn-secondary"
                    >
                        "Cancel"
                    </button>
                </div>
            </form>
        </div>
    }
}

#[component]
fn ContainerView(
    #[prop(optional)] node_ref: NodeRef<html::Div>,
    container: state::graph::Node,
) -> impl IntoView {
    move || {
        container.properties().with(|properties| {
            if properties.is_ok() {
                Either::Left(view! { <ContainerOk node_ref container=container.clone() /> })
            } else {
                Either::Right(view! { <ContainerErr node_ref container=container.clone() /> })
            }
        })
    }
}

/// A container whose properties are valid.
/// The state of analyses and assets is unknown.
#[component]
fn ContainerOk(
    #[prop(optional)] node_ref: NodeRef<html::Div>,
    container: state::graph::Node,
) -> impl IntoView {
    assert!(container
        .properties()
        .with_untracked(|properties| properties.is_ok()));

    let project = expect_context::<state::Project>();
    let graph = expect_context::<state::Graph>();
    let messages = expect_context::<types::Messages>();
    let context_menu_root = expect_context::<ContextMenuContainerRoot>();
    let context_menu_ok = expect_context::<ContextMenuContainerOk>();
    let context_menu_active_container =
        expect_context::<RwSignal<Option<ContextMenuActiveContainer>>>();
    let workspace_graph_state = expect_context::<state::WorkspaceGraph>();
    let (drag_over, set_drag_over) = signal(0);
    provide_context(Container(container.clone()));

    let title = {
        let properties = container.properties().read_only();
        move || properties.with(|properties| properties.as_ref().unwrap().name())
    };

    let rid = {
        let properties = container.properties().read_only();
        move || {
            properties.with(|properties| {
                properties
                    .as_ref()
                    .unwrap()
                    .rid()
                    .with(|rid| rid.to_string())
            })
        }
    };

    let path = {
        let graph = graph.clone();
        let container = container.clone();
        move || {
            graph
                .path(&container)
                .unwrap()
                .to_string_lossy()
                .to_string()
        }
    };

    let selection_resource = container
        .properties()
        .with_untracked(|properties| {
            let properties = properties.as_ref().unwrap();
            properties.rid().with_untracked(|container_id| {
                workspace_graph_state
                    .selection_resources()
                    .get(container_id)
            })
        })
        .unwrap();

    let mousedown = {
        let rid = container
            .properties()
            .with_untracked(|properties| properties.as_ref().unwrap().rid().read_only());
        let selection_resources = workspace_graph_state.selection_resources().clone();
        move |e: MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }
            e.stop_propagation();

            let action = rid.with_untracked(|rid| {
                selection_resources.selected().with_untracked(|selected| {
                    interpret_resource_selection_action(rid, selected, e.shift_key())
                })
            });
            match action {
                SelectionAction::Unselect => {
                    rid.with_untracked(|rid| selection_resources.set(rid, false).unwrap())
                }
                SelectionAction::Select => {
                    rid.with_untracked(|rid| selection_resources.set(rid, true).unwrap())
                }
                SelectionAction::SelectOnly => {
                    rid.with_untracked(|rid| selection_resources.select_only(rid).unwrap())
                }
                SelectionAction::Clear => selection_resources.clear(),
            }
        }
    };

    let highlight = move || selection_resource.get() || drag_over() > 0;
    let contextmenu = {
        let graph = graph.clone();
        let container = container.clone();
        let context_menu_root = context_menu_root.clone();
        let context_menu_ok = context_menu_ok.clone();

        move |e: MouseEvent| {
            e.prevent_default();

            let is_root = Arc::ptr_eq(&container, graph.root());
            context_menu_active_container.update(|active_container| {
                let _ = active_container.insert(container.clone().into());
            });
            let context_menu_root = context_menu_root.clone();
            let context_menu_ok = context_menu_ok.clone();
            spawn_local(async move {
                if is_root {
                    context_menu_root.popup().await.unwrap();
                } else {
                    context_menu_ok.popup().await.unwrap();
                };
            });
        }
    };

    let drop = {
        let project = project.rid().read_only();
        let graph = graph.clone();
        let container = container.clone();
        let messages = messages.clone();
        move |e: DragEvent| {
            e.prevent_default();
            set_drag_over(0);

            let data = e.data_transfer().unwrap();
            let data = data.get_data(common::APPLICATION_JSON).unwrap();
            let Ok(action) = serde_json::from_str::<actions::container::Action>(&data) else {
                tracing::warn!("invalid action: `{}`", data);
                return;
            };
            match action {
                actions::container::Action::AddAnalysisAssociation(analysis) => {
                    handle_container_action_add_analysis_accociation(
                        analysis,
                        container.clone(),
                        &graph,
                        project.get_untracked(),
                        messages.clone(),
                    )
                }
            }
        }
    };

    let wheel = move |e: WheelEvent| {
        // NB: Allow zoom events to propogate, but not wheel scroll events.
        if !e.ctrl_key() {
            e.stop_propagation();
        }
    };

    view! {
        <div
            on:mousedown=mousedown
            on:contextmenu=contextmenu
            on:dragenter=move |_| set_drag_over.update(|count| *count += 1)
            on:dragleave=move |_| set_drag_over.update(|count| *count -= 1)
            on:dragenter_windows=move |_: web_sys::Event| set_drag_over.update(|count| *count += 1)
            on:dragleave_windows=move |_: web_sys::Event| set_drag_over.update(|count| *count -= 1)
            on:dragover=move |e| e.prevent_default()
            on:drop=drop
            class=(
                ["border-2", "border-secondary-900", "dark:border-secondary-100"],
                {
                    let highlight = highlight.clone();
                    move || !highlight()
                },
            )
            class=(["border-4", "border-primary-700"], highlight.clone())
            class="h-full cursor-pointer rounded bg-white dark:bg-secondary-700"
            data-resource=DATA_KEY_CONTAINER
            data-rid=rid
            data-path=path
        >
            // NB: inner div with node ref is used for resizing observer to obtain content height.
            <div node_ref=node_ref class="h-full flex flex-col">
                <div class="pb-2 text-center text-lg">
                    <span class="font-primary">{title}</span>
                </div>
                <div
                    on:wheel=wheel
                    class="grow overflow-y-auto scrollbar-thin scrollbar-thumb-rounded-full scrollbar-track-rounded-full"
                >
                    <ContainerPreview
                        properties=container.properties().read_only()
                        assets=container.assets().read_only()
                        analyses=container.analyses().read_only()
                    />
                </div>
            </div>
        </div>
    }
}

#[component]
fn ContainerPreview(
    properties: ReadSignal<state::container::PropertiesState>,
    analyses: ReadSignal<state::container::AnalysesState>,
    assets: ReadSignal<state::container::AssetsState>,
) -> impl IntoView {
    assert!(properties.with_untracked(|properties| properties.is_ok()));
    assert!(analyses.with_untracked(|analyses| analyses.is_ok()));
    let workspace_state = expect_context::<state::Workspace>();
    let state = workspace_state.preview().clone();

    let kind =
        properties.with_untracked(|properties| properties.as_ref().unwrap().kind().read_only());

    let description = properties
        .with_untracked(|properties| properties.as_ref().unwrap().description().read_only());

    let tags =
        properties.with_untracked(|properties| properties.as_ref().unwrap().tags().read_only());

    let metadata =
        properties.with_untracked(|properties| properties.as_ref().unwrap().metadata().read_only());

    view! {
        <div class="overflow-y-auto scrollbar">
            <Assets assets />

            <Analyses analyses=analyses
                .with_untracked(|analyses| analyses.as_ref().unwrap().read_only()) />

            <div class="py border-t border-secondary-200 dark:border-secondary-800">
                <div class:hidden=move || { state.with(|preview| !preview.kind) } class="px-2">
                    {move || kind().unwrap_or("(no type)".to_string())}
                </div>
                <div
                    class:hidden=move || { state.with(|preview| !preview.description) }
                    class="px-2"
                >
                    {move || description().unwrap_or("(no description)".to_string())}
                </div>
                <div class:hidden=move || { state.with(|preview| !preview.tags) } class="px-2">
                    {move || {
                        tags.with(|tags| {
                            if tags.is_empty() { "(no tags)".to_string() } else { tags.join(", ") }
                        })
                    }}

                </div>
                <Metadata metadata />
            </div>
        </div>
    }
}

#[component]
fn Assets(assets: ReadSignal<state::container::AssetsState>) -> impl IntoView {
    let messages = expect_context::<types::Messages>();
    move || {
        assets.with(|assets| match assets {
            Ok(assets) => Either::Left(view! { <AssetsPreview assets=assets.read_only() /> }),
            Err(err) => {
                tracing::error!(?err);
                messages.update(|messages| {
                    // TODO: Get path of container.
                    let mut msg = types::message::Builder::error("Could not load assets.");
                    msg.body(format!("{err:?}"));
                    messages.push(msg.build());
                });

                Either::Right(view! { <div class="text-center">"(assets error)"</div> })
            }
        })
    }
}

#[component]
fn AssetsPreview(assets: ReadSignal<Vec<state::Asset>>) -> impl IntoView {
    let workspace_state = expect_context::<state::Workspace>();
    let assets_sorted = move || {
        let mut assets = assets.get();
        assets.sort_by_key(|asset| {
            asset
                .name()
                .get()
                .unwrap_or_else(|| asset.path().get().to_string_lossy().to_string())
                .to_lowercase()
        });
        assets
    };

    view! {
        <div
            class:hidden=move || workspace_state.preview().with(|preview| !preview.assets)
            class="pb"
        >
            <Show
                when=move || assets.with(|assets| !assets.is_empty())
                fallback=|| view! { <NoData /> }
            >
                <For each=assets_sorted key=|asset| asset.rid().get() let:asset>
                    <Asset asset />
                </For>
            </Show>
        </div>
    }
}

#[component]
fn NoData() -> impl IntoView {
    view! { <div class="px-2">"(no data)"</div> }
}

#[component]
fn Asset(asset: state::Asset) -> impl IntoView {
    let project = expect_context::<state::Project>();
    let graph = expect_context::<state::Graph>();
    let container = expect_context::<Container>();
    let workspace_graph_state = expect_context::<state::WorkspaceGraph>();
    let context_menu = expect_context::<ContextMenuAsset>();
    let context_menu_active_asset = expect_context::<RwSignal<Option<ContextMenuActiveAsset>>>();
    let messages = expect_context::<types::Messages>();

    let rid = {
        let rid = asset.rid();
        move || rid.with(|rid| rid.to_string())
    };

    let selection_resource = asset
        .rid()
        .with_untracked(|rid| workspace_graph_state.selection_resources().get(rid))
        .unwrap();

    let title = asset_title_closure(&asset);

    let mousedown = {
        let selection_resources = workspace_graph_state.selection_resources().clone();
        let rid = asset.rid().read_only();
        move |e: MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }
            e.stop_propagation();

            let action = rid.with_untracked(|rid| {
                selection_resources.selected().with_untracked(|selected| {
                    interpret_resource_selection_action(rid, selected, e.shift_key())
                })
            });
            match action {
                SelectionAction::Unselect => {
                    rid.with_untracked(|rid| selection_resources.set(rid, false).unwrap())
                }
                SelectionAction::Select => {
                    rid.with_untracked(|rid| selection_resources.set(rid, true).unwrap())
                }
                SelectionAction::SelectOnly => {
                    rid.with_untracked(|rid| selection_resources.select_only(rid).unwrap())
                }
                SelectionAction::Clear => selection_resources.clear(),
            }
        }
    };

    let contextmenu = {
        let asset = asset.clone();
        move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();

            context_menu_active_asset.update(|active_asset| {
                let _ = active_asset.insert(asset.rid().get_untracked().into());
            });

            let menu = context_menu.clone();
            spawn_local(async move {
                menu.popup().await.unwrap();
            });
        }
    };

    let remove: Action<_, _> = Action::new_unsync({
        let asset = asset.clone();
        let container = container.clone();
        let graph = graph.clone();
        let project = project.rid().read_only();
        let messages = messages.clone();

        move |_| {
            let asset = asset.clone();
            let container = container.clone();
            let graph = graph.clone();
            let project = project.clone();
            let messages = messages.clone();

            async move {
                let container_path = graph.path(&container).unwrap();
                if let Err(err) = remove_asset(
                    project.get_untracked(),
                    container_path,
                    asset.path().get_untracked(),
                )
                .await
                {
                    tracing::error!(?err);
                    let mut msg = types::message::Builder::error("Could not remove asset file");
                    msg.body(format!("{err:?}"));
                    messages.update(|messages| messages.push(msg.build()));
                };
            }
        }
    });

    let remove_asset = move |e: MouseEvent| {
        if e.button() != types::MouseButton::Primary {
            return;
        }

        e.stop_propagation();
        remove.dispatch(());
    };

    let icon = Signal::derive({
        let path = asset.path().read_only();
        move || path.with(|path| components::icon::file_type_icon(path))
    });
    let icon_color = Signal::derive({
        let path = asset.path().read_only();
        move || path.with(|path| components::icon::file_type_icon_color(path))
    });
    let icon_class = Signal::derive(move || {
        icon_color.with(|color| format!("inline-flex items-center {color}"))
    });

    view! {
        <div
            on:mousedown=mousedown
            on:contextmenu=contextmenu
            title=asset_title_closure(&asset)
            class=(["bg-secondary-300", "dark:bg-secondary-600"], selection_resource.clone())
            class="flex gap-2 cursor-pointer px-2 py-0.5 border border-transparent \
            hover:border-secondary-600 dark:hover:border-secondary-400"
            data-resource=DATA_KEY_ASSET
            data-rid=rid
        >
            <div class="grow inline-flex gap-1 items-center">
                <span class=icon_class>
                    <Icon icon />
                </span>
                <TruncateLeft class="grow" inner_class="align-middle">
                    {title}
                </TruncateLeft>
            </div>
            <div>
                <button
                    on:mousedown=remove_asset
                    class="align-middle rounded-sm hover:bg-secondary-200 dark:hover:bg-secondary-800"
                >
                    <Icon icon=components::icon::Remove />
                </button>
            </div>
        </div>
    }
}

#[component]
fn Analyses(analyses: ReadSignal<Vec<state::AnalysisAssociation>>) -> impl IntoView {
    let workspace_state = expect_context::<state::Workspace>();
    let analyses_sorted = move || {
        let mut analyses = analyses.get();
        // TODO: Sort by title as least significant.
        analyses.sort_by_key(|analysis| (analysis.priority().get(), analysis.autorun().get()));
        analyses
    };

    view! {
        <div
            class:hidden=move || workspace_state.preview().with(|preview| !preview.analyses)
            class="py border-t border-secondary-200 dark:border-secondary-800"
        >
            <Show
                when=move || analyses.with(|analyses| !analyses.is_empty())
                fallback=|| view! { <NoAnalyses /> }
            >
                <For
                    each=analyses_sorted
                    key=|association| association.analysis().clone()
                    let:association
                >
                    <AnalysisAssociation association />
                </For>
            </Show>
        </div>
    }
}

#[component]
fn NoAnalyses() -> impl IntoView {
    view! { <div class="px-2">"(no analyses)"</div> }
}

#[component]
fn AnalysisAssociation(association: state::AnalysisAssociation) -> impl IntoView {
    let project = expect_context::<state::Project>();
    let graph = expect_context::<state::Graph>();
    let container = expect_context::<Container>();
    let messages = expect_context::<types::Messages>();

    let title = {
        let association = association.clone();
        let project = project.clone();
        move || {
            project.analyses().with(|analyses| {
                let db::state::DataResource::Ok(analyses) = analyses else {
                    return None;
                };

                analyses.with(|analyses| {
                    analyses.iter().find_map(|analysis| {
                        analysis.properties().with(|properties| {
                            if properties.id() != association.analysis() {
                                return None;
                            }

                            let title = match properties {
                                local::types::AnalysisKind::Script(script) => {
                                    if let Some(name) = script.name.as_ref() {
                                        name.clone()
                                    } else {
                                        script.path.to_string_lossy().to_string()
                                    }
                                }

                                local::types::AnalysisKind::ExcelTemplate(template) => {
                                    if let Some(name) = template.name.as_ref() {
                                        name.clone()
                                    } else {
                                        template.template.path.to_string_lossy().to_string()
                                    }
                                }
                            };

                            Some(title)
                        })
                    })
                })
            })
        }
    };

    let hover_title = {
        let association = association.clone();
        let title = title.clone();
        move || {
            if title().is_none() {
                Some(association.analysis().to_string())
            } else {
                None
            }
        }
    };

    let update_associations: Action<_, _> = Action::new_unsync({
        let project = project.clone();
        let container = container.clone();
        let messages = messages.clone();
        move |associations: &Vec<AnalysisAssociation>| {
            let project = project.rid().get_untracked();
            let container_path = graph.path(&container).unwrap();
            let messages = messages.clone();
            let associations = associations.clone();
            async move {
                if let Err(err) = commands::container::update_analysis_associations(
                    project,
                    container_path,
                    associations,
                )
                .await
                {
                    tracing::error!(?err);
                    let mut msg =
                        types::message::Builder::error("Could not update analysis associations.");
                    msg.body(format!("{err:?}"));
                    messages.update(|messages| messages.push(msg.build()));
                }
            }
        }
    });

    let autorun_toggle = {
        let association = association.clone();
        let container = container.clone();

        move |e: MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }
            e.stop_propagation();

            let mut associations = container.analyses().with_untracked(|analyses| {
                analyses.as_ref().unwrap().with_untracked(|associations| {
                    associations
                        .iter()
                        .map(|association| association.as_association())
                        .collect::<Vec<_>>()
                })
            });
            let assoc = associations
                .iter_mut()
                .find(|analysis| analysis.analysis() == association.analysis())
                .unwrap();
            assoc.autorun = !assoc.autorun;

            update_associations.dispatch(associations);
        }
    };

    let remove_association = {
        let association = association.clone();
        let container = container.clone();

        move |e: MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }
            e.stop_propagation();

            let mut associations = container.analyses().with_untracked(|analyses| {
                analyses.as_ref().unwrap().with_untracked(|associations| {
                    associations
                        .iter()
                        .map(|association| association.as_association())
                        .collect::<Vec<_>>()
                })
            });
            associations.retain(|assoc| assoc.analysis() != association.analysis());

            update_associations.dispatch(associations);
        }
    };

    view! {
        <div class="flex gap-2 px-2">
            <div class="inline-flex grow">
                <div title=hover_title class="grow">
                    {move || title().unwrap_or("(no title)".to_string())}
                </div>
                <div class="inline-flex gap-1">
                    <span>"(" {association.priority()} ")"</span>
                    <span on:mousedown=autorun_toggle class="inline-flex items-center">
                        {move || {
                            if association.autorun().get() {
                                view! { <Icon icon=icondata::BsStarFill /> }
                            } else {
                                view! { <Icon icon=icondata::BsStar /> }
                            }
                        }}

                    </span>
                </div>
            </div>
            <div>
                <button
                    on:mousedown=remove_association
                    class="align-middle rounded-sm hover:bg-secondary-200 dark:hover:bg-secondary-800"
                >
                    <Icon icon=components::icon::Remove />
                </button>
            </div>
        </div>
    }
}

#[component]
fn Metadata(metadata: ReadSignal<state::Metadata>) -> impl IntoView {
    let workspace_state = expect_context::<state::Workspace>();
    let metadata_sorted = move || {
        let mut metadata = metadata.get();
        metadata.sort_by_key(|(key, _)| key.clone().to_lowercase());
        metadata
    };

    view! {
        <div class:hidden=move || { workspace_state.preview().with(|preview| !preview.metadata) }>
            <Show
                when=move || metadata.with(|metadata| !metadata.is_empty())
                fallback=|| view! { <NoMetadata /> }
            >
                <For each=metadata_sorted key=|(key, _)| key.clone() let:datum>
                    <div class="px-2">
                        <span>
                            <strong>{datum.0}</strong>
                            ": "
                        </span>
                        <span>{move || datum.1.with(|value| value.to_string())}</span>
                    </div>
                </For>
            </Show>
        </div>
    }
}

#[component]
fn NoMetadata() -> impl IntoView {
    view! { <div class="px-2">"(no metadata)"</div> }
}

#[component]
fn ContainerErr(
    #[prop(optional)] node_ref: NodeRef<html::Div>,
    container: state::graph::Node,
) -> impl IntoView {
    let graph = expect_context::<state::Graph>();
    let context_menu_root = expect_context::<ContextMenuContainerRoot>();
    let context_menu_err = expect_context::<ContextMenuContainerErr>();
    let context_menu_active_container =
        expect_context::<RwSignal<Option<ContextMenuActiveContainer>>>();

    let path = {
        let graph = graph.clone();
        let container = container.clone();
        move || {
            graph
                .path(&container)
                .unwrap()
                .to_string_lossy()
                .to_string()
        }
    };

    let show_details = RwSignal::new(false);
    let error = {
        let properties = container.properties().read_only();
        move || {
            properties.with(|properties| {
                let db::state::DataResource::Err(error) = properties else {
                    panic!("invalid state");
                };

                format!("{error:?}")
            })
        }
    };

    let contextmenu = {
        let graph = graph.clone();
        let container = container.clone();
        let context_menu_root = context_menu_root.clone();
        let context_menu_err = context_menu_err.clone();

        move |e: MouseEvent| {
            e.prevent_default();

            let is_root = Arc::ptr_eq(&container, graph.root());
            context_menu_active_container.update(|active_container| {
                let _ = active_container.insert(container.clone().into());
            });
            let context_menu_root = context_menu_root.clone();
            let context_menu_err = context_menu_err.clone();
            spawn_local(async move {
                if is_root {
                    context_menu_root.popup().await.unwrap();
                } else {
                    context_menu_err.popup().await.unwrap();
                };
            });
        }
    };

    view! {
        <div
            on:contextmenu=contextmenu
            node_ref=node_ref
            class="h-full flex flex-col border-4 border-syre-red-600 rounded bg-white dark:bg-secondary-700"
            data-resource=DATA_KEY_CONTAINER
            data-path=path
        >
            <div class="pb-2 text-center text-lg">
                {move || container.name().with(|name| name.to_string_lossy().to_string())}
            </div>

            <div class="grow">
                <div class="text-center relative border-syre-red-600">
                    <strong>"Error"</strong>
                    <div class="absolute top-0 right-2">
                        <ToggleExpand expanded=show_details />
                    </div>
                </div>

                <div class:hidden=move || !show_details() class="grow scroll-y-auto px-2">
                    {error}
                </div>
            </div>
        </div>
    }
}

struct ViewboxDimensions {
    x: isize,
    y: isize,
    width: usize,
    height: usize,
}

/// Calculate new canvas viewbox dimensions.
///
/// # Arguments
/// + `e`: Triggering event.
/// + `width`: Viewbox width.
/// + `height`: Viewbox height.
fn calculate_canvas_viewbox_scaling(
    e: WheelEvent,
    x: isize,
    y: isize,
    width: usize,
    height: usize,
) -> ViewboxDimensions {
    let dy = e.delta_y();
    let scale = if dy < 0.0 {
        VB_SCALE_ENLARGE
    } else if dy > 0.0 {
        VB_SCALE_REDUCE
    } else {
        return ViewboxDimensions {
            x,
            y,
            width,
            height,
        };
    };

    let width_new = (width as f32 * scale).round() as usize;
    let height_new = (height as f32 * scale).round() as usize;
    let width_new = common::clamp(
        width_new.try_into().unwrap(),
        VB_WIDTH_MIN.try_into().unwrap(),
        VB_WIDTH_MAX.try_into().unwrap(),
    );
    let height_new = common::clamp(
        height_new.try_into().unwrap(),
        VB_HEIGHT_MIN.try_into().unwrap(),
        VB_HEIGHT_MAX.try_into().unwrap(),
    );

    let dw = width_new as isize - width as isize;
    let dh = height_new as isize - height as isize;
    let x_new = x - dw / 2;
    let y_new = y - dh / 2;

    ViewboxDimensions {
        x: x_new,
        y: y_new,
        width: width_new,
        height: height_new,
    }
}

/// Calculates new canvase viewbox position.
///
/// # Arguments
/// + `dx`: Wheel shift in x.
/// + `dy`: Wheel shift in y.
/// + `x`: Viewbox x position.
/// + `y`: Viewbox y position.
/// + `width``: Viewbox width.
/// + `height`: Viewbox height.
/// + `scale`: Viewbox scale.
/// + `graph_width`: Graph width.
/// + `graph_height`: Graph height.
///
/// # Returns
/// Viewbox (x, y).  
fn calculate_canvas_position_from_wheel_event(
    dx: f64,
    dy: f64,
    x: isize,
    y: isize,
    width: usize,
    height: usize,
    scale: f64,
    graph_width: usize,
    graph_height: usize,
) -> (isize, isize) {
    let x = x + (dx / scale) as isize;
    let y = y + (dy / scale) as isize;
    let x_max = (graph_width * (CONTAINER_WIDTH + PADDING_X_SIBLING)) as isize - width as isize / 2;
    let y_max = cmp::max(
        (graph_height * (MAX_CONTAINER_HEIGHT + PADDING_Y_CHILDREN)) as isize - height as isize / 2,
        0,
    );
    let x = common::clamp(
        x,
        -TryInto::<isize>::try_into(width / 2).unwrap(),
        x_max.try_into().unwrap(),
    );
    let y = common::clamp(
        y,
        -TryInto::<isize>::try_into(height / 2).unwrap(),
        y_max.try_into().unwrap(),
    );
    (x, y)
}

fn handle_container_action_add_analysis_accociation(
    analysis: ResourceId,
    container: state::graph::Node,
    graph: &state::Graph,
    project: ResourceId,
    messages: types::Messages,
) {
    let associations = container.analyses().read_only();
    let Some(mut associations) = associations.with_untracked(|associations| {
        let db::state::DataResource::Ok(associations) = associations else {
            panic!("invalid state");
        };

        if associations.with(|associations| {
            associations
                .iter()
                .any(|association| *association.analysis() == analysis)
        }) {
            None
        } else {
            Some(
                associations
                    .get_untracked()
                    .into_iter()
                    .map(|assoc| assoc.as_association())
                    .collect::<Vec<_>>(),
            )
        }
    }) else {
        return;
    };
    associations.push(AnalysisAssociation::new(analysis));

    let project = project.clone();
    let container = graph.path(&container).unwrap();
    spawn_local(async move {
        if let Err(err) =
            commands::container::update_analysis_associations(project, container, associations)
                .await
        {
            tracing::error!(?err);
            let mut msg = types::message::Builder::error("Could not save container.");
            msg.body(format!("{err:?}"));
            messages.update(|messages| messages.push(msg.build()));
        }
    });
}

async fn handle_context_menu_container_root_events(
    project: state::Project,
    graph: state::Graph,
    messages: types::Messages,
    container_open: Channel<String>,
) {
    let mut container_open = container_open.fuse();
    loop {
        futures::select! {
            event = container_open.next() => match event {
                None => continue,
                Some(_id) => {
                   handle_context_menu_container_events_container_open(graph.root(), &project, &graph, messages).await
                }
            },
        }
    }
}

async fn handle_context_menu_container_ok_events(
    project: state::Project,
    graph: state::Graph,
    messages: types::Messages,
    context_menu_active_container: ReadSignal<Option<ContextMenuActiveContainer>>,
    container_open: Channel<String>,
    container_duplicate: Channel<String>,
    container_trash: Channel<String>,
) {
    let mut container_open = container_open.fuse();
    let mut container_duplicate = container_duplicate.fuse();
    let mut container_trash = container_trash.fuse();
    loop {
        futures::select! {
            event = container_open.next() => match event {
                None => continue,
                Some(_id) => {
                    let container = context_menu_active_container.get_untracked().unwrap();
                    handle_context_menu_container_events_container_open(&*container, &project, &graph, messages).await
                }
            },

            event = container_duplicate.next() => match event {
                None => continue,
                Some(_id) => {
                    handle_context_menu_container_ok_events_container_duplicate(context_menu_active_container, &project, &graph, messages).await

                }
            },

            event = container_trash.next() => match event {
                None => continue,
                Some(_id) => {
                    let container = context_menu_active_container.get_untracked().unwrap();
                    let container_path = graph.path(&container).unwrap();
                    let path = common::normalize_path_sep(container_path);
                    let project_id = project.rid().get_untracked();
                    if let Err(err) =  trash_container(project_id, path).await {
                            messages.update(|messages|{
                                let mut msg = types::message::Builder::error("Could not trash container.");
                                msg.body(format!("{err:?}"));
                                messages.push(msg.build());
                            });
                        }
                }
            }
        }
    }
}

async fn handle_context_menu_container_err_events(
    project: state::Project,
    graph: state::Graph,
    messages: types::Messages,
    context_menu_active_container: ReadSignal<Option<ContextMenuActiveContainer>>,
    container_open: Channel<String>,
    container_trash: Channel<String>,
) {
    let mut container_open = container_open.fuse();
    let mut container_trash = container_trash.fuse();
    loop {
        futures::select! {
            event = container_open.next() => match event {
                None => continue,
                Some(_id) => {
                    let container = context_menu_active_container.get_untracked().unwrap();
                    handle_context_menu_container_events_container_open(&*container, &project, &graph, messages).await
                }
            },

            event = container_trash.next() => match event {
                None => continue,
                Some(_id) => {
                    let container = context_menu_active_container.get_untracked().unwrap();
                    let container_path = graph.path(&container).unwrap();
                    let path = common::normalize_path_sep(container_path);
                    let project_id = project.rid().get_untracked();
                    if let Err(err) =  trash_container(project_id, path).await {
                            messages.update(|messages|{
                                let mut msg = types::message::Builder::error("Could not trash container.");
                                msg.body(format!("{err:?}"));
                                messages.push(msg.build());
                            });
                        }
                }
            }
        }
    }
}

async fn handle_context_menu_container_events_container_open(
    container: &state::graph::Node,
    project: &state::Project,
    graph: &state::Graph,
    messages: types::Messages,
) {
    let data_root = project
        .path()
        .get_untracked()
        .join(project.properties().data_root().get_untracked());

    let container_path = graph.path(container).unwrap();
    let path = common::container_system_path(data_root, container_path);

    if let Err(err) = commands::fs::open_file(path).await {
        messages.update(|messages| {
            let mut msg = types::message::Builder::error("Could not open container folder.");
            msg.body(format!("{err:?}"));
            messages.push(msg.build());
        });
    }
}

async fn handle_context_menu_container_ok_events_container_duplicate(
    active_container: ReadSignal<Option<ContextMenuActiveContainer>>,
    project: &state::Project,
    graph: &state::Graph,
    messages: types::Messages,
) {
    let container = active_container.get_untracked().unwrap();
    let container_path = graph.path(&container).unwrap();
    let path = common::normalize_path_sep(&container_path);
    let project_id = project.rid().get_untracked();
    let data_root = project
        .path()
        .get_untracked()
        .join(project.properties().data_root().get_untracked());

    let system_path = common::container_system_path(data_root, &path);
    let system_path = common::normalize_path_sep(system_path);
    let size = match commands::fs::file_size(vec![system_path]).await {
        Ok(size) => {
            assert_eq!(size.len(), 1);
            size[0]
        }
        Err(err) => {
            tracing::error!(?err);
            0
        }
    };

    if size > super::common::FS_RESOURCE_ACTION_NOTIFY_THRESHOLD {
        let msg = types::message::Builder::info(format!("Duplicating tree {container_path:?}."));
        let msg = msg.build();
        messages.update(|messages| messages.push(msg))
    }

    match duplicate_container(project_id, path).await {
        Ok(_) => {
            if size > super::common::FS_RESOURCE_ACTION_NOTIFY_THRESHOLD {
                let msg = types::message::Builder::success(format!(
                    "Completed duplicating {container_path:?}."
                ));
                let msg = msg.build();
                messages.update(|messages| {
                    messages.push(msg);
                });
            }
        }

        Err(err) => {
            let mut msg =
                types::message::Builder::error(format!("Could not duplicate {container_path:?}."));
            msg.body(format!("{err:?}"));
            let msg = msg.build();
            messages.update(|messages| {
                messages.push(msg);
            });
        }
    }
}

async fn handle_context_menu_asset_events(
    project: state::Project,
    graph: state::Graph,
    messages: types::Messages,
    context_menu_active_asset: ReadSignal<Option<ContextMenuActiveAsset>>,
    asset_open: Channel<String>,
) {
    let mut asset_open = asset_open.fuse();
    loop {
        futures::select! {
            event = asset_open.next() => match event {
                None => continue,
                Some(_id) => {
                    let data_root = project
                        .path()
                        .get_untracked()
                        .join(project.properties().data_root().get_untracked());

                    let asset = context_menu_active_asset.get_untracked().unwrap();
                    let container = graph.find_by_asset_id(&*asset).unwrap();
                    let container_path = graph.path(&container).unwrap();
                    let container_path = common::container_system_path(data_root, container_path);
                    let db::state::DataResource::Ok(assets) = container.assets().get_untracked() else {
                        panic!("invalid state");
                    };
                    let asset_path = assets.with_untracked(|assets| assets.iter().find_map(|container_asset| {
                         container_asset.rid().with_untracked(|rid| if *rid == *asset {
                            Some(container_asset.path().get_untracked())
                        } else {
                            None
                        })
                    })).unwrap();
                    let path = container_path.join(asset_path);

                    if let Err(err) = commands::fs::open_file(path)
                        .await {
                            messages.update(|messages|{
                                let mut msg = types::message::Builder::error("Could not open asset file.");
                                msg.body(format!("{err:?}"));
                            messages.push(msg.build());
                        });
                    }
            }
            }
        }
    }
}

async fn duplicate_container(
    project: ResourceId,
    container: PathBuf,
) -> Result<(), lib::command::graph::error::duplicate::Error> {
    #[derive(Serialize)]
    struct Args {
        project: ResourceId,
        container: PathBuf,
    }

    tauri_sys::core::invoke_result("container_duplicate", Args { project, container }).await
}

async fn trash_container(project: ResourceId, container: PathBuf) -> Result<(), io::ErrorKind> {
    #[derive(Serialize)]
    struct Args {
        project: ResourceId,
        container: PathBuf,
    }

    tauri_sys::core::invoke_result::<(), lib::command::error::IoErrorKind>(
        "container_trash",
        Args { project, container },
    )
    .await
    .map_err(|err| err.into())
}

async fn remove_asset(
    project: ResourceId,
    container: PathBuf,
    asset: PathBuf,
) -> Result<(), io::ErrorKind> {
    #[derive(Serialize)]
    struct Args {
        project: ResourceId,
        container: PathBuf,
        asset: PathBuf,
    }

    tauri_sys::core::invoke_result::<(), lib::command::error::IoErrorKind>(
        "asset_remove_file",
        Args {
            project,
            container,
            asset,
        },
    )
    .await
    .map_err(|err| err.into())
}

mod display {
    use super::state;
    use leptos::prelude::*;
    use std::{num::NonZeroUsize, sync::Arc};

    // TODO: May be unnecesasry to wrap in `Arc`.
    type Node = Arc<Data>;

    #[derive(Clone, Debug)]
    pub struct Data {
        container: state::graph::Node,
        visibility: ArcReadSignal<bool>,
        children: ArcRwSignal<Vec<(state::graph::Node, Signal<NonZeroUsize>)>>,

        // TODO: Use trigger?
        /// Updates internal state when `children` changes.
        _update: Effect<LocalStorage>,
    }

    impl Data {
        fn from(
            container: state::graph::Node,
            children: ReadSignal<Vec<state::graph::Node>>,
            visibility: ArcReadSignal<bool>,
            nodes: ArcReadSignal<Vec<Node>>,
        ) -> Self {
            let state_children = children;
            let children = ArcRwSignal::new(vec![]);

            let update = Effect::new({
                let children = children.clone();
                move |_| {
                    let (removed, added) = state_children.with(|state_children| {
                        let removed = children.with_untracked(|children| {
                            children
                                .iter()
                                .filter(|(child, _)| {
                                    !state_children
                                        .iter()
                                        .any(|state_child| Arc::ptr_eq(state_child, child))
                                })
                                .map(|(child, _)| child)
                                .cloned()
                                .collect::<Vec<_>>()
                        });

                        let added = state_children
                            .iter()
                            .filter(|state_child| {
                                children.with_untracked(|children| {
                                    !children
                                        .iter()
                                        .any(|(child, _)| Arc::ptr_eq(child, state_child))
                                })
                            })
                            .cloned()
                            .collect::<Vec<_>>();

                        (removed, added)
                    });

                    children.update(|children| {
                        children.retain(|(child, _)| {
                            !removed.iter().any(|removed| Arc::ptr_eq(removed, child))
                        });

                        let added = added.iter().map(|added| {
                            nodes.with_untracked(|nodes| {
                                let node = nodes
                                    .iter()
                                    .find(|node| Arc::ptr_eq(&node.container, added))
                                    .unwrap();

                                (node.container.clone(), node.width())
                            })
                        });
                        children.extend(added);
                    });
                }
            });

            Self {
                container,
                visibility,
                children,
                _update: update,
            }
        }

        pub fn container(&self) -> &state::graph::Node {
            &self.container
        }

        pub fn children(&self) -> ArcReadSignal<Vec<(state::graph::Node, Signal<NonZeroUsize>)>> {
            self.children.read_only()
        }

        pub fn width(&self) -> Signal<NonZeroUsize> {
            let visibility = self.visibility.clone();
            let children = self.children.read_only();
            Signal::derive({
                move || {
                    children.with(|children| {
                        let children_widths = children
                            .iter()
                            .map(|(_data, width)| width)
                            .collect::<Vec<_>>();

                        if visibility.get() && !children_widths.is_empty() {
                            let width = children_widths
                                .iter()
                                .fold(0, |width, child_width| width + child_width.get().get());

                            NonZeroUsize::new(width).unwrap()
                        } else {
                            NonZeroUsize::new(1).unwrap()
                        }
                    })
                }
            })
        }
    }

    #[derive(Clone, Debug)]
    pub struct State {
        nodes: ArcRwSignal<Vec<Node>>,
        root: state::graph::Node,
        children: ReadSignal<state::graph::Children>,

        // TODO: Use trigger?
        /// Updates internal state when `children` changes.
        _update: Effect<LocalStorage>,
    }

    impl State {
        pub fn from(
            root: state::graph::Node,
            edges: ReadSignal<state::graph::Children>,
            visibilities: ReadSignal<state::workspace_graph::ContainerVisibility>,
        ) -> Self {
            let nodes = ArcRwSignal::new(vec![]);
            let data_nodes = edges.with_untracked(|children| {
                children
                    .iter()
                    .map(|(container, state_children)| {
                        let visibility = visibilities
                            .with_untracked(|visibilities| {
                                visibilities.iter().find_map(|(node, visibility)| {
                                    Arc::ptr_eq(node, container).then_some(visibility.read_only())
                                })
                            })
                            .unwrap();

                        Node::new(Data::from(
                            container.clone(),
                            state_children.read_only(),
                            visibility,
                            nodes.read_only(),
                        ))
                    })
                    .collect::<Vec<_>>()
            });

            data_nodes.iter().for_each(|data| {
                let container_children = edges.with_untracked(|children| {
                    children
                        .iter()
                        .find_map(|(parent, children)| {
                            Arc::ptr_eq(parent, &data.container).then_some(children.read_only())
                        })
                        .unwrap()
                });

                let display_children = container_children.with_untracked(|container_children| {
                    container_children
                        .iter()
                        .map(|container_child| {
                            data_nodes
                                .iter()
                                .find_map(|node| {
                                    Arc::ptr_eq(&node.container, container_child)
                                        .then_some((node.container.clone(), node.width()))
                                })
                                .unwrap()
                        })
                        .collect::<Vec<_>>()
                });

                data.children
                    .update_untracked(|children| children.extend(display_children))
            });

            nodes.write_untracked().extend(data_nodes);
            let update = Effect::new({
                let nodes = nodes.clone();
                move |_| {
                    edges.with(|edges| {
                        let removed = nodes.with_untracked(|nodes| {
                            nodes
                                .iter()
                                .filter(|data| {
                                    !edges
                                        .iter()
                                        .any(|(parent, _)| Arc::ptr_eq(parent, &data.container))
                                })
                                .map(|data| data.container.clone())
                                .collect::<Vec<_>>()
                        });

                        if !removed.is_empty() {
                            nodes.update(|nodes| {
                                nodes.retain(|data| {
                                    !removed
                                        .iter()
                                        .any(|removed| Arc::ptr_eq(removed, &data.container))
                                });
                            });
                        }

                        let added_states = edges
                            .iter()
                            .filter(|(parent, _)| {
                                nodes.with_untracked(|nodes| {
                                    !nodes
                                        .iter()
                                        .any(|node| Arc::ptr_eq(&node.container, parent))
                                })
                            })
                            .collect::<Vec<_>>();

                        let added_data = added_states
                            .iter()
                            .map(|(parent, state_children)| {
                                let visibility = visibilities.with_untracked(|visibilities| {
                                    visibilities
                                        .iter()
                                        .find_map(|(container, visibility)| {
                                            Arc::ptr_eq(container, parent)
                                                .then_some(visibility.read_only())
                                        })
                                        .unwrap()
                                });

                                Node::new(Data::from(
                                    parent.clone(),
                                    state_children.read_only(),
                                    visibility,
                                    nodes.read_only(),
                                ))
                            })
                            .collect::<Vec<_>>();

                        added_data.iter().for_each(|data| {
                            let state_children = added_states
                                .iter()
                                .find_map(|(parent, children)| {
                                    Arc::ptr_eq(parent, &data.container)
                                        .then_some(children.read_only())
                                })
                                .unwrap();

                            let children_width = state_children.with_untracked(|state_children| {
                                state_children
                                    .iter()
                                    .map(|state_child| {
                                        let data_child = added_data
                                            .iter()
                                            .find(|data_child| {
                                                Arc::ptr_eq(&data_child.container, state_child)
                                            })
                                            .unwrap();

                                        (state_child.clone(), data_child.width())
                                    })
                                    .collect::<Vec<_>>()
                            });

                            data.children
                                .update_untracked(|children| children.extend(children_width))
                        });

                        nodes.update(|nodes| nodes.extend(added_data));
                    })
                }
            });

            Self {
                nodes,
                root,
                children: edges,
                _update: update,
            }
        }

        pub fn nodes(&self) -> ArcReadSignal<Vec<Node>> {
            self.nodes.read_only()
        }

        pub fn get(&self, container: &state::graph::Node) -> Option<Node> {
            self.nodes.with_untracked(|nodes| {
                nodes
                    .iter()
                    .find(|node| Arc::ptr_eq(&node.container, container))
                    .cloned()
            })
        }

        pub fn width(&self, container: &state::graph::Node) -> Option<Signal<NonZeroUsize>> {
            self.nodes.with_untracked(|nodes| {
                nodes.iter().find_map(|node| {
                    Arc::ptr_eq(&node.container, container).then_some(node.width())
                })
            })
        }
    }
}
