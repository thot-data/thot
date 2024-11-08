use super::{errors_to_list_view, InputDebounce, PopoutPortal};
use crate::{
    components,
    pages::project::{self, state::workspace_graph},
    types,
};
use description::Editor as Description;
use kind::Editor as Kind;
use leptos::{
    ev::{Event, MouseEvent},
    *,
};
use leptos_icons::Icon;
use metadata::{AddDatum, Editor as Metadata};
use serde::Serialize;
use state::{ActiveResources, State};
use std::path::PathBuf;
use syre_core::types::ResourceId;
use syre_desktop_lib as lib;
use tags::{AddTags, Editor as Tags};

#[derive(Clone, Copy)]
enum Widget {
    AddTags,
    AddMetadatum,
}

mod state {
    use super::super::common::bulk;
    use crate::pages::project::state::{self, workspace_graph};
    use leptos::*;
    use std::collections::HashMap;
    use syre_local_database as db;

    #[derive(Clone, Debug)]
    pub struct State {
        kinds: Vec<ReadSignal<Option<String>>>,
        descriptions: Vec<ReadSignal<Option<String>>>,
        tags: Vec<ReadSignal<Vec<String>>>,
        metadata: Vec<ReadSignal<state::Metadata>>,
    }

    impl State {
        pub fn from_states(containers: Vec<state::graph::Node>, assets: Vec<state::Asset>) -> Self {
            let mut kinds = assets
                .iter()
                .map(|state| state.kind().read_only())
                .collect::<Vec<_>>();
            let mut descriptions = assets
                .iter()
                .map(|state| state.description().read_only())
                .collect::<Vec<_>>();
            let mut tags = assets
                .iter()
                .map(|state| state.tags().read_only())
                .collect::<Vec<_>>();
            let mut metadata = assets
                .iter()
                .map(|state| state.metadata().read_only())
                .collect::<Vec<_>>();

            containers
                .iter()
                .map(|state| {
                    state.properties().with_untracked(|properties| {
                        let db::state::DataResource::Ok(properties) = properties else {
                            panic!("invalid state");
                        };

                        (
                            properties.kind().read_only(),
                            properties.description().read_only(),
                            properties.tags().read_only(),
                            properties.metadata().read_only(),
                        )
                    })
                })
                .fold((), |(), (kind, description, tag, metadatum)| {
                    kinds.push(kind);
                    descriptions.push(description);
                    tags.push(tag);
                    metadata.push(metadatum);
                });

            Self {
                kinds,
                descriptions,
                tags,
                metadata,
            }
        }
    }

    impl State {
        pub fn kind(&self) -> Signal<bulk::Value<Option<String>>> {
            Signal::derive({
                let kinds = self.kinds.clone();
                move || {
                    let mut values = kinds
                        .iter()
                        .map(|kind| kind.get_untracked())
                        .collect::<Vec<_>>();
                    values.sort();
                    values.dedup();

                    match &values[..] {
                        [value] => bulk::Value::Equal(value.clone()),
                        _ => bulk::Value::Mixed,
                    }
                }
            })
        }

        pub fn description(&self) -> Signal<bulk::Value<Option<String>>> {
            Signal::derive({
                let descriptions = self.descriptions.clone();
                move || {
                    let mut values = descriptions
                        .iter()
                        .map(|description| description.get())
                        .collect::<Vec<_>>();
                    values.sort();
                    values.dedup();

                    match &values[..] {
                        [value] => bulk::Value::Equal(value.clone()),
                        _ => bulk::Value::Mixed,
                    }
                }
            })
        }

        /// Intersection of all tags.
        pub fn tags(&self) -> Signal<Vec<String>> {
            Signal::derive({
                let tags = self.tags.clone();
                move || {
                    tags.iter()
                        .map(|tags| tags.get())
                        .reduce(|intersection, tags| {
                            let mut intersection = intersection.clone();
                            intersection.retain(|current| tags.contains(current));
                            intersection
                        })
                        .unwrap()
                }
            })
        }

