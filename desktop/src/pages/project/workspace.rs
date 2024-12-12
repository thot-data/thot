use super::{
    super::Settings, canvas, properties, state, Canvas, LayersNav, ProjectBar, PropertiesBar,
};
use crate::{
    commands, common,
    components::{self, drawer, Drawer, Logo},
    types,
};
use futures::stream::StreamExt;
use leptos::{
    either::{either, Either},
    ev::MouseEvent,
    html,
    portal::Portal,
    prelude::*,
    task::spawn_local,
};
use leptos_icons::*;
use leptos_router::{components::A, hooks::use_params_map};
use serde::Serialize;
use std::{io, path::PathBuf, str::FromStr, sync::Arc};
use syre_core::{self as core, types::ResourceId};
use syre_desktop_lib as lib;
use syre_local::{self as local, types::AnalysisKind};
use syre_local_database as db;
use tauri_sys::window::DragDropPayload;
use wasm_bindgen::JsCast;

/// Drag-drop event debounce in ms.
const THROTTLE_DRAG_EVENT: f64 = 50.0;

#[derive(Clone, Copy, derive_more::Deref, derive_more::From)]
struct ShowSettings(RwSignal<bool>);
impl ShowSettings {
    pub fn new() -> Self {
        Self(RwSignal::new(false))
    }
}

#[derive(derive_more::Deref, derive_more::From, Clone, PartialEq)]
pub struct DragOverWorkspaceResource(Option<WorkspaceResource>);
impl DragOverWorkspaceResource {
    pub fn new() -> Self {
        Self(None)
    }

    pub fn into_inner(self) -> Option<WorkspaceResource> {
        self.0
    }
}

#[component]
pub fn Workspace() -> impl IntoView {
    let params = use_params_map();
    let id =
        move || params.with(|params| ResourceId::from_str(&params.get("id").unwrap()).unwrap());
    let active_user = LocalResource::new(commands::user::fetch_user);
    let resources = LocalResource::new(move || fetch_project_resources(id()));

    view! {
        <Suspense fallback=Loading>
            <ErrorBoundary fallback=|errors| {
                view! { <UserError errors /> }
            }>
                {move || Suspend::new(async move {
                    let user = active_user.await;
                    user.map(|user| match user {
                        None => Either::Left(view! { <NoUser /> }),
                        Some(user) => {
                            Either::Right(
                                view! {
                                    <Suspense fallback=Loading>
                                        {
                                            let user = user.clone();
                                            move || Suspend::new({
                                                let user = user.clone();
                                                async move {
                                                    let resources = resources.await;
                                                    resources
                                                        .map(|(project_path, project_data, graph)| {
                                                            Either::Left(
                                                                view! {
                                                                    <WorkspaceView user project_path project_data graph />
                                                                },
                                                            )
                                                        })
                                                        .unwrap_or(Either::Right(view! { <NoProject /> }))
                                                }
                                            })
                                        }

                                    </Suspense>
                                },
                            )
                        }
                    })
                })}
            </ErrorBoundary>
        </Suspense>
    }
}

#[component]
fn Loading() -> impl IntoView {
    view! { <div class="pt-4 text-center">"Loading..."</div> }
}

#[component]
fn NoUser() -> impl IntoView {
    let messages = expect_context::<types::Messages>();
    let navigate = leptos_router::hooks::use_navigate();

    let msg = types::message::Builder::error("You are not logged in.").build();
    messages.update(|messages| messages.push(msg));
    navigate("login", Default::default());

    view! {
        <div class="text-center">
            <p>"You are not logged in."</p>
            <p>"Taking you to the login page."</p>
        </div>
    }
}

#[component]
fn UserError(errors: ArcRwSignal<Errors>) -> impl IntoView {
    view! {
        <div class="text-center">
            <div class="text-large p4">"Error with user."</div>
            <div>{format!("{errors:?}")}</div>
        </div>
    }
}

#[component]
fn NoProject() -> impl IntoView {
    view! {
        <div>
            <div class="p-4 text-center">"Project state was not found."</div>
            <div class="text-center">
                <A href="/" attr:class="btn btn-primary">
                    "Dashboard"
                </A>
            </div>
        </div>
    }
}

#[component]
fn WorkspaceView(
    user: core::system::User,
    project_path: PathBuf,
    project_data: db::state::ProjectData,
    graph: db::state::FolderResource<db::state::Graph>,
) -> impl IntoView {
    assert!(project_data.properties().is_ok());

    let analyze_node = NodeRef::<html::Div>::new();
    let project = state::Project::new(project_path, project_data);
    provide_context(user);
    provide_context(state::Workspace::new());
    provide_context(project.clone());
    provide_context(DragOverWorkspaceResource::new());
    provide_context(RwSignal::new(properties::EditorKind::default()));
    let user_settings = types::settings::User::new(lib::settings::User::default());
    provide_context(user_settings.clone());

    let show_settings = ShowSettings::new();
    provide_context(show_settings);

    spawn_local({
        let project = project.clone();
        async move {
            let mut listener = tauri_sys::event::listen::<Vec<lib::Event>>(
                &project
                    .rid()
                    .with_untracked(|rid| lib::event::topic::graph(rid)),
            )
            .await
            .unwrap();

            while let Some(events) = listener.next().await {
                tracing::debug!(?events);
                for event in events.payload {
                    let lib::EventKind::Project(update) = event.kind() else {
                        panic!("invalid event kind");
                    };

                    match update {
                        db::event::Project::FolderRemoved
                        | db::event::Project::Moved(_)
                        | db::event::Project::Properties(_)
                        | db::event::Project::Settings(_)
                        | db::event::Project::Analyses(_)
                        | db::event::Project::AnalysisFile(_) => {
                            handle_event_project(event, project.clone())
                        }

                        db::event::Project::Graph(_)
                        | db::event::Project::Container { .. }
                        | db::event::Project::Asset { .. }
                        | db::event::Project::AssetFile(_)
                        | db::event::Project::Flag { .. } => continue, // handled elsewhere
                    }
                }
            }
        }
    });

    view! {
        <div class="select-none flex flex-col h-full relative">
            <ProjectNav />
            <div class="border-b">
                <ProjectBar analyze_node />
            </div>
            {move || {
                either!(
                    graph.as_ref(),
                    db::state::FolderResource::Present(graph) => {
                        view! { <WorkspaceGraph graph=graph.clone() analyze_node /> }
                    },
                    db::state::FolderResource::Absent => view! { <NoGraph /> },
                )
            }}

            <div
                class=(["-right-full", "left-full"], move || !show_settings())
                class=(["right-0", "left-0"], move || show_settings())
                class="absolute top-0 bottom-0 transition-absolute-position z-20"
            >
                <Settings onclose=Callback::new(move |_| show_settings.set(false)) />
            </div>
        </div>
    }
}

#[component]
fn NoGraph() -> impl IntoView {
    view! { <div class="text-center pt-4">"Data graph does not exist."</div> }
}

