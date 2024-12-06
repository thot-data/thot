use super::{
    common::{asset_title_closure, interpret_resource_selection_action, SelectionAction},
    state,
    workspace::ViewboxState,
};
use crate::{
    commands, common,
    components::{self, ToggleExpand, TruncateLeft},
    types,
};
use futures::StreamExt;
use leptos::{ev::MouseEvent, prelude::*, task::spawn_local};
use leptos_icons::Icon;
use std::rc::Rc;
use syre_core::types::ResourceId;
use syre_desktop_lib as lib;
use syre_local_database as db;
use tauri_sys::{core::Channel, menu};

/// Context menu for containers that are `Ok`.
#[derive(derive_more::Deref, Clone)]
struct ContextMenuContainerOk(Rc<menu::Menu>);
impl ContextMenuContainerOk {
    pub fn new(menu: Rc<menu::Menu>) -> Self {
        Self(menu)
    }
}

/// Context menu for assets.
#[derive(derive_more::Deref, Clone)]
struct ContextMenuAsset(Rc<menu::Menu>);
impl ContextMenuAsset {
    pub fn new(menu: Rc<menu::Menu>) -> Self {
        Self(menu)
    }
}

/// Active container for the container context menu.
#[derive(derive_more::Deref, derive_more::From, Clone)]
struct ContextMenuActiveContainer(state::graph::Node);

/// Active asset for the asset context menu.
#[derive(derive_more::Deref, derive_more::From, Clone)]
struct ContextMenuActiveAsset(ResourceId);