        /// Intersection of all metadata.
        pub fn metadata(&self) -> Signal<bulk::Metadata> {
            Signal::derive({
                let states = self.metadata.clone();
                move || {
                    let mut metadata = HashMap::new();
                    states.iter().for_each(|state| {
                        state.with(|data| {
                            data.iter().for_each(|(key, value)| {
                                let entry = metadata.entry(key.clone()).or_insert(vec![]);
                                entry.push(value.read_only());
                            });
                        });
                    });

                    metadata
                        .into_iter()
                        .filter_map(|(key, values)| {
                            if values.len() != states.len() {
                                return None;
                            }

                            Some(bulk::Metadatum::new(key, values))
                        })
                        .collect()
                }
            })
        }
    }

    #[derive(derive_more::Deref, derive_more::From, Clone)]
    pub struct ActiveResources(ReadSignal<Vec<workspace_graph::Resource>>);
}

#[component]
pub fn Editor(resources: ReadSignal<Vec<workspace_graph::Resource>>) -> impl IntoView {
    assert!(resources.with_untracked(|resources| resources.len()) > 1);
    let graph = expect_context::<project::state::Graph>();
    let popout_portal = expect_context::<PopoutPortal>();
    let (widget, set_widget) = create_signal(None);
    let wrapper_node = NodeRef::<html::Div>::new();
    let tags_node = NodeRef::<html::Div>::new();
    let metadata_node = NodeRef::<html::Div>::new();

    provide_context(Signal::derive(move || {
        let (containers, assets) = resources.with(|resources| {
            let (containers, assets) = partition_resources(resources);
            assert!(containers.len() > 0);
            assert!(assets.len() > 0);

            let containers = containers
                .iter()
                .map(|&resource| {
                    resource
                        .rid()
                        .with_untracked(|rid| graph.find_by_id(rid).unwrap())
                })
                .collect::<Vec<_>>();

            let assets = assets
                .iter()
                .map(|resource| {
                    resource
                        .rid()
                        .with_untracked(|rid| graph.find_asset_by_id(rid).unwrap())
                })
                .collect::<Vec<_>>();

            (containers, assets)
        });

        State::from_states(containers, assets)
    }));

    provide_context::<ActiveResources>(resources.into());

    let resource_lengths = move || {
        resources.with(|resources| {
            let (containers, assets) = partition_resources(resources);
            (containers.len(), assets.len())
        })
    };

    let show_add_tags = move |e: MouseEvent| {
        if e.button() == types::MouseButton::Primary {
            let wrapper = wrapper_node.get_untracked().unwrap();
            let base = tags_node.get_untracked().unwrap();
            let portal = popout_portal.get_untracked().unwrap();

            let top = super::detail_popout_top(&portal, &base, &wrapper);
            (*portal)
                .style()
                .set_property("top", &format!("{top}px"))
                .unwrap();

            set_widget.update(|widget| {
                #[allow(unused_must_use)]
                {
                    widget.insert(Widget::AddTags);
                }
            });
        }
    };

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

            set_widget.update(|widget| {
                #[allow(unused_must_use)]
                {
                    widget.insert(Widget::AddMetadatum);
                }
            });
        }
    };

    let scroll = move |_: Event| {
        let wrapper = wrapper_node.get_untracked().unwrap();
        let portal = popout_portal.get_untracked().unwrap();
        let Some(base) = widget.with(|widget| {
            widget.map(|widget| match widget {
                Widget::AddTags => tags_node,
                Widget::AddMetadatum => metadata_node,
            })
        }) else {
            return;
        };
        let base = base.get_untracked().unwrap();

        let top = super::detail_popout_top(&portal, &base, &wrapper);
        (*portal)
            .style()
            .set_property("top", &format!("{top}px"))
            .unwrap();
    };

    let on_widget_close = move |_| {
        set_widget.update(|widget| {
            widget.take();
        });
    };

    view! {
        <div ref=wrapper_node on:scroll=scroll class="overflow-y-auto pr-2 h-full scrollbar-thin">
            <div class="text-center pt-1 pb-2">
                <h3 class="font-primary">"Bulk resources"</h3>
                <span class="text-sm text-secondary-500 dark:text-secondary-400">
                    "Editing "
                    {move || {
                        let (containers, assets) = resource_lengths();
                        let containers = if containers == 1 {
                            format!("1 container")
                        } else {
                            format!("{containers} containers")
                        };
                        let assets = if assets == 1 {
                            format!("1 asset")
                        } else {
                            format!("{assets} assets")
                        };
                        view! {
                            <span>{containers}</span>
                            ", "
                            <span>{assets}</span>
                        }
                    }}
                </span>
            </div>
            <form on:submit=move |e| e.prevent_default()>
                <div class="px-1 pb-1">
                    <label>
                        <span class="block">"Type"</span>
                        <Kind />
                    </label>
                </div>
                <div class="px-1 pb-1">
                    <label>
                        <span class="block">"Description"</span>
                        <Description />
                    </label>
                </div>
                <div
                    ref=tags_node
                    class="relative py-4 border-t border-t-secondary-200 dark:border-t-secondary-700"
                >
                    <label class="block px-1">
                        <div class="flex">
                            <span class="grow">"Tags"</span>
                            <span>
                                // TODO: Button hover state seems to be triggered by hovering over
                                // parent section.
                                <button
                                    on:mousedown=show_add_tags
                                    class=(
                                        ["bg-primary-400", "dark:bg-primary-700"],
                                        move || {
                                            widget
                                                .with(|widget| {
                                                    widget
                                                        .map_or(false, |widget| matches!(widget, Widget::AddTags))
                                                })
                                        },
                                    )

                                    class=(
                                        ["hover:bg-secondary-200", "dark:hover:bg-secondary-700"],
                                        move || {
                                            widget
                                                .with(|widget| {
                                                    widget
                                                        .map_or(false, |widget| !matches!(widget, Widget::AddTags))
                                                })
                                        },
                                    )

                                    class="aspect-square w-full rounded-sm"
                                >
                                    <Icon icon=components::icon::Add />
                                </button>
                            </span>
                        </div>
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
                                <button
                                    on:mousedown=show_add_metadatum
                                    class=(
                                        ["bg-primary-400", "dark:bg-primary-700"],
                                        move || {
                                            widget
                                                .with(|widget| {
                                                    widget
                                                        .map_or(
                                                            false,
                                                            |widget| matches!(widget, Widget::AddMetadatum),
                                                        )
                                                })
                                        },
                                    )

                                    class=(
                                        ["hover:bg-secondary-200", "dark:hover:bg-secondary-700"],
                                        move || {
                                            widget
                                                .with(|widget| {
                                                    widget
                                                        .map_or(
                                                            false,
                                                            |widget| !matches!(widget, Widget::AddMetadatum),
                                                        )
                                                })
                                        },
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
            <Show
                when=move || widget.with(|widget| widget.is_some()) && popout_portal.get().is_some()
                fallback=|| view! {}
            >
                {move || {
                    let mount = popout_portal.get_untracked().unwrap();
                    let mount = (*mount).clone();
                    view! {
                        <Portal mount>
                            {move || match widget().unwrap() {
                                Widget::AddTags => {
                                    view! { <AddTags onclose=on_widget_close.clone() /> }
                                }
                                Widget::AddMetadatum => {
                                    view! { <AddDatum onclose=on_widget_close.clone() /> }
                                }
                            }}
                        </Portal>
                    }
                }}
            </Show>
        </div>
    }
}

