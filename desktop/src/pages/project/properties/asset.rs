use super::detail_popout_top;
use crate::{components, pages::project::state, types};
use leptos::{either::either, ev::MouseEvent, prelude::*};
use leptos_icons::Icon;

#[derive(PartialEq, Clone, Copy)]
enum EditorView {
    Properties,
    Flags,
}

#[component]
pub fn Editor(asset: state::Asset) -> impl IntoView {
    let editor_view = RwSignal::new(EditorView::Properties);
    provide_context(editor_view);

    {
        let asset = asset.clone();
        move || {
            either!(
                editor_view(),
                EditorView::Properties => view! { <properties::Editor asset=asset.clone() /> },
                EditorView::Flags => view! { <flags::Editor asset=asset.clone() /> },
            )
        }
    }
}

#[component]
fn Header() -> impl IntoView {
    let editor_view = expect_context::<RwSignal<EditorView>>();
    let set_editor_view = move |e: MouseEvent, view: EditorView| {
        if e.button() != types::MouseButton::Primary {
            return;
        }
        e.stop_propagation();

        if view != editor_view() {
            editor_view.set(view);
        }
    };

    view! {
        <div class="pb-2">
            <div class="text-center pt-1 pb-2">
                <h3 class="font-primary">"Asset"</h3>
            </div>
            <div class="border-y flex gap-1 items-center px-2">
                <button
                    on:mousedown=move |e| set_editor_view(e, EditorView::Properties)
                    class=(
                        ["bg-secondary-50", "dark:bg-secondary-600"],
                        move || matches!(editor_view(), EditorView::Properties),
                    )
                    class="p-1 hover:bg-secondary-50 dark:hover:bg-secondary-600"
                >
                    <Icon icon=components::icon::Edit />
                </button>
                <button
                    on:mousedown=move |e| set_editor_view(e, EditorView::Flags)
                    class=(
                        ["bg-secondary-50", "dark:bg-secondary-600"],
                        move || matches!(editor_view(), EditorView::Flags),
                    )
                    class="p-1 hover:bg-secondary-50 dark:hover:bg-secondary-600"
                >
                    <Icon icon=components::icon::Flag />
                </button>
            </div>
        </div>
    }
}

mod properties {
    use super::super::{InputDebounce, PopoutPortal};
    use crate::{components, pages::project::state, types};
    use description::Editor as Description;
    use kind::Editor as Kind;
    use leptos::{
        ev::{Event, MouseEvent},
        html,
        portal::Portal,
        prelude::*,
    };
    use leptos_icons::Icon;
    use metadata::{AddDatum, Editor as Metadata};
    use name::Editor as Name;
    use serde::Serialize;
    use std::path::PathBuf;
    use syre_core::types::ResourceId;
    use tags::Editor as Tags;

    #[derive(derive_more::Deref, Clone)]
    struct ActiveAsset(state::Asset);