#[component]
fn WorkspaceGraph(graph: db::state::Graph, analyze_node: NodeRef<html::Div>) -> impl IntoView {
    let project = expect_context::<state::Project>();
    let messages = expect_context::<types::Messages>();
    let graph = state::Graph::new(graph);
    let workspace_graph_state = state::WorkspaceGraph::new(&graph);
    let viewbox = ViewboxState::default();
    provide_context(graph.clone());
    provide_context(workspace_graph_state.clone());
    provide_context(viewbox.clone());

    let (drag_over_event, set_drag_over_event) = signal(tauri_sys::window::DragDropEvent::Leave);
    let drag_over_event = leptos_use::signal_throttled(drag_over_event, THROTTLE_DRAG_EVENT);
    let (drag_over_container_elm, set_drag_over_container_elm) = signal_local(None);
    let (drag_over_workspace_resource, set_drag_over_workspace_resource) =
        signal(DragOverWorkspaceResource::new());
    let drag_over_workspace_resource_signal: Signal<DragOverWorkspaceResource> =
        Signal::from(drag_over_workspace_resource);
    provide_context(drag_over_workspace_resource_signal);

    spawn_local({
        let project = project.clone();
        let graph = graph.clone();
        let workspace_graph_state = workspace_graph_state.clone();
        async move {
            let mut listener = tauri_sys::event::listen::<Vec<lib::Event>>(
                &project
                    .rid()
                    .with_untracked(|rid| lib::event::topic::graph(rid)),
            )
            .await
            .unwrap();

            while let Some(events) = listener.next().await {
                for event in events.payload {
                    let lib::EventKind::Project(update) = event.kind() else {
                        panic!("invalid event kind");
                    };

                    match update {
                        db::event::Project::FolderRemoved
                        | db::event::Project::Moved(_)
                        | db::event::Project::Properties(_)
                        | db::event::Project::Settings(_)
                        | db::event::Project::Analyses(_)
                        | db::event::Project::AnalysisFile(_) => continue, // handled elsewhere

                        db::event::Project::Graph(_)
                        | db::event::Project::Container { .. }
                        | db::event::Project::Asset { .. }
                        | db::event::Project::AssetFile(_)
                        | db::event::Project::Flag { .. } => {
                            handle_event_graph(event, graph.clone(), workspace_graph_state.clone())
                        }
                    }
                }
            }
        }
    });

    {
        // TODO: Tested on Linux and Windows.
        // Need to test on Mac.
        // Check if needed on unix systems.
        use tauri_sys::window::DragDropEvent;
        let _ = Effect::watch(
            move || drag_over_event.get(),
            {
                let graph = graph.clone();
                move |event, _, _| match event {
                    DragDropEvent::Enter(payload) => {
                        // Cursor entered window
                        if payload.paths().is_empty() {
                            return;
                        }

                        let payload = payload.clone();
                        spawn_local(async move {
                            let (resource, elm) =
                                match resource_from_position(payload.position()).await {
                                    None => (None, None),
                                    Some((resource, elm)) => (Some(resource), elm),
                                };
                            if drag_over_workspace_resource
                                .with_untracked(|current| resource != **current)
                            {
                                set_drag_over_workspace_resource(resource.into());
                                if let Some(container) = elm {
                                    set_drag_over_container_elm.update(|elm| {
                                        let _ = elm.insert(container);
                                    });
                                }
                            }
                        });
                    }
                    DragDropEvent::Over(payload) => {
                        let payload = payload.clone();
                        spawn_local(async move {
                            let (resource, elm) =
                                match resource_from_position(payload.position()).await {
                                    None => (None, None),
                                    Some((resource, elm)) => (Some(resource), elm),
                                };
                            if drag_over_workspace_resource
                                .with_untracked(|current| resource != **current)
                            {
                                set_drag_over_workspace_resource(resource.into());
                                if let Some(container) = elm {
                                    set_drag_over_container_elm.update(|elm| {
                                        let _ = elm.insert(container);
                                    });
                                } else {
                                    set_drag_over_container_elm.update(|elm| {
                                        elm.take();
                                    });
                                }
                            }
                        });
                    }
                    DragDropEvent::Leave => {
                        // Cursor exited window
                        if drag_over_workspace_resource.with_untracked(|current| current.is_some())
                        {
                            set_drag_over_workspace_resource(None.into());
                        }
                    }
                    DragDropEvent::Drop(payload) => {
                        if let Some(resource) =
                            drag_over_workspace_resource.get_untracked().into_inner()
                        {
                            set_drag_over_workspace_resource(None.into());

                            // NB: Spawn seperate thread to handle large copies.
                            let payload = payload.clone();
                            spawn_local({
                                let project = project.clone();
                                let graph = graph.clone();
                                async move {
                                    handle_drop_event(resource, payload, &project, &graph, messages)
                                        .await
                                }
                            });
                        }
                    }
                }
            },
            false,
        );

        let _ = Effect::watch(
            drag_over_container_elm,
            move |elm, prev_container, _| {
                if let Some(elm) = prev_container {
                    if let Some(container) = elm.as_ref() {
                        let event = web_sys::Event::new("dragleave_windows").unwrap();
                        container.dispatch_event(&event).unwrap();
                    }
                }

                if let Some(container) = elm.as_ref() {
                    let event = web_sys::Event::new("dragenter_windows").unwrap();
                    container.dispatch_event(&event).unwrap();
                }

                elm.clone()
            },
            false,
        );

        spawn_local(async move {
            let window = tauri_sys::window::get_current();
            let mut listener = window.on_drag_drop_event().await.unwrap();
            while let Some(event) = listener.next().await {
                set_drag_over_event(event.payload);
            }
        });
    }

    view! {
        <div class="flex grow min-h-0">
            <div class="grow flex min-h-0 relative overflow-hidden">
                <Drawer
                    dock=drawer::Dock::East
                    absolute=true
                    class="min-w-28 max-w-[40%] bg-white dark:bg-secondary-800 w-1/6 border-r"
                >
                    <LayersNav />
                </Drawer>
                <div class="grow">
                    <Canvas />
                </div>
                <Drawer
                    dock=drawer::Dock::West
                    absolute=true
                    class="min-w-28 max-w-[40%] bg-white dark:bg-secondary-800 w-1/6 border-l"
                >
                    <PropertiesBar />
                </Drawer>
            </div>

            {move || {
                if let Some(analyze_node) = analyze_node.get() {
                    let mount = (*analyze_node).clone();
                    Either::Left(
                        view! {
                            <Portal mount>
                                <analyze::Analyze />
                            </Portal>
                        },
                    )
                } else {
                    Either::Right(())
                }
            }}
        </div>
    }
}

#[component]
fn ProjectNav() -> impl IntoView {
    let show_settings = expect_context::<ShowSettings>();
    let open_settings = move |e: MouseEvent| {
        if e.button() != types::MouseButton::Primary {
            return;
        }

        show_settings.set(true);
    };

    view! {
        <nav class="px-2 border-b dark:bg-secondary-900 flex items-center">
            <ol class="flex grow">
                <li>
                    <A href="/">
                        <Logo attr:class="h-4" />
                    </A>
                </li>
            </ol>
            <ol>
                <li>
                    <button
                        on:mousedown=open_settings
                        type="button"
                        class="align-middle p-1 hover:bg-secondary-100 dark:hover:bg-secondary-800 rounded border border-transparent hover:border-black dark:hover:border-white"
                    >
                        <Icon icon=components::icon::Settings />
                    </button>
                </li>
            </ol>
        </nav>
    }
}

mod analyze {
    use super::state;
    use crate::{components, types};
    use futures::stream::StreamExt;
    use leptos::{ev::MouseEvent, prelude::*, task::spawn_local};
    use leptos_icons::*;
    use std::path::PathBuf;
    use syre_core::types::ResourceId;
    use syre_desktop_lib as lib;
    use syre_local::types::AnalysisKind;
    use syre_local_database as db;

    enum AnalysisState {
        Idle,
        Pending,
        Running { completed: usize, remaining: usize },
        Cancelling { completed: usize, remaining: usize },
        Killing { completed: usize, remaining: usize },
    }

    impl AnalysisState {
        pub fn active(&self) -> bool {
            match self {
                AnalysisState::Idle | AnalysisState::Pending => false,
                AnalysisState::Running { .. }
                | AnalysisState::Cancelling { .. }
                | AnalysisState::Killing { .. } => true,
            }
        }

        pub fn running(&self) -> bool {
            matches!(self, AnalysisState::Running { .. })
        }
    }

    #[component]
    pub fn Analyze() -> impl IntoView {
        let project = expect_context::<state::Project>();
        let graph = expect_context::<state::Graph>();
        let messages = expect_context::<types::Messages>();
        let analysis_state = RwSignal::new(AnalysisState::Idle);
        provide_context(analysis_state);

        let action = Action::new({
            let analyses = project.analyses();
            let project = project.rid().read_only();
            move |root: &PathBuf| {
                let graph = graph.clone();
                let root = root.clone();
                async move {
                    analysis_state.set(AnalysisState::Pending);
                    let rx: tauri_sys::core::Channel<lib::event::analysis::Update> =
                        match trigger_analysis(project.get_untracked(), root).await {
                            Ok(rx) => rx,
                            Err(err) => {
                                tracing::error!(?err);
                                analysis_state.set(AnalysisState::Idle);
                                let mut msg = types::message::Builder::error(
                                    "Could not initialize analysis.",
                                );
                                msg.body(format!("{err:?}"));
                                messages.update(|messages| messages.push(msg.build()));
                                return;
                            }
                        };

                    spawn_local(handle_analysis_updates(
                        rx,
                        analysis_state,
                        analyses,
                        graph.clone(),
                        messages,
                    ));
                }
            }
        });

        view! {
            <Show
                when=move || analysis_state.with(|state| state.active())
                fallback=move || view! { <Trigger action /> }
            >
                <Analyzing />
            </Show>
        }
    }

    #[component]
    fn Trigger(action: Action<PathBuf, ()>) -> impl IntoView {
        let workspace_graph_state = expect_context::<state::WorkspaceGraph>();
        let graph = expect_context::<state::Graph>();

        let single_container_selected = {
            let selected_resources = workspace_graph_state.selection_resources().selected();
            move || {
                selected_resources.with(|resources| {
                    resources.len() == 1
                        && matches!(
                            resources[0].kind(),
                            state::workspace_graph::ResourceKind::Container
                        )
                        && resources[0].rid().with_untracked(|rid| {
                            graph.root().properties().with_untracked(|properties| {
                                properties.as_ref().map_or(true, |properties| {
                                    properties.rid().with_untracked(|root| root != rid)
                                })
                            })
                        })
                })
            }
        };

        let trigger_analysis = move |e: MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }

            action.dispatch(PathBuf::from("/"));
        };

        view! {
            <div class="flex">
                <button
                    on:mousedown=trigger_analysis
                    class:rounded-r={
                        let single_container_selected = single_container_selected.clone();
                        move || !single_container_selected()
                    }
                    class="flex gap-2 items-center btn-primary rounded-l px-4 \
                    disabled:bg-primary-800 dark:disabled:bg-primary-400 \
                    disabled:cursor-not-allowed"
                    disabled={
                        let pending = action.pending();
                        move || pending.get()
                    }
                >
                    "Analyze"
                </button>
                <Show when=single_container_selected fallback=|| ()>
                    <TriggerActions action />
                </Show>
            </div>
        }
    }

    #[component]
    fn TriggerActions(action: Action<PathBuf, ()>) -> impl IntoView {
        let workspace_graph_state = expect_context::<state::WorkspaceGraph>();
        let graph = expect_context::<state::Graph>();
        let selected_resources = workspace_graph_state.selection_resources().selected();

        let trigger_analysis_from_root = move |e: MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }

            action.dispatch(PathBuf::from("/"));
        };

        let trigger_analysis_from_container = move |e: MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }

            let root = selected_resources.with_untracked(|resources| {
                resources[0].rid().with_untracked(|rid| {
                    let root = graph.find_by_id(rid).unwrap();
                    graph.path(&root).unwrap()
                })
            });

            action.dispatch(PathBuf::from(root));
        };

        view! {
            <div class="group relative">
                <div class="flex items-center h-full btn-primary rounded-r px-1 border-l border-l-primary-800">
                    <Icon icon=components::icon::ChevronDown />
                </div>
                <div class="absolute top-full left-0 group-[&:not(:hover)]:hidden \
                z-10 rounded-b rounded-r border border-secondary-800 dark:border-secondary-200 \
                bg-white dark:bg-secondary-700">
                    <ul>
                        <li class="hover:bg-100 dark:hover:bg-secondary-800">
                            <button
                                on:click=trigger_analysis_from_container
                                class="px-2 text-nowrap"
                                title="Only analyze the subtree from the selected root container."
                            >
                                "From container"
                            </button>
                        </li>
                        <li class="hover:bg-100 dark:hover:bg-secondary-800">
                            <button
                                on:click=trigger_analysis_from_root
                                class="px-2"
                                title="Aanlyze the entire project."
                            >
                                "Project"
                            </button>
                        </li>
                    </ul>
                </div>
            </div>
        }
    }

    #[component]
    fn Analyzing() -> impl IntoView {
        let analysis_state = expect_context::<RwSignal<AnalysisState>>();
        let title = move || {
            analysis_state.with(|state| match state {
                AnalysisState::Idle | AnalysisState::Pending => unreachable!(),
                AnalysisState::Running {
                    completed,
                    remaining,
                }
                | AnalysisState::Cancelling {
                    completed,
                    remaining,
                }
                | AnalysisState::Killing {
                    completed,
                    remaining,
                } => format!("{} of {} remaining", remaining, completed + remaining),
            })
        };

        let percent_complete = move || {
            analysis_state.with(|state| match state {
                AnalysisState::Idle | AnalysisState::Pending => unreachable!(),
                AnalysisState::Running {
                    completed,
                    remaining,
                }
                | AnalysisState::Cancelling {
                    completed,
                    remaining,
                }
                | AnalysisState::Killing {
                    completed,
                    remaining,
                } => {
                    let percent_complete = 100 * completed / (completed + remaining);
                    format!("{percent_complete}%")
                }
            })
        };

        let text = move || {
            analysis_state.with(|state| match state {
                AnalysisState::Running { .. } => "Analyzing",
                AnalysisState::Cancelling { .. } => "Cancelling",
                AnalysisState::Killing { .. } => "Killing",
                _ => panic!("invalid state"),
            })
        };

        view! {
            <div class="flex">
                <button
                    class="relative btn-primary rounded-l px-4 cursor-not-allowed"
                    class:rounded-r=move || analysis_state.with(|state| !state.running())
                    title=title
                    disabled=true
                >
                    <div class="flex gap-2 items-center">
                        {text} <span class="animate-spin">
                            <Icon icon=components::icon::Refresh />
                        </span>
                    </div>
                    <div class="absolute bottom-0 left-1 right-1 h-0.5 rounded-full bg-primary-800 dark:bg-primary-700">
                        <span
                            class="h-full block rounded-full bg-syre-green-800 dark:bg-syre-green-500"
                            style:width=percent_complete
                        ></span>
                    </div>
                </button>
                <Show when=move || analysis_state.with(|state| state.running()) fallback=|| ()>
                    <AnalyzingActions />
                </Show>
            </div>
        }
    }

    #[component]
    fn AnalyzingActions() -> impl IntoView {
        let analysis_state = expect_context::<RwSignal<AnalysisState>>();
        let cancel_analysis = move |e: MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }

            analysis_state.update(|state| {
                let AnalysisState::Running {
                    completed,
                    remaining,
                } = state
                else {
                    panic!("invalid state");
                };

                *state = AnalysisState::Cancelling {
                    completed: *completed,
                    remaining: *remaining,
                }
            });
            spawn_local(async { cancel_analysis().await });
        };

        let kill_analysis = move |e: MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }

            analysis_state.update(|state| {
                let AnalysisState::Running {
                    completed,
                    remaining,
                } = state
                else {
                    panic!("invalid state");
                };

                *state = AnalysisState::Killing {
                    completed: *completed,
                    remaining: *remaining,
                }
            });
            spawn_local(async { kill_analysis().await });
        };

        view! {
            <div class="group relative">
                <div class="flex items-center h-full btn-primary rounded-r px-1 border-l border-l-primary-800">
                    <Icon icon=components::icon::ChevronDown />
                </div>
                <div class="absolute top-full left-0 group-[&:not(:hover)]:hidden \
                z-10 rounded-b rounded-r border border-secondary-800 dark:border-secondary-200 \
                bg-white dark:bg-secondary-700">
                    <ul>
                        <li class="hover:bg-100 dark:hover:bg-secondary-800">
                            <button
                                on:click=cancel_analysis
                                class="px-2"
                                title="Cancel all remaining analyses, allowing those currently running to finish."
                            >
                                "Cancel"
                            </button>
                        </li>
                        <li class="hover:bg-100 dark:hover:bg-secondary-800">
                            <button
                                on:click=kill_analysis
                                class="px-2"
                                title="Immediately kill all analyses, even those currently running."
                            >
                                "Kill"
                            </button>
                        </li>
                    </ul>
                </div>
            </div>
        }
    }

    async fn trigger_analysis(
        project: ResourceId,
        root: impl Into<PathBuf>,
    ) -> Result<
        tauri_sys::core::Channel<lib::event::analysis::Update>,
        lib::command::project::error::TriggerAnalysis,
    > {
        #[derive(serde::Serialize)]
        struct Args<'a> {
            rx: &'a tauri_sys::core::Channel<lib::event::analysis::Update>,
            project: ResourceId,
            root: PathBuf,
            max_tasks: Option<usize>,
        }

        let rx = tauri_sys::core::Channel::new();
        tauri_sys::core::invoke_result::<(), lib::command::project::error::TriggerAnalysis>(
            "trigger_analysis",
            Args {
                rx: &rx,
                project,
                root: root.into(),
                max_tasks: None,
            },
        )
        .await?;

        Ok(rx)
    }

    async fn handle_analysis_updates(
        mut rx: tauri_sys::core::Channel<lib::event::analysis::Update>,
        analysis_state: RwSignal<AnalysisState>,
        analyses: RwSignal<db::state::DataResource<RwSignal<Vec<state::Analysis>>>>,
        graph: state::Graph,
        messages: types::Messages,
    ) {
        while let Some(event) = rx.next().await {
            match event {
                lib::event::analysis::Update::Progress {
                    completed: update_completed,
                    remaining: update_remaining,
                } => analysis_state.update(|state| match state {
                    AnalysisState::Idle => panic!("invalid state"),

                    AnalysisState::Pending => {
                        *state = AnalysisState::Running {
                            completed: update_completed,
                            remaining: update_remaining,
                        }
                    }

                    AnalysisState::Running {
                        completed,
                        remaining,
                    }
                    | AnalysisState::Cancelling {
                        completed,
                        remaining,
                    }
                    | AnalysisState::Killing {
                        completed,
                        remaining,
                    } => {
                        *completed = update_completed;
                        *remaining = update_remaining;
                    }
                }),

                lib::event::analysis::Update::Done(status) => {
                    analysis_state.set(AnalysisState::Idle);
                    let errors = status
                        .iter()
                        .filter(|status| {
                            status
                                .output()
                                .map(|output| !output.status.success())
                                .unwrap_or(false)
                        })
                        .collect::<Vec<_>>();

                    if errors.is_empty() {
                        let msg = types::message::Builder::success("Analysis complete.");
                        messages.update(|messages| messages.push(msg.build()));
                    } else {
                        let mut msg =
                            types::message::Builder::error("Errors occurred during analysis.");
                        msg.body(view! {
                            <ol class="list-decimal">
                                {errors
                                    .iter()
                                    .map(|err| {
                                        let analysis = analyses
                                            .with_untracked(|analyses| {
                                                analyses
                                                    .as_ref()
                                                    .unwrap()
                                                    .with_untracked(|analyses| {
                                                        analyses
                                                            .iter()
                                                            .find_map(|analysis| {
                                                                analysis
                                                                    .properties()
                                                                    .with_untracked(|analysis| {
                                                                        match analysis {
                                                                            AnalysisKind::Script(script) => {
                                                                                (script.rid() == err.analysis())
                                                                                    .then_some(script.path.to_string_lossy().to_string())
                                                                            }
                                                                            AnalysisKind::ExcelTemplate(template) => {
                                                                                (template.rid() == err.analysis())
                                                                                    .then_some(
                                                                                        template.template.path.to_string_lossy().to_string(),
                                                                                    )
                                                                            }
                                                                        }
                                                                    })
                                                            })
                                                            .unwrap()
                                                    })
                                            });
                                        let container = graph.find_by_id(err.container()).unwrap();
                                        let container = graph
                                            .path(&container)
                                            .unwrap()
                                            .to_string_lossy()
                                            .to_string();
                                        let stderr = err
                                            .output()
                                            .map_or(
                                                "Could not retrieve error message".to_string(),
                                                |output| {
                                                    String::from_utf8(output.stderr.clone()).unwrap()
                                                },
                                            );
                                        view! {
                                            <li class="pb-4">
                                                <div>
                                                    <strong>{analysis}</strong>
                                                    " running on "
                                                    <strong>{container}</strong>
                                                </div>
                                                ": "
                                                <div>{stderr}</div>
                                            </li>
                                        }
                                    })
                                    .collect::<Vec<_>>()}
                            </ol>
                        });
                        messages.update(|messages| messages.push(msg.build()));
                    }
                    break;
                }
            }
        }
    }

    async fn cancel_analysis() {
        tauri_sys::core::invoke::<()>("cancel_analysis", ()).await
    }

    async fn kill_analysis() {
        tauri_sys::core::invoke::<()>("kill_analysis", ()).await
    }
}