mod kind {
    use super::{
        super::common::bulk::kind::Editor as KindEditor, update_properties, ActiveResources,
        InputDebounce, State,
    };
    use crate::{pages::project::state, types::Messages};
    use leptos::*;
    use syre_desktop_lib::command::asset::bulk::PropertiesUpdate;

    #[component]
    pub fn Editor() -> impl IntoView {
        let project = expect_context::<state::Project>();
        let graph = expect_context::<state::Graph>();
        let messages = expect_context::<Messages>();
        let resources = expect_context::<ActiveResources>();
        let state = expect_context::<Signal<State>>();
        let input_debounce = expect_context::<InputDebounce>();

        let oninput = Callback::new(move |input_value: Option<String>| {
            let mut update = PropertiesUpdate::default();
            let _ = update.kind.insert(input_value.clone());
            spawn_local({
                let project = project.rid().get_untracked();
                let resources = resources.clone();
                let graph = graph.clone();
                let messages = messages.clone();
                async move {
                    update_properties(project, resources, update, &graph, messages).await;
                }
            });
        });

        view! { <KindEditor value=state.with(|state| { state.kind() }) oninput debounce=*input_debounce /> }
    }
}

mod description {
    use super::{
        super::common::bulk::description::Editor as DescriptionEditor, update_properties,
        ActiveResources, InputDebounce, State,
    };
    use crate::{pages::project::state, types::Messages};
    use leptos::*;
    use syre_desktop_lib::command::asset::bulk::PropertiesUpdate;