    #[component]
    pub fn Editor(asset: state::Asset) -> impl IntoView {
        let popout_portal = expect_context::<PopoutPortal>();
        let add_metadatum_visible = RwSignal::new(false);
        let wrapper_node = NodeRef::<html::Div>::new();
        let metadata_node = NodeRef::<html::Div>::new();
        provide_context(ActiveAsset(asset.clone()));

        let show_add_metadatum = move |e: MouseEvent| {
            if e.button() == types::MouseButton::Primary {
                let wrapper = wrapper_node.get_untracked().unwrap();
                let base = metadata_node.get_untracked().unwrap();
                let portal = popout_portal.get_untracked().unwrap();

                let top = super::detail_popout_top(&portal, &base, &wrapper);
                (*portal)
                    .style()
                    .set_property("top", &format!("{top}px"))
                    .unwrap();

                add_metadatum_visible.set(true);
            }
        };

        let scroll = move |_: Event| {
            let wrapper = wrapper_node.get_untracked().unwrap();
            let base = metadata_node.get_untracked().unwrap();
            let portal = popout_portal.get_untracked().unwrap();

            let top = super::detail_popout_top(&portal, &base, &wrapper);
            (*portal)
                .style()
                .set_property("top", &format!("{top}px"))
                .unwrap();
        };

        view! {
            <div
                node_ref=wrapper_node
                on:scroll=scroll
                class="overflow-y-auto h-full scrollbar-thin"
            >
                <super::Header />
                <form on:submit=|e| e.prevent_default()>
                    <div class="pb-1 px-1">
                        <label>
                            <span class="block">"Name"</span>
                            <Name />
                        </label>
                    </div>
                    <div class="pb-1 px-1">
                        <label>
                            <span class="block">"Type"</span>
                            <Kind />
                        </label>
                    </div>
                    <div class="pb-1 px-1">
                        <label>
                            <span class="block">"Description"</span>
                            <Description />
                        </label>
                    </div>
                    <div class="pb-4 px-1">
                        <label>
                            <span class="block">"Tags"</span>
                            <Tags />
                        </label>
                    </div>
                    <div
                        node_ref=metadata_node
                        class="relative py-4 border-t border-t-secondary-200 dark:border-t-secondary-700"
                    >
                        <label class="px-1 block">
                            <div class="flex">
                                <span class="grow">"Metadata"</span>
                                <span>
                                    // TODO: Button hover state seems to be triggered by hovering over
                                    // parent section.
                                    <button
                                        on:mousedown=show_add_metadatum
                                        class=(
                                            ["bg-primary-400", "dark:bg-primary-700"],
                                            add_metadatum_visible,
                                        )

                                        class=(
                                            ["hover:bg-secondary-200", "dark:hover:bg-secondary-700"],
                                            move || !add_metadatum_visible(),
                                        )

                                        class="aspect-square w-full rounded-xs"
                                    >
                                        <Icon icon=components::icon::Add />
                                    </button>

                                </span>
                            </div>
                            <Metadata />
                        </label>
                    </div>
                </form>
                <div class="px-1 py-2 border-t dark:border-t-secondary-700 overflow-x-auto select-all text-nowrap scrollbar-thin">
                    {move || asset.path().with(|path| path.to_string_lossy().to_string())}
                </div>
                <Show
                    when=move || add_metadatum_visible() && popout_portal.get().is_some()
                    fallback=|| view! {}
                >
                    {move || {
                        let mount = popout_portal.get().unwrap();
                        let mount = (*mount).clone();
                        view! {
                            <Portal mount>
                                <AddDatum onclose=move || add_metadatum_visible.set(false) />
                            </Portal>
                        }
                    }}
                </Show>
            </div>
        }
    }

    mod name {
        use super::{update_properties, ActiveAsset, InputDebounce};
        use crate::{components::form::debounced::InputText, pages::project::state, types};
        use leptos::{prelude::*, task::spawn_local};

        #[component]
        pub fn Editor() -> impl IntoView {
            let project = expect_context::<state::Project>();
            let graph = expect_context::<state::Graph>();
            let asset = expect_context::<ActiveAsset>();
            let messages = expect_context::<types::Messages>();
            let input_debounce = expect_context::<InputDebounce>();

            let input_value = Signal::derive({
                let value = asset.name().read_only();
                move || value.with(|value| value.clone().unwrap_or(String::new()))
            });

            let oninput = Callback::new({
                let messages = messages.write_only();
                move |value: String| {
                    let mut properties = asset.as_properties();
                    let value = value.trim();
                    properties.name = if value.is_empty() {
                        None
                    } else {
                        Some(value.to_string())
                    };

                    let project = project.rid().get_untracked();
                    let node = asset
                        .rid()
                        .with_untracked(|rid| graph.find_by_asset_id(rid).unwrap());
                    let container_path = graph.path(&node).unwrap();
                    let asset_path = asset.path().get_untracked();
                    let messages = messages.clone();
                    spawn_local(async move {
                        if let Err(err) =
                            update_properties(project, container_path, asset_path, properties).await
                        {
                            tracing::error!(?err);
                            let mut msg =
                                types::message::Builder::error("Could not save container");
                            msg.body(format!("{err:?}"));
                            messages.update(|messages| messages.push(msg.build()));
                        }
                    });
                }
            });

            view! {
                <InputText
                    value=input_value
                    oninput
                    debounce=*input_debounce
                    attr:class="input-compact"
                />
            }
        }
    }