#[derive(PartialEq, Clone)]
pub enum WorkspaceResource {
    /// Analyses properties bar.
    Analyses,

    /// Container canvas ui.
    Container(ResourceId),

    /// Asset canvas ui.
    Asset(ResourceId),
}

/// State of an `svg` `viewbox` atribute.
///
/// Used for [`Canvas`].
#[derive(Debug, Clone)]
pub struct ViewboxState {
    x: RwSignal<isize>,
    y: RwSignal<isize>,
    width: RwSignal<usize>,
    height: RwSignal<usize>,
}

impl ViewboxState {
    pub fn x(&self) -> &RwSignal<isize> {
        &self.x
    }

    pub fn y(&self) -> &RwSignal<isize> {
        &self.y
    }

    pub fn width(&self) -> &RwSignal<usize> {
        &self.width
    }

    pub fn height(&self) -> &RwSignal<usize> {
        &self.height
    }
}

impl Default for ViewboxState {
    fn default() -> Self {
        use super::canvas;

        Self {
            x: RwSignal::new(0),
            y: RwSignal::new(0),
            width: RwSignal::new(canvas::VB_BASE),
            height: RwSignal::new(canvas::VB_BASE),
        }
    }
}

impl std::fmt::Display for ViewboxState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            self.x.get_untracked(),
            self.y.get_untracked(),
            self.width.get_untracked(),
            self.height.get_untracked()
        )
    }
}

/// Get a resource from a location on screen.
///
/// # Returns
/// `Some(resource, Option<element>)` if the position represents a resource.
/// `Option<element>` is `Some` for container resources, where `element` is the DOM
/// element representing the container.
async fn resource_from_position(
    position: &tauri_sys::dpi::PhysicalPosition,
) -> Option<(WorkspaceResource, Option<web_sys::Element>)> {
    let monitor = tauri_sys::window::current_monitor().await.unwrap();
    let position = position.as_logical(monitor.scale_factor());
    let (x, y) = (position.x(), position.y());
    if analyses_from_point(x, y) {
        Some((WorkspaceResource::Analyses, None))
    } else if let Some((id, elm)) = container_from_point(x, y) {
        Some((WorkspaceResource::Container(id), Some(elm)))
    } else {
        None
    }
}

/// Is the point within the analyses properties bar.
///
/// # Arguments
/// `x`, `y`: Logical size.
fn analyses_from_point(x: isize, y: isize) -> bool {
    document()
        .elements_from_point(x as f32, y as f32)
        .iter()
        .find(|elm| {
            let elm = elm.dyn_ref::<web_sys::Element>().unwrap();
            elm.id() == properties::ANALYSES_ID
        })
        .is_some()
}

/// Container the point is over.
///
/// # Arguments
/// `x`, `y`: Logical size.
///
/// # Returns
/// `Some((id, elm))` if the point is over a valid container.`
fn container_from_point(x: isize, y: isize) -> Option<(ResourceId, web_sys::Element)> {
    document()
        .elements_from_point(x as f32, y as f32)
        .iter()
        .find_map(|elm| {
            let elm = elm.dyn_ref::<web_sys::Element>().unwrap();
            if let Some(kind) = elm.get_attribute("data-resource") {
                if kind == canvas::DATA_KEY_CONTAINER {
                    if let Some(rid) = elm.get_attribute("data-rid") {
                        let rid = ResourceId::from_str(&rid).unwrap();
                        return Some((rid, elm.clone()));
                    }
                }

                None
            } else {
                None
            }
        })
}

async fn handle_drop_event(
    resource: WorkspaceResource,
    payload: DragDropPayload,
    project: &state::Project,
    graph: &state::Graph,
    messages: types::Messages,
) {
    match resource {
        WorkspaceResource::Analyses => {
            handle_drop_event_analyses(payload, project.rid().get_untracked(), messages).await
        }
        WorkspaceResource::Container(container) => {
            handle_drop_event_container(
                container,
                payload,
                project.rid().get_untracked(),
                graph,
                messages,
            )
            .await
        }
        WorkspaceResource::Asset(_) => todo!(),
    }
}

/// Handle drop event on a container.
async fn handle_drop_event_container(
    container: ResourceId,
    payload: DragDropPayload,
    project: ResourceId,
    graph: &state::Graph,
    messages: types::Messages,
) {
    let container_node = graph.find_by_id(&container).unwrap();
    let container_path = graph.path(&container_node).unwrap();

    let transfer_size = match commands::fs::file_size(payload.paths().clone())
        .await
        .map(|sizes| {
            sizes
                .into_iter()
                .reduce(|total, size| total + size)
                .unwrap_or(0)
        }) {
        Ok(size) => size,
        Err(err) => {
            tracing::error!(?err);
            0
        }
    };

    if transfer_size > super::common::FS_RESOURCE_ACTION_NOTIFY_THRESHOLD {
        let msg = types::message::Builder::info("Transferring files.");
        let msg = msg.build();
        messages.update(|messages| {
            messages.push(msg);
        });
    }

    match add_fs_resources_to_graph(project, container_path, payload.paths().clone()).await {
        Ok(_) => {
            if transfer_size > super::common::FS_RESOURCE_ACTION_NOTIFY_THRESHOLD {
                let msg = types::message::Builder::success("File transfer complete.");
                let msg = msg.build();
                messages.update(|messages| {
                    messages.push(msg);
                });
            }
        }
        Err(errors) => {
            tracing::error!(?errors);
            todo!();
        }
    }
}

/// Adds file system resources (file or folder) to the project's data graph.
async fn add_fs_resources_to_graph(
    project: ResourceId,
    parent: PathBuf,
    paths: Vec<PathBuf>,
) -> Result<(), Vec<(PathBuf, io::ErrorKind)>> {
    #[derive(Serialize)]
    struct Args {
        resources: Vec<lib::types::AddFsGraphResourceData>,
    }

    let resources = paths
        .into_iter()
        .map(|path| lib::types::AddFsGraphResourceData {
            project: project.clone(),
            path,
            parent: parent.clone(),
            action: local::types::FsResourceAction::Copy, // TODO: Get from user preferences.
        })
        .collect();

    tauri_sys::core::invoke_result::<(), Vec<(PathBuf, lib::command::error::IoErrorKind)>>(
        "add_file_system_resources",
        Args { resources },
    )
    .await
    .map_err(|errors| {
        errors
            .into_iter()
            .map(|(path, err)| (path, err.0))
            .collect()
    })
}

/// Handle a drop event on the project analyses bar.
async fn handle_drop_event_analyses(
    payload: DragDropPayload,
    project: ResourceId,
    messages: types::Messages,
) {
    let transfer_size = match commands::fs::file_size(payload.paths().clone()).await {
        Ok(sizes) => sizes
            .into_iter()
            .reduce(|total, size| total + size)
            .unwrap_or(0),
        Err(err) => {
            tracing::error!(?err);
            0
        }
    };

    if transfer_size > super::common::FS_RESOURCE_ACTION_NOTIFY_THRESHOLD {
        let msg = types::message::Builder::info("Adding analyses.");
        let msg = msg.build();
        messages.update(|messages| messages.push(msg));
    }

    match add_fs_resources_to_analyses(payload.paths().clone(), project).await {
        Ok(_) => {
            if transfer_size > super::common::FS_RESOURCE_ACTION_NOTIFY_THRESHOLD {
                let msg = types::message::Builder::success("Analyses added.");
                let msg = msg.build();
                messages.update(|messages| messages.push(msg));
            }
        }
        Err(err) => {
            let mut msg = types::message::Builder::error("Could not add analyses.");
            msg.body(format!("{err:?}"));
            let msg = msg.build();
            messages.update(|messages| messages.push(msg));
        }
    }
}

async fn add_fs_resources_to_analyses(
    paths: Vec<PathBuf>,
    project: ResourceId,
) -> Result<(), Vec<lib::command::analyses::error::AddAnalyses>> {
    #[derive(Serialize)]
    struct Args {
        project: ResourceId,
        resources: Vec<lib::types::AddFsAnalysisResourceData>,
    }
    let resources = paths
        .into_iter()
        .map(|path| lib::types::AddFsAnalysisResourceData {
            path: path.clone(),
            parent: PathBuf::from("/"),
            action: local::types::FsResourceAction::Copy,
        })
        .collect();

    tauri_sys::core::invoke_result("project_add_analyses", Args { project, resources }).await
}

