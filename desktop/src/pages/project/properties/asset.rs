use super::{InputDebounce, PopoutPortal};
use crate::{components, pages::project::state, types};
use description::Editor as Description;
use kind::Editor as Kind;
use leptos::{
    ev::{Event, MouseEvent},
    *,
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
    let add_metadatum_visible = create_rw_signal(false);
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
        <div ref=wrapper_node on:scroll=scroll class="overflow-y-auto pr-2 h-full scrollbar-thin">
            <div class="text-center pt-1 pb-2">
                <h3 class="font-primary">"Asset"</h3>
            </div>
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
                    ref=metadata_node
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

                                    class="aspect-square w-full rounded-sm"
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
                            <AddDatum onclose=move |_| add_metadatum_visible.set(false) />
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
    use leptos::*;

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
                        let mut msg = types::message::Builder::error("Could not save container");
                        msg.body(format!("{err:?}"));
                        messages.update(|messages| messages.push(msg.build()));
                    }
                });
            }
        });

        view! { <InputText value=input_value oninput debounce=*input_debounce class="input-compact" /> }
    }
}

mod kind {
    use super::{
        super::common::kind::Editor as KindEditor, update_properties, ActiveAsset, InputDebounce,
    };
    use crate::{pages::project::state, types};
    use leptos::*;

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
                        let mut msg = types::message::Builder::error("Could not save container");
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
        super::common::description::Editor as DescriptionEditor, update_properties, ActiveAsset,
        InputDebounce,
    };
    use crate::{pages::project::state, types};
    use leptos::*;

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
                        let mut msg = types::message::Builder::error("Could not save container");
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
        super::common::tags::Editor as TagsEditor, update_properties, ActiveAsset, InputDebounce,
    };
    use crate::{pages::project::state, types};
    use leptos::*;

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
                        let mut msg = types::message::Builder::error("Could not save container");
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
        super::common::metadata::{AddDatum as AddDatumEditor, ValueEditor},
        update_properties, ActiveAsset, InputDebounce,
    };
    use crate::{
        components::{self, DetailPopout},
        pages::project::state,
        types,
    };
    use leptos::{ev::MouseEvent, *};
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

        let onadd = {
            let asset = asset.clone();
            move |(key, value): (String, Value)| {
                assert!(!key.is_empty());
                assert!(!asset
                    .metadata()
                    .with(|metadata| metadata.iter().any(|(k, _)| *k == key)));

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
                            onclose(());
                        }
                    }
                });
            }
        };

        let close = move |_| {
            if let Some(onclose) = onclose {
                onclose(());
            }
        };

        view! {
            <DetailPopout title="Add metadata" onclose=Callback::new(close)>
                <AddDatumEditor
                    keys=Signal::derive(keys)
                    onadd=Callback::new(onadd)
                    class="w-full px-1"
                />
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

        let (input_value, set_input_value) = create_signal(value.get_untracked());
        let input_value = leptos_use::signal_debounced(input_value, *input_debounce);
        let _ = watch(
            input_value,
            move |value, _, _| {
                set_input_value(value.clone());
            },
            false,
        );

        // TODO: Handle errors with messages.
        // See https://github.com/leptos-rs/leptos/issues/2041
        let _ = watch(
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
                            if let Err(err) =
                                update_properties(project, container_path, asset_path, properties)
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

        let remove_datum = Callback::new({
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
                        let mut msg = types::message::Builder::error("Could not save container");
                        msg.body(format!("{err:?}"));
                        messages.update(|messages| messages.push(msg.build()));
                    }
                });
            }
        });

        view! {
            <div class="pb-2">
                <div class="flex">
                    <span class="grow">{key}</span>
                    <button
                        type="button"
                        on:mousedown=remove_datum
                        class="aspect-square h-full rounded-sm hover:bg-secondary-200 dark:hover:bg-secondary-700"
                    >
                        <Icon icon=components::icon::Remove />
                    </button>
                </div>
                <ValueEditor value=input_value set_value=set_input_value />
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