    mod kind {
        use super::{
            super::super::common::kind::Editor as KindEditor, update_properties, ActiveAsset,
            InputDebounce,
        };
        use crate::{pages::project::state, types};
        use leptos::{prelude::*, task::spawn_local};

        #[component]
        pub fn Editor() -> impl IntoView {
            let project = expect_context::<state::Project>();
            let graph = expect_context::<state::Graph>();
            let asset = expect_context::<ActiveAsset>();
            let messages = expect_context::<types::Messages>();
            let input_debounce = expect_context::<InputDebounce>();

            let oninput = Callback::new({
                let asset = asset.clone();
                let messages = messages.write_only();
                move |value: Option<String>| {
                    let mut properties = asset.as_properties();
                    properties.kind = value;

                    let project = project.rid().get_untracked();
                    let node = asset
                        .rid()
                        .with_untracked(|rid| graph.find_by_asset_id(rid).unwrap());
                    let container_path = graph.path(&node).unwrap();
                    let asset_path = asset.path().get_untracked();
                    let messages = messages.clone();
                    spawn_local(async move {
                        if let Err(err) =
                            update_properties(project, container_path, asset_path, properties).await
                        {
                            tracing::error!(?err);
                            let mut msg =
                                types::message::Builder::error("Could not save container");
                            msg.body(format!("{err:?}"));
                            messages.update(|messages| messages.push(msg.build()));
                        }
                    });
                }
            });

            view! {
                <KindEditor
                    value=asset.kind().read_only()
                    oninput
                    debounce=*input_debounce
                    class="input-compact"
                />
            }
        }
    }

    mod description {
        use super::{
            super::super::common::description::Editor as DescriptionEditor, update_properties,
            ActiveAsset, InputDebounce,
        };
        use crate::{pages::project::state, types};
        use leptos::{prelude::*, task::spawn_local};

        #[component]
        pub fn Editor() -> impl IntoView {
            let project = expect_context::<state::Project>();
            let graph = expect_context::<state::Graph>();
            let asset = expect_context::<ActiveAsset>();
            let messages = expect_context::<types::Messages>();
            let input_debounce = expect_context::<InputDebounce>();

            let oninput = Callback::new({
                let asset = asset.clone();
                let messages = messages.write_only();
                move |value: Option<String>| {
                    let mut properties = asset.as_properties();
                    properties.description = value;

                    let project = project.rid().get_untracked();
                    let node = asset
                        .rid()
                        .with_untracked(|rid| graph.find_by_asset_id(rid).unwrap());
                    let container_path = graph.path(&node).unwrap();
                    let asset_path = asset.path().get_untracked();
                    let messages = messages.clone();
                    spawn_local(async move {
                        if let Err(err) =
                            update_properties(project, container_path, asset_path, properties).await
                        {
                            tracing::error!(?err);
                            let mut msg =
                                types::message::Builder::error("Could not save container");
                            msg.body(format!("{err:?}"));
                            messages.update(|messages| messages.push(msg.build()));
                        }
                    });
                }
            });

            view! {
                <DescriptionEditor
                    value=asset.description().read_only()
                    oninput
                    debounce=*input_debounce
                    class="input-compact w-full align-top"
                />
            }
        }
    }

    mod tags {
        use super::{
            super::super::common::tags::Editor as TagsEditor, update_properties, ActiveAsset,
            InputDebounce,
        };
        use crate::{pages::project::state, types};
        use leptos::{prelude::*, task::spawn_local};

        #[component]
        pub fn Editor() -> impl IntoView {
            let project = expect_context::<state::Project>();
            let graph = expect_context::<state::Graph>();
            let asset = expect_context::<ActiveAsset>();
            let messages = expect_context::<types::Messages>();
            let input_debounce = expect_context::<InputDebounce>();

            let oninput = Callback::new({
                let asset = asset.clone();
                let messages = messages.write_only();
                move |value: Vec<String>| {
                    let mut properties = asset.as_properties();
                    properties.tags = value;

                    let project = project.rid().get_untracked();
                    let node = asset
                        .rid()
                        .with_untracked(|rid| graph.find_by_asset_id(rid).unwrap());
                    let container_path = graph.path(&node).unwrap();
                    let asset_path = asset.path().get_untracked();
                    let messages = messages.clone();
                    spawn_local(async move {
                        if let Err(err) =
                            update_properties(project, container_path, asset_path, properties).await
                        {
                            tracing::error!(?err);
                            let mut msg =
                                types::message::Builder::error("Could not save container");
                            msg.body(format!("{err:?}"));
                            messages.update(|messages| messages.push(msg.build()));
                        }
                    });
                }
            });

            view! {
                <TagsEditor
                    value=asset.tags().read_only()
                    oninput
                    debounce=*input_debounce
                    class="input-compact"
                />
            }
        }
    }