/// # Returns
/// Project's path, data, and graph.
async fn fetch_project_resources(
    project: ResourceId,
) -> Option<(
    PathBuf,
    db::state::ProjectData,
    db::state::FolderResource<db::state::Graph>,
)> {
    #[derive(Serialize)]
    struct Args {
        project: ResourceId,
    }

    let resources = tauri_sys::core::invoke::<
        Option<(
            PathBuf,
            db::state::ProjectData,
            db::state::FolderResource<db::state::Graph>,
        )>,
    >("project_resources", Args { project })
    .await;

    assert!(if let Some((_, data, _)) = resources.as_ref() {
        data.properties().is_ok()
    } else {
        true
    });

    resources
}

fn handle_event_project(event: lib::Event, project: state::Project) {
    let lib::EventKind::Project(update) = event.kind() else {
        panic!("invalid event kind");
    };

    match update {
        db::event::Project::Graph(_)
        | db::event::Project::Container { .. }
        | db::event::Project::Asset { .. }
        | db::event::Project::AssetFile(_)
        | db::event::Project::Flag { .. } => unreachable!("handled elsewhere"),

        db::event::Project::FolderRemoved => todo!(),
        db::event::Project::Moved(_) => todo!(),
        db::event::Project::Properties(_) => handle_event_project_properties(event, project),
        db::event::Project::Settings(_) => todo!(),
        db::event::Project::Analyses(_) => handle_event_project_analyses(event, project),
        db::event::Project::AnalysisFile(_) => todo!(),
    }
}

fn handle_event_project_properties(event: lib::Event, project: state::Project) {
    let lib::EventKind::Project(db::event::Project::Properties(update)) = event.kind() else {
        panic!("invalid event kind");
    };

    match update {
        db::event::DataResource::Created(_) => todo!(),
        db::event::DataResource::Removed => todo!(),
        db::event::DataResource::Corrupted(io_serde) => todo!(),
        db::event::DataResource::Repaired(_) => todo!(),
        db::event::DataResource::Modified(_) => {
            handle_event_project_properties_modified(event, project)
        }
    }
}

fn handle_event_project_properties_modified(event: lib::Event, project: state::Project) {
    let lib::EventKind::Project(db::event::Project::Properties(db::event::DataResource::Modified(
        update,
    ))) = event.kind()
    else {
        panic!("invalid event kind");
    };

    if project
        .properties()
        .name()
        .with_untracked(|name| *name != update.name)
    {
        project.properties().name().set(update.name.clone());
    }

    if project
        .properties()
        .description()
        .with_untracked(|description| *description != update.description)
    {
        project
            .properties()
            .description()
            .set(update.description.clone());
    }

    if project
        .properties()
        .data_root()
        .with_untracked(|data_root| *data_root != update.data_root)
    {
        project
            .properties()
            .data_root()
            .set(update.data_root.clone());
    }

    if project
        .properties()
        .analysis_root()
        .with_untracked(|analysis_root| *analysis_root != update.analysis_root)
    {
        project
            .properties()
            .analysis_root()
            .set(update.analysis_root.clone());
    }
}

fn handle_event_project_analyses(event: lib::Event, project: state::Project) {
    let lib::EventKind::Project(db::event::Project::Analyses(update)) = event.kind() else {
        panic!("invalid event kind");
    };

    match update {
        db::event::DataResource::Created(_) => todo!(),
        db::event::DataResource::Removed => todo!(),
        db::event::DataResource::Corrupted(_) => todo!(),
        db::event::DataResource::Repaired(_) => todo!(),
        db::event::DataResource::Modified(_) => {
            handle_event_project_analyses_modified(event, project)
        }
    }
}

fn handle_event_project_analyses_modified(event: lib::Event, project: state::Project) {
    let lib::EventKind::Project(db::event::Project::Analyses(db::event::DataResource::Modified(
        update,
    ))) = event.kind()
    else {
        panic!("invalid event kind");
    };

    let analyses = project.analyses().with_untracked(|analyses| {
        let db::state::DataResource::Ok(analyses) = analyses else {
            panic!("invalid state");
        };

        analyses.clone()
    });

    analyses.update(|analyses| {
        analyses.retain(|analysis| {
            update.iter().any(|update_analysis| {
                analysis.properties().with_untracked(|properties| {
                    match (properties, update_analysis.properties()) {
                        (AnalysisKind::Script(properties), AnalysisKind::Script(update)) => {
                            properties.rid() == update.rid()
                        }

                        (
                            AnalysisKind::ExcelTemplate(properties),
                            AnalysisKind::ExcelTemplate(update),
                        ) => properties.rid() == update.rid(),

                        _ => false,
                    }
                })
            })
        });

        for update_analysis in update.iter() {
            if !analyses.iter().any(|analysis| {
                analysis.properties().with_untracked(|properties| {
                    match (properties, update_analysis.properties()) {
                        (AnalysisKind::Script(properties), AnalysisKind::Script(update)) => {
                            properties.rid() == update.rid()
                        }

                        (
                            AnalysisKind::ExcelTemplate(properties),
                            AnalysisKind::ExcelTemplate(update),
                        ) => properties.rid() == update.rid(),

                        _ => false,
                    }
                })
            }) {
                analyses.push(state::Analysis::from_state(update_analysis));
            }
        }
    });

    analyses.with_untracked(|analyses| {
        for update_analysis in update.iter() {
            let update_properties = update_analysis.properties();
            let analysis = analyses
                .iter()
                .find(|analysis| {
                    analysis.properties().with_untracked(|properties| {
                        match (properties, update_properties) {
                            (AnalysisKind::Script(properties), AnalysisKind::Script(update)) => {
                                properties.rid() == update.rid()
                            }

                            (
                                AnalysisKind::ExcelTemplate(properties),
                                AnalysisKind::ExcelTemplate(update),
                            ) => properties.rid() == update.rid(),

                            _ => false,
                        }
                    })
                })
                .unwrap();

            analysis.properties().update(|properties| {
                match (properties, update_analysis.properties()) {
                    (AnalysisKind::Script(properties), AnalysisKind::Script(update)) => {
                        *properties = update.clone();
                    }

                    (
                        AnalysisKind::ExcelTemplate(properties),
                        AnalysisKind::ExcelTemplate(update),
                    ) => {
                        *properties = update.clone();
                    }

                    _ => panic!("analysis kinds do not match"),
                }
            });

            analysis
                .fs_resource()
                .update(|present| *present = update_analysis.fs_resource().clone());
        }
    });
}

fn handle_event_graph(
    event: lib::Event,
    graph: state::Graph,
    workspace_graph_state: state::WorkspaceGraph,
) {
    let lib::EventKind::Project(update) = event.kind() else {
        panic!("invalid event kind");
    };

    match update {
        db::event::Project::FolderRemoved
        | db::event::Project::Moved(_)
        | db::event::Project::Properties(_)
        | db::event::Project::Settings(_)
        | db::event::Project::Analyses(_)
        | db::event::Project::AnalysisFile(_) => unreachable!("handled elsewhere"),

        db::event::Project::Graph(_) => {
            handle_event_graph_graph(event, graph, workspace_graph_state)
        }
        db::event::Project::Container { .. } => {
            handle_event_graph_container(event, graph, workspace_graph_state.selection_resources())
        }
        db::event::Project::Asset { .. } => handle_event_graph_asset(event, graph),
        db::event::Project::AssetFile(_) => handle_event_graph_asset_file(event, graph),
        db::event::Project::Flag { .. } => todo!(),
    }
}

fn handle_event_graph_graph(
    event: lib::Event,
    graph: state::Graph,
    workspace_graph_state: state::WorkspaceGraph,
) {
    let lib::EventKind::Project(db::event::Project::Graph(update)) = event.kind() else {
        panic!("invalid event kind");
    };

    match update {
        db::event::Graph::Created(_) => todo!(),
        db::event::Graph::Inserted { .. } => {
            handle_event_graph_graph_inserted(event, graph, workspace_graph_state)
        }
        db::event::Graph::Renamed { from, to } => handle_event_graph_graph_renamed(event, graph),
        db::event::Graph::Moved { from, to } => todo!(),
        db::event::Graph::Removed(_) => {
            handle_event_graph_graph_removed(event, graph, workspace_graph_state)
        }
    }
}

fn handle_event_graph_graph_inserted(
    event: lib::Event,
    graph: state::Graph,
    workspace_graph_state: state::WorkspaceGraph,
) {
    let lib::EventKind::Project(db::event::Project::Graph(db::event::Graph::Inserted {
        parent,
        graph: subgraph,
    })) = event.kind()
    else {
        panic!("invalid event kind");
    };

    // NB: Must create visibility and selection resource signals first before inserting nodes into graph.
    // Downstream components expect a visibility signal to be present.
    let subgraph = state::Graph::new(subgraph.clone());

    let selection_resources = subgraph.nodes().with_untracked(|nodes| {
        nodes
            .iter()
            .flat_map(|node| {
                let mut resources = vec![];
                node.properties().with_untracked(|properties| {
                    if let db::state::DataResource::Ok(properties) = properties {
                        resources.push(state::workspace_graph::ResourceSelection::new(
                            properties.rid().read_only(),
                            state::workspace_graph::ResourceKind::Container,
                        ))
                    }
                });

                node.assets().with_untracked(|assets| {
                    if let db::state::DataResource::Ok(assets) = assets {
                        let assets = assets.with_untracked(|assets| {
                            assets
                                .iter()
                                .map(|asset| {
                                    state::workspace_graph::ResourceSelection::new(
                                        asset.rid().read_only(),
                                        state::workspace_graph::ResourceKind::Asset,
                                    )
                                })
                                .collect::<Vec<_>>()
                        });

                        resources.extend(assets);
                    }
                });

                resources
            })
            .collect::<Vec<_>>()
    });

    workspace_graph_state
        .selection_resources()
        .extend(selection_resources);

    let visibility_inserted = subgraph.nodes().with_untracked(|nodes| {
        nodes
            .iter()
            .cloned()
            .map(|container| (container, RwSignal::new(true)))
            .collect::<Vec<_>>()
    });

    workspace_graph_state
        .container_visiblity()
        .update(|visibilities| {
            visibilities.extend(visibility_inserted);
        });

    graph
        .insert(common::normalize_path_sep(parent), subgraph)
        .unwrap();
}