    #[component]
    pub fn Editor() -> impl IntoView {
        let project = expect_context::<state::Project>();
        let graph = expect_context::<state::Graph>();
        let messages = expect_context::<Messages>();
        let resources = expect_context::<ActiveResources>();
        let state = expect_context::<Signal<State>>();
        let input_debounce = expect_context::<InputDebounce>();

        let oninput = Callback::new(move |input_value: Option<String>| {
            let mut update = PropertiesUpdate::default();
            let _ = update.description.insert(input_value.clone());
            spawn_local({
                let project = project.rid().get_untracked();
                let resources = resources.clone();
                let graph = graph.clone();
                let messages = messages.clone();
                async move { update_properties(project, resources, update, &graph, messages).await }
            });
        });

        view! {
            <DescriptionEditor
                value=state.with(|state| { state.description() })
                oninput
                debounce=*input_debounce
                class="input-compact w-full align-top"
            />
        }
    }
}

mod tags {
    use super::{
        super::common::bulk::tags::{AddTags as AddTagsEditor, Editor as TagsEditor},
        update_properties, ActiveResources, State,
    };
    use crate::{components::DetailPopout, pages::project::state, types::Messages};
    use leptos::*;
    use syre_desktop_lib::command::{asset::bulk::PropertiesUpdate, bulk::TagsAction};

    #[component]
    pub fn Editor() -> impl IntoView {
        let project = expect_context::<state::Project>();
        let graph = expect_context::<state::Graph>();
        let messages = expect_context::<Messages>();
        let resources = expect_context::<ActiveResources>();
        let state = expect_context::<Signal<State>>();
        let onremove = Callback::new({
            let graph = graph.clone();
            let project = project.clone();
            let resources = resources.clone();
            move |value: String| {
                if value.is_empty() {
                    return;
                };

                let mut update = PropertiesUpdate::default();
                update.tags = TagsAction {
                    insert: vec![],
                    remove: vec![value.clone()],
                };
                spawn_local({
                    let project = project.rid().get_untracked();
                    let resources = resources.clone();
                    let graph = graph.clone();
                    let messages = messages.clone();
                    async move { update_properties(project, resources, update, &graph, messages).await }
                });
            }
        });

        view! { <TagsEditor value=state.with(|state| { state.tags() }) onremove /> }
    }

    #[component]
    pub fn AddTags(#[prop(optional, into)] onclose: Option<Callback<()>>) -> impl IntoView {
        let project = expect_context::<state::Project>();
        let graph = expect_context::<state::Graph>();
        let messages = expect_context::<Messages>();
        let resources = expect_context::<ActiveResources>();
        let onadd = Callback::new(move |tags: Vec<String>| {
            if tags.is_empty() {
                return;
            };

            let mut update = PropertiesUpdate::default();
            update.tags = TagsAction {
                insert: tags.clone(),
                remove: vec![],
            };
            spawn_local({
                let project = project.rid().get_untracked();
                let resources = resources.clone();
                let graph = graph.clone();
                let messages = messages.clone();
                async move {
                    update_properties(project, resources, update, &graph, messages).await;
                    if let Some(onclose) = onclose {
                        onclose(());
                    }
                }
            });
        });

        let close = move |_| {
            if let Some(onclose) = onclose {
                onclose(());
            }
        };

        view! {
            <DetailPopout title="Add tags" onclose=Callback::new(close)>
                <AddTagsEditor onadd=Callback::new(onadd) class="w-full px-1" />
            </DetailPopout>
        }
    }
}

mod metadata {
    use super::{
        super::common::{
            bulk::metadata::Editor as MetadataEditor, metadata::AddDatum as AddDatumEditor,
        },
        update_properties, ActiveResources, InputDebounce, State,
    };
    use crate::{components::DetailPopout, pages::project::state, types::Messages};
    use leptos::*;
    use syre_core::types::data;
    use syre_desktop_lib::command::{asset::bulk::PropertiesUpdate, bulk::MetadataAction};