    mod metadata {
        use super::{
            super::super::common::metadata::{AddDatum as AddDatumEditor, ValueEditor},
            update_properties, ActiveAsset, InputDebounce,
        };
        use crate::{
            components::{self, DetailPopout},
            pages::project::state,
            types,
        };
        use leptos::{ev::MouseEvent, prelude::*, task::spawn_local};
        use leptos_icons::Icon;
        use syre_core::types::{ResourceId, Value};

        #[component]
        pub fn Editor() -> impl IntoView {
            let asset = expect_context::<ActiveAsset>();
            let value_sorted = move || {
                let mut value = asset.metadata().get();
                value.sort_by_key(|(key, _)| key.clone());
                value
            };

            view! {
                <For each=value_sorted key=|(key, _)| key.clone() let:datum>
                    {move || {
                        let (key, value) = &datum;
                        view! { <DatumEditor key=key.clone() value=value.read_only() /> }
                    }}
                </For>
            }
        }

        #[component]
        pub fn AddDatum(#[prop(optional, into)] onclose: Option<Callback<()>>) -> impl IntoView {
            let project = expect_context::<state::Project>();
            let graph = expect_context::<state::Graph>();
            let asset = expect_context::<ActiveAsset>();
            let keys = {
                let asset = asset.clone();
                move || {
                    asset.metadata().with(|metadata| {
                        metadata
                            .iter()
                            .map(|(key, _)| key.clone())
                            .collect::<Vec<_>>()
                    })
                }
            };

            let onadd = Callback::new({
                let asset = asset.clone();
                move |(key, value): (String, Value)| {
                    assert!(!key.is_empty());
                    assert!(!asset
                        .metadata()
                        .with_untracked(|metadata| metadata.iter().any(|(k, _)| *k == key)));

                    let mut properties = asset.as_properties();
                    let mut metadata = asset
                        .metadata()
                        .with_untracked(|metadata| metadata.as_properties());
                    metadata.insert(key, value);
                    properties.metadata = metadata;

                    let node = asset
                        .rid()
                        .with_untracked(|rid| graph.find_by_asset_id(rid).unwrap());
                    let container_path = graph.path(&node).unwrap();
                    let asset_path = asset.path().get_untracked();
                    let project = project.rid().get_untracked();
                    spawn_local(async move {
                        if let Err(err) =
                            update_properties(project, container_path, asset_path, properties).await
                        {
                            tracing::error!(?err);
                            todo!()
                        } else {
                            if let Some(onclose) = onclose {
                                onclose.run(());
                            }
                        }
                    });
                }
            });

            let close_popout = Callback::new(move |_| {
                if let Some(onclose) = onclose {
                    onclose.run(());
                }
            });

            view! {
                <DetailPopout title="Add metadata" onclose=close_popout>
                    <AddDatumEditor keys=Signal::derive(keys) onadd class="w-full px-1" />
                </DetailPopout>
            }
        }