fn handle_event_graph_graph_renamed(event: lib::Event, graph: state::Graph) {
    let lib::EventKind::Project(db::event::Project::Graph(db::event::Graph::Renamed { from, to })) =
        event.kind()
    else {
        panic!("invalid event kind");
    };

    graph.rename(from, to).unwrap();
}

fn handle_event_graph_graph_removed(
    event: lib::Event,
    graph: state::Graph,
    workspace_graph_state: state::WorkspaceGraph,
) {
    let lib::EventKind::Project(db::event::Project::Graph(db::event::Graph::Removed(path))) =
        event.kind()
    else {
        panic!("invalid event kind");
    };

    // NB: Must remove nodes first, then remove visibility signals.
    // Downstream components expect a visibility signal to be present.
    let path = common::normalize_path_sep(path);
    let removed = graph.remove(&path).unwrap();

    let removed_ids = removed
        .iter()
        .flat_map(|node| {
            let mut resources = vec![];
            node.properties().with_untracked(|properties| {
                if let db::state::DataResource::Ok(properties) = properties {
                    resources.push(properties.rid().get_untracked());
                }
            });

            node.assets().with_untracked(|assets| {
                if let db::state::DataResource::Ok(assets) = assets {
                    assets.with_untracked(|assets| {
                        let assets = assets.iter().map(|asset| asset.rid().get_untracked());

                        resources.extend(assets);
                    })
                }
            });

            resources
        })
        .collect::<Vec<_>>();
    workspace_graph_state
        .selection_resources()
        .remove(&removed_ids);

    workspace_graph_state
        .container_visiblity()
        .update(|visibilities| {
            visibilities.retain(|(container, _)| {
                graph
                    .nodes()
                    .with_untracked(|nodes| nodes.iter().any(|node| Arc::ptr_eq(node, container)))
            });
        });
}

fn handle_event_graph_container(
    event: lib::Event,
    graph: state::Graph,
    selection_resources: &state::workspace_graph::SelectionResources,
) {
    let lib::EventKind::Project(db::event::Project::Container { update, .. }) = event.kind() else {
        panic!("invalid event kind");
    };

    match update {
        db::event::Container::Properties(_) => {
            handle_event_graph_container_properties(event, graph, selection_resources)
        }
        db::event::Container::Settings(_) => handle_event_graph_container_settings(event, graph),
        db::event::Container::Assets(_) => {
            handle_event_graph_container_assets(event, graph, selection_resources)
        }
    }
}

fn handle_event_graph_container_properties(
    event: lib::Event,
    graph: state::Graph,
    selection_resources: &state::workspace_graph::SelectionResources,
) {
    let lib::EventKind::Project(db::event::Project::Container {
        update: db::event::Container::Properties(update),
        ..
    }) = event.kind()
    else {
        panic!("invalid event kind");
    };

    match update {
        db::event::DataResource::Created(_) => {
            handle_event_graph_container_properties_created(event, graph, selection_resources)
        }
        db::event::DataResource::Removed => todo!(),
        db::event::DataResource::Corrupted(_) => {
            handle_event_graph_container_properties_corrupted(event, graph, selection_resources)
        }
        db::event::DataResource::Repaired(_) => {
            handle_event_graph_container_properties_repaired(event, graph, selection_resources)
        }
        db::event::DataResource::Modified(_) => {
            handle_event_graph_container_properties_modified(event, graph)
        }
    }
}

fn handle_event_graph_container_properties_created(
    event: lib::Event,
    graph: state::Graph,
    selection_resources: &state::workspace_graph::SelectionResources,
) {
    let lib::EventKind::Project(db::event::Project::Container {
        path,
        update: db::event::Container::Properties(db::event::DataResource::Created(update)),
    }) = event.kind()
    else {
        panic!("invalid event kind");
    };
    let container = graph
        .find(common::normalize_path_sep(path))
        .unwrap()
        .unwrap();

    match update {
        Ok(update) => {
            if container
                .properties()
                .with_untracked(|properties| properties.is_err())
            {
                let properties = state::container::Properties::new(
                    update.rid.clone(),
                    update.properties.clone(),
                );

                selection_resources.push(state::workspace_graph::ResourceSelection::new(
                    properties.rid().read_only(),
                    state::workspace_graph::ResourceKind::Container,
                ));

                container.properties().update(|container_properties| {
                    *container_properties = db::state::DataResource::Ok(properties);
                });
            } else {
                update_container_properties(container, update);
            }
        }

        Err(err) => {
            if !container.properties().with(|properties| {
                if let Err(properties_err) = properties {
                    properties_err == err
                } else {
                    false
                }
            }) {
                container
                    .properties()
                    .update(|properties| *properties = Err(err.clone()));
            }

            if !container.analyses().with(|analyses| {
                if let Err(analyses_err) = analyses {
                    analyses_err == err
                } else {
                    false
                }
            }) {
                container
                    .analyses()
                    .update(|analyses| *analyses = Err(err.clone()));
            }
        }
    }
}

fn handle_event_graph_container_properties_repaired(
    event: lib::Event,
    graph: state::Graph,
    selection_resources: &state::workspace_graph::SelectionResources,
) {
    let lib::EventKind::Project(db::event::Project::Container {
        path,
        update: db::event::Container::Properties(db::event::DataResource::Repaired(update)),
    }) = event.kind()
    else {
        panic!("invalid event kind");
    };
    let container = graph
        .find(common::normalize_path_sep(path))
        .unwrap()
        .unwrap();

    assert!(container
        .properties()
        .with_untracked(|properties| properties.is_err()));

    let properties =
        state::container::Properties::new(update.rid.clone(), update.properties.clone());

    selection_resources.push(state::workspace_graph::ResourceSelection::new(
        properties.rid().read_only(),
        state::workspace_graph::ResourceKind::Container,
    ));

    container.properties().update(|container_properties| {
        *container_properties = db::state::DataResource::Ok(properties);
    });
}

fn handle_event_graph_container_properties_corrupted(
    event: lib::Event,
    graph: state::Graph,
    selection_resources: &state::workspace_graph::SelectionResources,
) {
    let lib::EventKind::Project(db::event::Project::Container {
        path,
        update: db::event::Container::Properties(db::event::DataResource::Corrupted(update)),
    }) = event.kind()
    else {
        panic!("invalid event kind");
    };
    let container = graph
        .find(common::normalize_path_sep(path))
        .unwrap()
        .unwrap();

    assert!(container
        .properties()
        .with_untracked(|properties| properties.is_ok()));

    let rid = container
        .properties()
        .with_untracked(|properties| properties.as_ref().unwrap().rid().get_untracked());
    selection_resources.remove(&vec![rid]);

    container.properties().update(|properties| {
        *properties = db::state::DataResource::Err(update.clone());
    });
}

fn handle_event_graph_container_properties_modified(event: lib::Event, graph: state::Graph) {
    let lib::EventKind::Project(db::event::Project::Container {
        path,
        update: db::event::Container::Properties(db::event::DataResource::Modified(update)),
    }) = event.kind()
    else {
        panic!("invalid event kind");
    };

    let container = graph
        .find(common::normalize_path_sep(path))
        .unwrap()
        .unwrap();

    update_container_properties(container, update);
}