    #[component]
    pub fn Editor() -> impl IntoView {
        let project = expect_context::<state::Project>();
        let graph = expect_context::<state::Graph>();
        let messages = expect_context::<Messages>();
        let resources = expect_context::<ActiveResources>();
        let state = expect_context::<Signal<State>>();
        let input_debounce = expect_context::<InputDebounce>();
        let (modifications, set_modifications) = create_signal(vec![]);
        let modifications = leptos_use::signal_debounced(modifications, *input_debounce);

        let onremove = Callback::new({
            let project = project.clone();
            let graph = graph.clone();
            let resources = resources.clone();
            let messages = messages.clone();
            move |value: String| {
                let mut update = PropertiesUpdate::default();
                update.metadata = MetadataAction {
                    add: vec![],
                    update: vec![],
                    remove: vec![value.clone()],
                };

                spawn_local({
                    let project = project.rid().get_untracked();
                    let resources = resources.clone();
                    let graph = graph.clone();
                    let messages = messages.clone();
                    async move { update_properties(project, resources, update, &graph, messages).await }
                });
            }
        });

        let onmodify = Callback::new(move |value: (String, data::Value)| {
            set_modifications.update(|modifications| modifications.push(value));
        });

        let _ = watch(
            modifications,
            {
                let project = project.clone();
                let graph = graph.clone();
                let resources = resources.clone();
                let messages = messages.clone();
                move |modifications, _, _| {
                    let mut update = PropertiesUpdate::default();
                    update.metadata = MetadataAction {
                        add: vec![],
                        update: modifications.clone(),
                        remove: vec![],
                    };
                    set_modifications.update_untracked(|modifications| modifications.clear());

                    spawn_local({
                        let project = project.rid().get_untracked();
                        let resources = resources.clone();
                        let graph = graph.clone();
                        let messages = messages.clone();
                        async move {
                            update_properties(project, resources, update, &graph, messages).await
                        }
                    });
                }
            },
            false,
        );

        view! { <MetadataEditor value=state.with(|state| { state.metadata() }) onremove onmodify /> }
    }