        #[component]
        pub fn DatumEditor(key: String, value: ReadSignal<Value>) -> impl IntoView {
            assert!(!key.is_empty());
            let project = expect_context::<state::Project>();
            let graph = expect_context::<state::Graph>();
            let asset = expect_context::<ActiveAsset>();
            let messages = expect_context::<types::Messages>();
            let input_debounce = expect_context::<InputDebounce>();

            let (input_value, set_input_value) = signal(value.get_untracked());
            let oninput = Callback::new(set_input_value);

            // TODO: Handle errors with messages.
            // See https://github.com/leptos-rs/leptos/issues/2041
            let _ = Effect::watch(
                input_value,
                {
                    let project = project.clone();
                    let graph = graph.clone();
                    let asset = asset.clone();
                    let key = key.clone();
                    move |value, _, asset_id| -> ResourceId {
                        // let messages = messages.write_only();
                        if asset.rid().with_untracked(|rid| {
                            if let Some(asset_id) = asset_id {
                                *rid != asset_id
                            } else {
                                false
                            }
                        }) {
                            return asset.rid().get_untracked();
                        }

                        let value = match value {
                            Value::String(value) => Value::String(value.trim().to_string()),
                            Value::Quantity { magnitude, unit } => Value::Quantity {
                                magnitude: magnitude.clone(),
                                unit: unit.trim().to_string(),
                            },
                            Value::Null | Value::Bool(_) | Value::Number(_) | Value::Array(_) => {
                                value.clone()
                            }
                        };
                        let mut properties = asset.as_properties();
                        properties.metadata.insert(key.clone(), value);

                        spawn_local({
                            let project = project.rid().get_untracked();
                            let node = asset
                                .rid()
                                .with_untracked(|rid| graph.find_by_asset_id(rid).unwrap());
                            let container_path = graph.path(&node).unwrap();
                            let asset_path = asset.path().get_untracked();
                            let messages = messages.clone();
                            async move {
                                if let Err(err) = update_properties(
                                    project,
                                    container_path,
                                    asset_path,
                                    properties,
                                )
                                .await
                                {
                                    tracing::error!(?err);
                                    let mut msg =
                                        types::message::Builder::error("Could not save asset");
                                    msg.body(format!("{err:?}"));
                                    messages.update(|messages| messages.push(msg.build()));
                                }
                            }
                        });

                        // return the current id to track if the asset changed
                        asset.rid().get_untracked()
                    }
                },
                false,
            );

            let remove_datum = {
                let project = project.clone();
                let graph = graph.clone();
                let asset = asset.clone();
                let messages = messages.clone();
                let key = key.clone();
                move |e: MouseEvent| {
                    if e.button() != types::MouseButton::Primary {
                        return;
                    }

                    let mut properties = asset.as_properties();
                    properties.metadata.retain(|k, _| k != &key);

                    let project = project.rid().get_untracked();
                    let node = asset
                        .rid()
                        .with_untracked(|rid| graph.find_by_asset_id(rid).unwrap());
                    let container_path = graph.path(&node).unwrap();
                    let asset_path = asset.path().get_untracked();
                    let messages = messages.clone();
                    spawn_local(async move {
                        if let Err(err) =
                            update_properties(project, container_path, asset_path, properties).await
                        {
                            tracing::error!(?err);
                            let mut msg =
                                types::message::Builder::error("Could not save container");
                            msg.body(format!("{err:?}"));
                            messages.update(|messages| messages.push(msg.build()));
                        }
                    });
                }
            };

            view! {
                <div class="pb-2">
                    <div class="flex">
                        <span class="grow">{key}</span>
                        <button
                            type="button"
                            on:mousedown=remove_datum
                            class="aspect-square h-full rounded-xs hover:bg-secondary-200 dark:hover:bg-secondary-700"
                        >
                            <Icon icon=components::icon::Remove />
                        </button>
                    </div>
                    <ValueEditor value=input_value oninput debounce=*input_debounce />
                </div>
            }
        }
    }

    async fn update_properties(
        project: ResourceId,
        container: impl Into<PathBuf>,
        asset: impl Into<PathBuf>,
        properties: syre_core::project::AssetProperties,
    ) -> Result<(), ()> {
        #[derive(Serialize)]
        struct Args {
            project: ResourceId,
            container: PathBuf,
            asset: PathBuf,
            // properties: syre_core::project::AssetProperties,
            properties: String, // TODO: Issue with serializing enum with Option. perform manually.
                                // See: https://github.com/tauri-apps/tauri/issues/5993
        }

        tauri_sys::core::invoke_result(
            "asset_properties_update",
            Args {
                project,
                container: container.into(),
                asset: asset.into(),
                properties: serde_json::to_string(&properties).unwrap(),
            },
        )
        .await
    }
}