fn update_container_properties(
    container: state::graph::Node,
    update: &local::types::StoredContainerProperties,
) {
    container.properties().with_untracked(|properties| {
        let db::state::DataResource::Ok(properties) = properties else {
            panic!("invalid state");
        };

        if properties.rid().with_untracked(|rid| update.rid != *rid) {
            properties.rid().set(update.rid.clone());
        }

        if properties
            .name()
            .with_untracked(|name| update.properties.name != *name)
        {
            properties.name().set(update.properties.name.clone());
        }

        if properties
            .kind()
            .with_untracked(|kind| update.properties.kind != *kind)
        {
            properties.kind().set(update.properties.kind.clone());
        }

        if properties
            .description()
            .with_untracked(|description| update.properties.description != *description)
        {
            properties
                .description()
                .set(update.properties.description.clone());
        }

        if properties
            .tags()
            .with_untracked(|tags| update.properties.tags != *tags)
        {
            properties.tags().set(update.properties.tags.clone());
        }

        update_metadata(properties.metadata(), &update.properties.metadata);
    });

    // NB: Can not nest signal updates or borrow error will occur.
    container.analyses().with_untracked(|analyses| {
        let db::state::DataResource::Ok(analyses) = analyses else {
            panic!("invalid state");
        };

        analyses.update(|analyses| {
            analyses.retain(|association| {
                update
                    .analyses
                    .iter()
                    .any(|assoc| assoc.analysis() == association.analysis())
            });

            let new = update
                .analyses
                .iter()
                .filter_map(|association_update| {
                    if !analyses
                        .iter()
                        .any(|association| association.analysis() == association_update.analysis())
                    {
                        Some(state::AnalysisAssociation::new(association_update.clone()))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            analyses.extend(new);
        });

        analyses.with_untracked(|analyses| {
            for association in analyses.iter() {
                let Some(association_update) = update
                    .analyses
                    .iter()
                    .find(|update| update.analysis() == association.analysis())
                else {
                    continue;
                };

                if association
                    .autorun()
                    .with_untracked(|autorun| association_update.autorun != *autorun)
                {
                    association.autorun().set(association_update.autorun);
                }

                if association
                    .priority()
                    .with_untracked(|priority| association_update.priority != *priority)
                {
                    association.priority().set(association_update.priority);
                }
            }
        });
    });
}

fn handle_event_graph_container_settings(event: lib::Event, graph: state::Graph) {
    let lib::EventKind::Project(db::event::Project::Container {
        path,
        update: db::event::Container::Settings(update),
    }) = event.kind()
    else {
        panic!("invalid event kind");
    };

    let container = graph
        .find(common::normalize_path_sep(path))
        .unwrap()
        .unwrap();

    match update {
        db::event::DataResource::Created(update) => match update {
            db::state::DataResource::Err(err) => {
                container
                    .settings()
                    .set(db::state::DataResource::Err(err.clone()));
            }

            db::state::DataResource::Ok(update) => {
                if container
                    .settings()
                    .with_untracked(|settings| settings.is_err())
                {
                    container.settings().set(db::state::DataResource::Ok(
                        state::container::Settings::new(update.clone()),
                    ));
                } else {
                    container.settings().with_untracked(|settings| {
                        let db::state::DataResource::Ok(settings) = settings else {
                            unreachable!("invalid state");
                        };

                        settings.creator().set(update.creator.clone());
                        settings.created().set(update.created.clone());
                        settings.permissions().set(update.permissions.clone());
                    });
                }
            }
        },
        db::event::DataResource::Removed => todo!(),
        db::event::DataResource::Corrupted(_) => todo!(),
        db::event::DataResource::Repaired(_) => todo!(),
        db::event::DataResource::Modified(update) => {
            container.settings().with_untracked(|settings| {
                let db::state::DataResource::Ok(settings) = settings else {
                    panic!("invalid state");
                };

                settings.creator().set(update.creator.clone());
                settings.created().set(update.created.clone());
                settings.permissions().set(update.permissions.clone());
            });
        }
    }
}

fn handle_event_graph_container_assets(
    event: lib::Event,
    graph: state::Graph,
    selection_resources: &state::workspace_graph::SelectionResources,
) {
    let lib::EventKind::Project(db::event::Project::Container {
        path: _path,
        update: db::event::Container::Assets(update),
    }) = event.kind()
    else {
        panic!("invalid event kind");
    };

    match update {
        db::event::DataResource::Created(_) => {
            handle_event_graph_container_assets_created(event, graph, selection_resources)
        }
        db::event::DataResource::Removed => todo!(),
        db::event::DataResource::Corrupted(_) => {
            handle_event_graph_container_assets_corrupted(event, graph, selection_resources)
        }
        db::event::DataResource::Repaired(_) => {
            handle_event_graph_container_assets_repaired(event, graph, selection_resources)
        }
        db::event::DataResource::Modified(_) => {
            handle_event_graph_container_assets_modified(event, graph, selection_resources)
        }
    }
}

fn handle_event_graph_container_assets_created(
    event: lib::Event,
    graph: state::Graph,
    selection_resources: &state::workspace_graph::SelectionResources,
) {
    let lib::EventKind::Project(db::event::Project::Container {
        path,
        update: db::event::Container::Assets(db::event::DataResource::Created(update)),
    }) = event.kind()
    else {
        panic!("invalid event kind");
    };

    let container = graph
        .find(common::normalize_path_sep(path))
        .unwrap()
        .unwrap();

    match update {
        db::state::DataResource::Err(err) => {
            container
                .assets()
                .set(db::state::DataResource::Err(err.clone()));
        }

        db::state::DataResource::Ok(update) => {
            if container.assets().with_untracked(|assets| assets.is_err()) {
                let assets = update
                    .iter()
                    .map(|asset| state::Asset::new(asset.clone()))
                    .collect::<Vec<_>>();

                let resources = assets
                    .iter()
                    .map(|asset| {
                        state::workspace_graph::ResourceSelection::new(
                            asset.rid().read_only(),
                            state::workspace_graph::ResourceKind::Asset,
                        )
                    })
                    .collect::<Vec<state::workspace_graph::ResourceSelection>>();
                selection_resources.extend(resources);

                container
                    .assets()
                    .set(db::state::DataResource::Ok(RwSignal::new(assets)));
            } else {
                let removed = container.assets().with_untracked(|assets| {
                    assets.as_ref().unwrap().with_untracked(|assets| {
                        assets
                            .iter()
                            .filter_map(|asset| {
                                (!update.iter().any(|update| {
                                    asset.rid().with_untracked(|rid| update.rid() == rid)
                                }))
                                .then_some(asset.rid().read_only())
                            })
                            .collect::<Vec<_>>()
                    })
                });

                let (modified, added): (Vec<_>, Vec<_>) = update.iter().partition(|update| {
                    container.assets().with_untracked(|assets| {
                        assets.as_ref().unwrap().with_untracked(|assets| {
                            assets
                                .iter()
                                .any(|asset| asset.rid().with_untracked(|rid| update.rid() == rid))
                        })
                    })
                });

                let added = added
                    .into_iter()
                    .map(|update| state::Asset::new(update.clone()))
                    .collect::<Vec<_>>();

                let removed_ids = removed.iter().map(|asset| asset.get_untracked()).collect();
                selection_resources.remove(&removed_ids);

                let added_selection_resources = added
                    .iter()
                    .map(|asset| {
                        state::workspace_graph::ResourceSelection::new(
                            asset.rid().read_only(),
                            state::workspace_graph::ResourceKind::Asset,
                        )
                    })
                    .collect();
                selection_resources.extend(added_selection_resources);

                container.assets().update(|assets| {
                    let db::state::DataResource::Ok(assets) = assets else {
                        panic!("invalid state");
                    };

                    assets.update(|assets| {
                        assets.retain(|asset| {
                            !removed.iter().any(|removed| {
                                removed.with_untracked(|removed| {
                                    asset.rid().with_untracked(|asset| removed == asset)
                                })
                            })
                        });

                        modified.into_iter().for_each(|update| {
                            let Some(asset) = assets.iter().find(|asset| {
                                asset.rid().with_untracked(|rid| rid == update.rid())
                            }) else {
                                panic!("invalid state");
                            };

                            update_asset(asset, update);
                        });

                        assets.extend(added);
                    });
                });
            }
        }
    }
}

fn handle_event_graph_container_assets_modified(
    event: lib::Event,
    graph: state::Graph,
    selection_resources: &state::workspace_graph::SelectionResources,
) {
    let lib::EventKind::Project(db::event::Project::Container {
        path,
        update: db::event::Container::Assets(db::event::DataResource::Modified(update)),
    }) = event.kind()
    else {
        panic!("invalid event kind");
    };

    let container = graph
        .find(common::normalize_path_sep(path))
        .unwrap()
        .unwrap();

    // NB: Can not nest signal updates or borrow error will occur.
    let (assets_update, assets_new): (Vec<_>, Vec<_>) =
        container.assets().with_untracked(|assets| {
            let db::state::DataResource::Ok(assets) = assets else {
                panic!("invalid state");
            };

            assets.with_untracked(|assets| {
                update.iter().partition(|update_asset| {
                    assets
                        .iter()
                        .any(|asset| asset.rid().with_untracked(|rid| rid == update_asset.rid()))
                })
            })
        });

    let assets_new = assets_new
        .into_iter()
        .map(|asset| state::Asset::new(asset.clone()))
        .collect::<Vec<_>>();

    let removed = container.assets().with_untracked(|assets| {
        let db::state::DataResource::Ok(assets) = assets else {
            panic!("invalid state");
        };

        assets.with_untracked(|assets| {
            assets
                .iter()
                .filter_map(|asset| {
                    (!update
                        .iter()
                        .any(|update| asset.rid().with_untracked(|rid| update.rid() == rid)))
                    .then_some(asset.rid().read_only())
                })
                .collect::<Vec<_>>()
        })
    });

    let removed_ids = removed
        .iter()
        .map(|removed| removed.get_untracked())
        .collect();
    selection_resources.remove(&removed_ids);

    let selection_resources_new = assets_new
        .iter()
        .map(|asset| {
            state::workspace_graph::ResourceSelection::new(
                asset.rid().read_only(),
                state::workspace_graph::ResourceKind::Asset,
            )
        })
        .collect();
    selection_resources.extend(selection_resources_new);

    container.assets().with_untracked(|assets| {
        let db::state::DataResource::Ok(assets) = assets else {
            panic!("invalid state");
        };

        assets.update(|assets| {
            assets.retain(|asset| {
                !removed.iter().any(|removed| {
                    asset
                        .rid()
                        .with_untracked(|rid| removed.with_untracked(|removed| removed == rid))
                })
            });

            assets.extend(assets_new);
        });
    });

    for asset_update in assets_update {
        let asset = container.assets().with_untracked(|assets| {
            let db::state::DataResource::Ok(assets) = assets else {
                panic!("invalid state");
            };

            assets
                .with_untracked(|assets| {
                    assets
                        .iter()
                        .find(|asset| asset.rid().with_untracked(|rid| rid == asset_update.rid()))
                        .cloned()
                })
                .unwrap()
        });

        update_asset(&asset, asset_update);
    }
}

fn update_asset(asset: &state::Asset, update: &db::state::Asset) {
    assert!(asset.rid().with_untracked(|rid| rid == update.rid()));

    if asset
        .name()
        .with_untracked(|name| name != &update.properties.name)
    {
        asset
            .name()
            .update(|name| *name = update.properties.name.clone());
    }

    if asset
        .kind()
        .with_untracked(|kind| kind != &update.properties.kind)
    {
        asset
            .kind()
            .update(|kind| *kind = update.properties.kind.clone());
    }

    if asset
        .description()
        .with_untracked(|description| description != &update.properties.description)
    {
        asset
            .description()
            .update(|description| *description = update.properties.description.clone());
    }

    if asset
        .tags()
        .with_untracked(|tags| tags != &update.properties.tags)
    {
        asset
            .tags()
            .update(|tags| *tags = update.properties.tags.clone());
    }

    if asset.path().with_untracked(|path| path != &update.path) {
        asset.path().update(|path| *path = update.path.clone());
    }

    if asset
        .fs_resource()
        .with_untracked(|fs_resource| fs_resource.is_present() != update.is_present())
    {
        asset.fs_resource().update(|fs_resource| {
            *fs_resource = if update.is_present() {
                db::state::FileResource::Present
            } else {
                db::state::FileResource::Absent
            }
        });
    }

    if asset
        .created()
        .with_untracked(|created| created != update.properties.created())
    {
        asset
            .created()
            .update(|created| *created = (*update).properties.created().clone());
    }

    if asset
        .creator()
        .with_untracked(|creator| creator != &update.properties.creator)
    {
        asset
            .creator()
            .update(|creator| *creator = (*update).properties.creator.clone());
    }

    update_metadata(asset.metadata(), &update.properties.metadata);
}

fn handle_event_graph_container_assets_corrupted(
    event: lib::Event,
    graph: state::Graph,
    selection_resources: &state::workspace_graph::SelectionResources,
) {
    let lib::EventKind::Project(db::event::Project::Container {
        path,
        update: db::event::Container::Assets(db::event::DataResource::Corrupted(err)),
    }) = event.kind()
    else {
        panic!("invalid event kind");
    };

    let container = graph
        .find(common::normalize_path_sep(path))
        .unwrap()
        .unwrap();

    let rids = container.assets().with_untracked(|assets| {
        assets.as_ref().unwrap().with_untracked(|assets| {
            assets
                .iter()
                .map(|asset| asset.rid().get_untracked())
                .collect::<Vec<_>>()
        })
    });
    selection_resources.remove(&rids);

    container.assets().update(|container_assets| {
        *container_assets = db::state::DataResource::Err(err.clone());
    });
}

fn handle_event_graph_container_assets_repaired(
    event: lib::Event,
    graph: state::Graph,
    selection_resources: &state::workspace_graph::SelectionResources,
) {
    let lib::EventKind::Project(db::event::Project::Container {
        path,
        update: db::event::Container::Assets(db::event::DataResource::Repaired(assets)),
    }) = event.kind()
    else {
        panic!("invalid event kind");
    };

    let container = graph
        .find(common::normalize_path_sep(path))
        .unwrap()
        .unwrap();

    let assets = assets
        .into_iter()
        .map(|asset| state::Asset::new(asset.clone()))
        .collect::<Vec<_>>();

    let selections = assets
        .iter()
        .map(|asset| {
            state::workspace_graph::ResourceSelection::new(
                asset.rid().read_only(),
                state::workspace_graph::ResourceKind::Asset,
            )
        })
        .collect::<Vec<_>>();
    selection_resources.extend(selections);

    container.assets().update(|container_assets| {
        *container_assets = db::state::DataResource::Ok(RwSignal::new(assets));
    });
}

fn handle_event_graph_asset(event: lib::Event, graph: state::Graph) {
    let lib::EventKind::Project(db::event::Project::Asset {
        container,
        asset,
        update,
    }) = event.kind()
    else {
        panic!("invalid event kind");
    };

    let container = graph
        .find(common::normalize_path_sep(container))
        .unwrap()
        .unwrap();

    match update {
        db::event::Asset::FileCreated | db::event::Asset::FileRemoved => {
            let fs_resource = container.assets().with_untracked(|assets| {
                let db::state::DataResource::Ok(assets) = assets else {
                    todo!();
                };
                assets.with_untracked(|assets| {
                    assets
                        .iter()
                        .find(|asset_state| asset_state.rid().with_untracked(|rid| rid == asset))
                        .unwrap()
                        .fs_resource()
                })
            });

            match update {
                db::event::Asset::FileCreated => fs_resource.set(db::state::FileResource::Present),
                db::event::Asset::FileRemoved => fs_resource.set(db::state::FileResource::Absent),
                _ => unreachable!(),
            };
        }
        db::event::Asset::Properties(update) => {
            container.assets().with_untracked(|assets| {
                let db::state::DataResource::Ok(assets) = assets else {
                    todo!();
                };

                let asset = assets.with_untracked(|assets| {
                    assets
                        .iter()
                        .find(|asset_state| asset_state.rid().with_untracked(|rid| rid == asset))
                        .unwrap()
                        .clone()
                });

                if asset
                    .fs_resource()
                    .with_untracked(|fs_resource| fs_resource.is_present() != update.is_present())
                {
                    let fs_resource = if update.is_present() {
                        db::state::FileResource::Present
                    } else {
                        db::state::FileResource::Absent
                    };
                    asset.fs_resource().set(fs_resource);
                }

                if asset
                    .name()
                    .with_untracked(|name| *name != update.properties.name)
                {
                    asset.name().set(update.properties.name.clone());
                }

                if asset
                    .kind()
                    .with_untracked(|kind| *kind != update.properties.kind)
                {
                    asset.kind().set(update.properties.kind.clone());
                }

                if asset
                    .description()
                    .with_untracked(|description| *description != update.properties.description)
                {
                    asset
                        .description()
                        .set(update.properties.description.clone());
                }

                if asset
                    .tags()
                    .with_untracked(|tags| *tags != update.properties.tags)
                {
                    asset.tags().set(update.properties.tags.clone());
                }

                asset.metadata().update(|metadata| {
                    metadata.retain(|(key, _)| {
                        update
                            .properties
                            .metadata
                            .iter()
                            .any(|(update_key, _)| key == update_key)
                    });

                    update
                        .properties
                        .metadata
                        .iter()
                        .for_each(|(update_key, update_value)| {
                            if let Some(value) = metadata.iter().find_map(|(key, value)| {
                                if update_key == key {
                                    Some(value)
                                } else {
                                    None
                                }
                            }) {
                                if value.with_untracked(|value| value != update_value) {
                                    value.set(update_value.clone())
                                }
                            } else {
                                metadata.push((
                                    update_key.clone(),
                                    RwSignal::new(update_value.clone()),
                                ));
                            }
                        });
                });
            });
        }
    }
}

fn handle_event_graph_asset_file(event: lib::Event, graph: state::Graph) {
    let lib::EventKind::Project(db::event::Project::AssetFile(kind)) = event.kind() else {
        panic!("invalid event kind");
    };

    match kind {
        db::event::AssetFile::Created(path) => todo!(),
        db::event::AssetFile::Removed(path) => todo!(),
        db::event::AssetFile::Renamed { from, to } => todo!(),
        db::event::AssetFile::Moved { from, to } => todo!(),
    }
}

fn update_metadata(metadata: RwSignal<state::Metadata>, update: &syre_core::project::Metadata) {
    // NB: Can not nest signal updates or borrow error will occur.
    let (keys_update, keys_new): (Vec<_>, Vec<_>) = metadata.with_untracked(|metadata| {
        update
            .keys()
            .partition(|key| metadata.iter().any(|(k, _)| k == *key))
    });

    metadata.update(|metadata| {
        metadata.retain(|(key, _)| keys_update.contains(&key));

        let new = update
            .iter()
            .filter_map(|(key, value)| {
                if keys_new.contains(&key) {
                    Some((key.clone(), RwSignal::new(value.clone())))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        metadata.extend(new);
    });

    metadata.with_untracked(|metadata| {
        for (update_key, update_value) in update.iter().filter(|(key, _)| keys_update.contains(key))
        {
            let value = metadata
                .iter()
                .find_map(
                    |(key, value)| {
                        if key == update_key {
                            Some(value)
                        } else {
                            None
                        }
                    },
                )
                .unwrap();

            if value.with_untracked(|value| update_value != value) {
                value.set(update_value.clone());
            }
        }
    })
}