    #[component]
    pub fn AddDatum(#[prop(optional, into)] onclose: Option<Callback<()>>) -> impl IntoView {
        let project = expect_context::<state::Project>();
        let graph = expect_context::<state::Graph>();
        let messages = expect_context::<Messages>();
        let resources = expect_context::<ActiveResources>();
        let state = expect_context::<Signal<State>>();
        let onadd = Callback::new({
            let project = project.clone();
            let graph = graph.clone();
            let resources = resources.clone();
            move |value: (String, data::Value)| {
                let mut update = PropertiesUpdate::default();
                update.metadata = MetadataAction {
                    add: vec![value.clone()],
                    update: vec![],
                    remove: vec![],
                };

                spawn_local({
                    let project = project.rid().get_untracked();
                    let resources = resources.clone();
                    let graph = graph.clone();
                    let messages = messages.clone();
                    async move {
                        update_properties(project, resources, update, &graph, messages).await;
                        if let Some(onclose) = onclose {
                            onclose(());
                        }
                    }
                });
            }
        });

        let keys = move || {
            state.with(|state| {
                state.metadata().with(|metadata| {
                    metadata
                        .iter()
                        .map(|datum| datum.key().clone())
                        .collect::<Vec<_>>()
                })
            })
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
}

// TODO: Move error handling to individual widgets as in other properties components?
async fn update_properties(
    project: ResourceId,
    resources: ActiveResources,
    update: lib::command::asset::bulk::PropertiesUpdate,
    graph: &project::state::Graph,
    messages: types::Messages,
) {
    let (containers, asset_ids) =
        resources.with_untracked(|resources| resources_to_update_args(resources, &graph));
    let expected_results_containers = containers.len();
    let expected_results_assets = asset_ids.len();

    match update_properties_invoke(project, containers, asset_ids, update).await {
        Err(err) => {
            let mut msg = types::message::Builder::error("Could not save properties.");
            msg.body(format!("{err:?}"));
            messages.update(|messages| messages.push(msg.build()));
        }

        Ok((container_results, asset_results)) => {
            assert_eq!(container_results.len(), expected_results_containers);
            assert_eq!(asset_results.len(), expected_results_assets);

            let container_errors = container_results
                .into_iter()
                .filter_map(|err| err.err())
                .collect::<Vec<_>>();

            let asset_errors = asset_results
                .into_iter()
                .filter_map(|err| err.err())
                .collect::<Vec<_>>();

            if !container_errors.is_empty() || !asset_errors.is_empty() {
                let mut msg = types::message::Builder::error("Could not save properties.");

                let errors = view! {
                    <div>
                        {if !container_errors.is_empty() {
                            view! {
                                <div>
                                    <h2>"Containers"</h2>
                                    {errors_to_list_view(container_errors)}
                                </div>
                            }
                                .into_view()
                        } else {
                            view! {}.into_view()
                        }}
                        {if !asset_errors.is_empty() {
                            view! {
                                <div>
                                    <h2>"Assets"</h2>
                                    {errors_to_list_view(asset_errors)}
                                </div>
                            }
                                .into_view()
                        } else {
                            view! {}.into_view()
                        }}
                    </div>
                };
                msg.body(errors);
                messages.update(|messages| messages.push(msg.build()));
            }
        }
    }
}

/// # Returns
/// Results each resources update as (containers, assets).
async fn update_properties_invoke(
    project: ResourceId,
    containers: Vec<PathBuf>,
    assets: Vec<lib::command::asset::bulk::ContainerAssets>,
    update: lib::command::asset::bulk::PropertiesUpdate,
) -> Result<
    (
        Vec<Result<(), lib::command::container::bulk::error::Update>>,
        Vec<Result<(), lib::command::asset::bulk::error::Update>>,
    ),
    lib::command::error::ProjectNotFound,
> {
    #[derive(Serialize)]
    struct Args {
        project: ResourceId,
        containers: Vec<PathBuf>,
        assets: Vec<lib::command::asset::bulk::ContainerAssets>,
        // update: lib::command::asset::bulk::PropertiesUpdate,
        update: String, // TODO: Issue with serializing enum with Option. perform manually.
                        // See: https://github.com/tauri-apps/tauri/issues/5993
    }

    tauri_sys::core::invoke_result(
        "properties_update_bulk_mixed",
        Args {
            project,
            containers,
            assets,
            update: serde_json::to_string(&update).unwrap(),
        },
    )
    .await
}

/// Partition resources into (containers, assets).
fn partition_resources<'a>(
    resources: &'a Vec<workspace_graph::Resource>,
) -> (
    Vec<&'a workspace_graph::Resource>,
    Vec<&'a workspace_graph::Resource>,
) {
    resources
        .iter()
        .partition(|resource| match resource.kind() {
            workspace_graph::ResourceKind::Container => true,
            workspace_graph::ResourceKind::Asset => false,
        })
}

/// Transforms a list of asset [`ResourceId`]s into
/// [`ContainerAssets`](lib::command::asset::bulk::ContainerAssets).
fn container_assets(
    assets: &Vec<ResourceId>,
    graph: &project::state::Graph,
) -> Vec<lib::command::asset::bulk::ContainerAssets> {
    let mut asset_ids = Vec::<(PathBuf, Vec<ResourceId>)>::new();
    for asset in assets {
        let node = graph.find_by_asset_id(asset).unwrap();
        let container = graph.path(&node).unwrap();
        if let Some(ref mut container_assets) = asset_ids
            .iter_mut()
            .find_map(|(container_id, assets)| (*container_id == container).then_some(assets))
        {
            container_assets.push(asset.clone());
        } else {
            asset_ids.push((container, vec![asset.clone()]));
        }
    }

    asset_ids.into_iter().map(|ids| ids.into()).collect()
}

fn resources_to_update_args(
    resources: &Vec<workspace_graph::Resource>,
    graph: &project::state::Graph,
) -> (
    Vec<PathBuf>,
    Vec<lib::command::asset::bulk::ContainerAssets>,
) {
    let (containers, assets) = partition_resources(resources);
    let containers = containers
        .iter()
        .map(|container| {
            let node = container
                .rid()
                .with_untracked(|rid| graph.find_by_id(rid).unwrap());
            graph.path(&node).unwrap()
        })
        .collect();

    let asset_ids = assets
        .iter()
        .map(|resource| resource.rid().get_untracked())
        .collect();
    let asset_ids = container_assets(&asset_ids, &graph);
    (containers, asset_ids)
}