mod flags {
    use crate::{commands, components, pages::project::state, types};
    use leptos::{either::Either, ev::MouseEvent, prelude::*};
    use leptos_icons::Icon;
    use std::path::PathBuf;
    use syre_local as local;

    #[component]
    pub fn Editor(asset: state::Asset) -> impl IntoView {
        let graph = expect_context::<state::Graph>();
        let flags_state = expect_context::<state::Flags>();
        let container = {
            let graph = graph.clone();
            let asset = asset.rid().read_only();
            move || graph.find_by_asset_id(&*asset.read()).unwrap()
        };
        let container_path = {
            let graph = graph.clone();
            move || graph.path(&container()).unwrap()
        };
        let asset_path = {
            let asset = asset.path().read_only();
            let container_path = container_path.clone();
            move || container_path().join(&*asset.read())
        };
        let flags = move || flags_state.find(&asset_path());

        view! {
            <div>
                <super::Header />
                {move || {
                    if flags().read().as_ref().map(|flags| flags.read().is_empty()).unwrap_or(true)
                    {
                        Either::Left(view! { <div class="text-lg text-center">"(no flags)"</div> })
                    } else {
                        Either::Right(
                            view! {
                                <Flags
                                    container=container_path()
                                    asset=asset.path().get()
                                    flags=flags().read().as_ref().unwrap().read_only()
                                />
                            },
                        )
                    }
                }}
            </div>
        }
    }

    #[component]
    fn Flags(
        /// Graph path to the container.
        container: PathBuf,
        asset: PathBuf,
        flags: ArcReadSignal<Vec<local::project::resources::Flag>>,
    ) -> impl IntoView {
        let project = expect_context::<state::Project>();
        let messages = expect_context::<types::Messages>();

        let remove_all_action = Action::new_local({
            let project = project.path().read_only();
            let container = container.clone();
            let asset = asset.clone();
            move |_| {
                let container = container.clone();
                let asset = asset.clone();
                async move {
                    if let Err(err) =
                        commands::flag::remove_all(project.get_untracked(), container, asset).await
                    {
                        let mut msg = types::message::Builder::error("Could not remove flag.");
                        msg.body(format!("{err:?}"));
                        messages.write().push(msg.build());
                    }
                }
            }
        });

        let trigger_remove_all = move |e: MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }
            e.stop_propagation();

            remove_all_action.dispatch(());
        };

        view! {
            <div>
                <div class="text-center pb-2">
                    <button
                        on:mousedown=trigger_remove_all
                        class="px-4 bg-secondary-50 dark:bg-secondary-600 rounded-sm border"
                        disabled=remove_all_action.pending()
                    >
                        "Remove all"
                    </button>
                </div>
                <ul>
                    <For each=flags key=move |flag| flag.id().clone() let:flag>
                        <li class="px-2">
                            <Flag
                                container=container.clone()
                                asset=asset.clone()
                                flag=flag.clone()
                            />
                        </li>
                    </For>
                </ul>
            </div>
        }
    }

    #[component]
    fn Flag(
        container: PathBuf,
        asset: PathBuf,
        flag: local::project::resources::Flag,
    ) -> impl IntoView {
        let project = expect_context::<state::Project>();
        let messages = expect_context::<types::Messages>();

        let remove_flag_action = Action::new_local({
            let project = project.path();
            let flag = flag.id().clone();
            let container = container.clone();
            move |_| {
                let flag = flag.clone();
                let container = container.clone();
                let asset = asset.clone();
                async move {
                    if let Err(err) =
                        commands::flag::remove(project.get_untracked(), container, asset, flag)
                            .await
                    {
                        let mut msg = types::message::Builder::error("Could not remove flag.");
                        msg.body(format!("{err:?}"));
                        messages.write().push(msg.build());
                    }
                }
            }
        });

        let trigger_remove_flag = move |e: MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }
            e.stop_propagation();

            remove_flag_action.dispatch(());
        };

        view! {
            <div class="flex">
                <div class="grow">
                    <div>{flag.message().clone()}</div>
                </div>
                <div>
                    <button on:mousedown=trigger_remove_flag disabled=remove_flag_action.pending()>
                        <Icon icon=components::icon::Remove />
                    </button>
                </div>
            </div>
        }
    }
}