#[component]
pub fn LayersNav() -> impl IntoView {
    let project = expect_context::<state::Project>();
    let graph = expect_context::<state::Graph>();
    let messages = expect_context::<types::Messages>();

    let context_menu_active_container = RwSignal::<Option<ContextMenuActiveContainer>>::new(None);
    let context_menu_active_asset = RwSignal::<Option<ContextMenuActiveAsset>>::new(None);

    provide_context(context_menu_active_container);
    provide_context(context_menu_active_asset);

    let context_menu_container_ok = LocalResource::new({
        let project = project.clone();
        let graph = graph.clone();
        let messages = messages.clone();
        move |_| {
            let project = project.clone();
            let graph = graph.clone();
            let messages = messages.clone();
            async move {
                let mut container_open = tauri_sys::menu::item::MenuItemOptions::new("Open");
                container_open.set_id("layers_nav:container-open");

                let (menu, mut listeners) = menu::Menu::with_id_and_items(
                    "layers_nav:container-context_menu",
                    vec![container_open.into()],
                )
                .await;

                spawn_local({
                    let container_open = listeners.pop().unwrap().unwrap();
                    handle_context_menu_container_events(
                        project,
                        graph,
                        messages,
                        context_menu_active_container.read_only(),
                        container_open,
                    )
                });

                Rc::new(menu)
            }
        }
    });

    let context_menu_asset = LocalResource::new({
        let project = project.clone();
        let graph = graph.clone();
        let messages = messages.clone();
        move |_| {
            let project = project.clone();
            let graph = graph.clone();
            let messages = messages.clone();
            async move {
                let mut asset_open = tauri_sys::menu::item::MenuItemOptions::new("Open");
                asset_open.set_id("layers_nav:asset-open");

                let (menu, mut listeners) = menu::Menu::with_id_and_items(
                    "layers_nav:asset-context_menu",
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

                Rc::new(menu)
            }
        }
    });

    view! {
        <Suspense fallback=move || {
            view! { <LayersNavLoading /> }
        }>
            {move || {
                let Some(context_menu_container_ok) = context_menu_container_ok.get() else {
                    return None;
                };
                let Some(context_menu_asset) = context_menu_asset.get() else {
                    return None;
                };
                Some(view! { <LayersNavView context_menu_container_ok context_menu_asset /> })
            }}

        </Suspense>
    }
}

#[component]
fn LayersNavLoading() -> impl IntoView {
    view! { <div class="text-center pt-4">"Setting up layers navigation"</div> }
}

#[component]
pub fn LayersNavView(
    context_menu_container_ok: Rc<menu::Menu>,
    context_menu_asset: Rc<menu::Menu>,
) -> impl IntoView {
    let graph = expect_context::<state::Graph>();
    provide_context(ContextMenuContainerOk::new(context_menu_container_ok));
    provide_context(ContextMenuAsset::new(context_menu_asset));

    view! {
        <div class="h-full pt-2 px-1 overflow-auto scrollbar-thin dark:scrollbar-track-secondary-800">
            <ContainerLayer root=graph.root().clone() />
        </div>
    }
}

#[component]
fn ContainerLayer(root: state::graph::Node, #[prop(optional)] depth: usize) -> impl IntoView {
    let graph = expect_context::<state::Graph>();
    let expanded = RwSignal::new(true);

    view! {
        <div>
            {
                let root = root.clone();
                move || {
                    if root.properties().with(|properties| properties.is_ok()) {
                        view! { <ContainerLayerTitleOk container=root.clone() depth expanded /> }
                    } else {
                        view! { <ContainerLayerTitleErr container=root.clone() depth /> }
                    }
                }
            } <div class:hidden=move || !expanded()>
                <AssetsLayer container=root.clone() depth />
                <div>
                    <For
                        each={
                            let root = root.clone();
                            let graph = graph.clone();
                            move || graph.children(&root).unwrap().get()
                        }

                        key={
                            let graph = graph.clone();
                            move |child| graph.path(&child)
                        }

                        let:child
                    >
                        <ContainerLayer root=child depth=depth + 1 />
                    </For>
                </div>
            </div>
        </div>
    }
}

#[component]
fn ContainerLayerTitleOk(
    container: state::graph::Node,
    depth: usize,
    expanded: RwSignal<bool>,
) -> impl IntoView {
    const CLICK_DEBOUNCE: f64 = 250.0;

    let graph = expect_context::<state::Graph>();
    let workspace_graph_state = expect_context::<state::WorkspaceGraph>();
    let context_menu = expect_context::<ContextMenuContainerOk>();
    let context_menu_active_container =
        expect_context::<RwSignal<Option<ContextMenuActiveContainer>>>();
    let viewbox = expect_context::<ViewboxState>();

    let (click_event, set_click_event) = signal::<Option<MouseEvent>>(None);
    let click_event = leptos_use::signal_debounced(click_event, CLICK_DEBOUNCE);

    let properties = {
        let container = container.clone();
        move || {
            container.properties().with(|properties| {
                let db::state::DataResource::Ok(properties) = properties else {
                    panic!("invalid state");
                };

                properties.clone()
            })
        }
    };

    let selected = container.properties().with_untracked(|properties| {
        properties
            .as_ref()
            .unwrap()
            .rid()
            .with_untracked(|rid| workspace_graph_state.selection_resources().get(rid))
            .unwrap()
    });

    let container_visibility = workspace_graph_state
        .container_visibility_get(&container)
        .unwrap();

    let toggle_container_visibility = move |e: MouseEvent| {
        if e.button() != types::MouseButton::Primary {
            return;
        }
        e.stop_propagation();

        container_visibility.set(!container_visibility());
    };

    let num_children = {
        let graph = graph.clone();
        let container = container.clone();
        move || {
            graph
                .children(&container)
                .map(|children| children.with(|children| children.len()))
                .unwrap_or(0)
        }
    };

    let title = {
        let properties = properties.clone();
        move || properties().name().get()
    };

    let tooltip = {
        let container = container.clone();
        move || {
            let path = graph.path(&container).unwrap();

            let path = lib::utils::remove_root_path(path);
            path.to_string_lossy().to_string()
        }
    };

    let click = {
        let rid = container
            .properties()
            .with_untracked(|properties| properties.as_ref().unwrap().rid().read_only());
        let selection_resources = workspace_graph_state.selection_resources().clone();
        move |e: &MouseEvent| {
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

    let dblclick = {
        let rid = container.properties().with_untracked(|properties| {
            let db::state::DataResource::Ok(properties) = properties else {
                panic!("invalid state");
            };

            properties.rid().read_only()
        });

        move |e: &MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }
            e.stop_propagation();

            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();
            let node = document
                .query_selector(&format!(
                    "[data-resource=\"{}\"][data-rid=\"{}\"]",
                    super::canvas::DATA_KEY_CONTAINER,
                    rid.get_untracked()
                ))
                .unwrap()
                .unwrap();

            let object = node.closest("foreignObject").unwrap().unwrap();
            let object_x = object.get_attribute("x").unwrap().parse::<isize>().unwrap();

            let wrapper = node.closest("svg").unwrap().unwrap();
            let mut x = wrapper
                .get_attribute("x")
                .unwrap()
                .parse::<isize>()
                .unwrap();
            let mut y = wrapper
                .get_attribute("y")
                .unwrap()
                .parse::<isize>()
                .unwrap();

            let mut current_wrapper = wrapper;
            while let Some(parent) = current_wrapper.parent_element() {
                let Some(wrapper) = parent.closest("svg").unwrap() else {
                    break;
                };
                let Some(wrapper_x) = wrapper.get_attribute("x") else {
                    break;
                };
                let Some(wrapper_y) = wrapper.get_attribute("y") else {
                    break;
                };

                x += wrapper_x.parse::<isize>().unwrap();
                y += wrapper_y.parse::<isize>().unwrap();
                current_wrapper = wrapper;
            }

            let x0 = x + object_x - viewbox.width().with_untracked(|width| width / 2) as isize;
            let y0 = y - viewbox.height().with_untracked(|height| height / 2) as isize;
            leptos::batch(|| {
                viewbox.x().set(x0);
                viewbox.y().set(y0);
            });
        }
    };

    let _ = Effect::watch(
        move || click_event.get(),
        move |e, _, _| {
            let Some(e) = e else {
                return;
            };

            match e.detail() {
                1 => click(e),
                2 => dblclick(e),
                _ => {}
            }
        },
        false,
    );

    let contextmenu = {
        let container = container.clone();
        move |e: MouseEvent| {
            e.prevent_default();

            context_menu_active_container.update(|active_container| {
                let _ = active_container.insert(container.clone().into());
            });

            let menu = context_menu.clone();
            spawn_local(async move {
                menu.popup().await.unwrap();
            });
        }
    };

    view! {
        <div
            on:mousedown=move |e| set_click_event(Some(e))
            on:contextmenu=contextmenu
            prop:title=tooltip
            style:padding-left=move || { depth_to_padding(depth) }
            class="flex gap-1 cursor-pointer border-y border-transparent hover:border-secondary-400"
            class=(["bg-primary-200", "dark:bg-secondary-900"], selected)
        >
            <div class="inline-flex gap-1">
                <span>
                    <ToggleExpand expanded />
                </span>
            </div>
            <div class="grow inline-flex gap-2">
                <div class="grow inline-flex gap-1">
                    <TruncateLeft>{title}</TruncateLeft>
                </div>
                <div>
                    {move || {
                        if num_children() > 0 {
                            let visibility_icon = Signal::derive(move || {
                                container_visibility
                                    .with(|visible| {
                                        if *visible {
                                            components::icon::Eye
                                        } else {
                                            components::icon::EyeClosed
                                        }
                                    })
                            });
                            view! {
                                <button
                                    type="button"
                                    on:mousedown=toggle_container_visibility
                                    class="align-middle"
                                >
                                    <Icon icon=visibility_icon />
                                </button>
                            }
                                .into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}

#[component]
fn ContainerLayerTitleErr(container: state::graph::Node, depth: usize) -> impl IntoView {
    let title = {
        let container = container.clone();
        move || {
            container
                .name()
                .with(|name| name.to_string_lossy().to_string())
        }
    };

    view! { <div style:padding-left=move || { depth_to_padding(depth) }>{title}</div> }
}

#[component]
fn AssetsLayer(container: state::graph::Node, depth: usize) -> impl IntoView {
    move || {
        container.assets().with(|assets| {
            if let db::state::DataResource::Ok(assets) = assets {
                view! { <AssetsLayerOk assets=assets.read_only() depth=depth /> }
            } else {
                view! { <AssetsLayerErr depth /> }
            }
        })
    }
}

#[component]
fn AssetsLayerOk(assets: ReadSignal<Vec<state::Asset>>, depth: usize) -> impl IntoView {
    let expanded = RwSignal::new(false);
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
        <div class="group/assets">
            <Show
                when=move || assets.with(|assets| !assets.is_empty())
                fallback=move || ().into_view()
            >
                <div style:padding-left=move || { depth_to_padding(depth + 1) } class="flex">
                    <div class="inline-flex gap-1">
                        <span>
                            <ToggleExpand expanded />
                        </span>
                    </div>
                    <div class="inline-flex grow items-center">
                        <span class="pr-1">
                            <Icon icon=icondata::BsFiles />
                        </span>
                        <span class="grow">"Assets"</span>
                    </div>
                </div>
                <div class:hidden=move || !expanded()>
                    <For each=assets_sorted key=move |asset| asset.rid().get() let:asset>
                        <AssetLayer asset depth />
                    </For>
                </div>
            </Show>
        </div>
    }
}

#[component]
fn AssetLayer(asset: state::Asset, depth: usize) -> impl IntoView {
    let workspace_graph_state = expect_context::<state::WorkspaceGraph>();
    let context_menu = expect_context::<ContextMenuAsset>();
    let context_menu_active_asset = expect_context::<RwSignal<Option<ContextMenuActiveAsset>>>();

    let title = asset_title_closure(&asset);

    let selected = asset
        .rid()
        .with_untracked(|rid| workspace_graph_state.selection_resources().get(rid))
        .unwrap();

    let mousedown = {
        let rid = asset.rid().read_only();
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

    let contextmenu = {
        let asset = asset.clone();
        move |e: MouseEvent| {
            e.prevent_default();

            context_menu_active_asset.update(|active_asset| {
                let _ = active_asset.insert(asset.rid().get_untracked().into());
            });

            let menu = context_menu.clone();
            spawn_local(async move {
                menu.popup().await.unwrap();
            });
        }
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
            style:padding-left=move || { depth_to_padding(depth + 2) }
            class=(["bg-primary-200", "dark:bg-secondary-900"], selected)
            class="cursor-pointer border-y border-transparent hover:border-secondary-400"
        >
            <div
                style:padding-left=move || { depth_to_padding(1) }
                class="flex gap-1 border-l border-transparent group-hover/assets:border-secondary-200 dark:group-hover/assets:border-secondary-600"
            >
                <div class=icon_class>
                    <Icon icon />
                </div>
                <div>
                    <TruncateLeft class="align-center" inner_class="align-middle">
                        {title}
                    </TruncateLeft>
                </div>
            </div>
        </div>
    }
}

#[component]
fn AssetsLayerErr(depth: usize) -> impl IntoView {
    view! { <div style:padding-left=move || { depth_to_padding(depth + 1) }>"(assets error)"</div> }
}

fn depth_to_padding(depth: usize) -> String {
    const LAYER_PADDING_SCALE: usize = 1;

    format!("{}ch", depth * LAYER_PADDING_SCALE)
}

async fn handle_context_menu_container_events(
    project: state::Project,
    graph: state::Graph,
    messages: types::Messages,
    context_menu_active_container: ReadSignal<Option<ContextMenuActiveContainer>>,
    container_open: Channel<String>,
) {
    let mut container_open = container_open.fuse();
    loop {
        futures::select! {
            event = container_open.next() => match event {
                None => continue,
                Some(_id) => {
                    let data_root = project
                        .path()
                        .get_untracked()
                        .join(project.properties().data_root().get_untracked());

                    let container = context_menu_active_container.get_untracked().unwrap();
                    let container_path = graph.path(&container).unwrap();
                    let path = common::container_system_path(data_root, container_path);

                    if let Err(err) = commands::fs::open_file(path)
                        .await {
                            messages.update(|messages|{
                                let mut msg = types::message::Builder::error("Could not open container folder.");
                                msg.body(format!("{err:?}"));
                            messages.push(msg.build());
                        });
                    }
            }
            }
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
