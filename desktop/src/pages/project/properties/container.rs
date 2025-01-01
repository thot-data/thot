use super::detail_popout_top;
use crate::{components, pages::project::state, types};
use leptos::{either::either, ev::MouseEvent, prelude::*};
use leptos_icons::Icon;

#[derive(Clone, Copy, PartialEq)]
enum EditorView {
    Properties,
    Flags,
}

#[component]
pub fn Editor(container: state::graph::Node) -> impl IntoView {
    let editor_view = RwSignal::new(EditorView::Properties);
    provide_context(editor_view);

    {
        let container = container.clone();
        move || {
            either!(
                editor_view(),
                EditorView::Properties => view! { <properties::Editor container=(*container).clone() /> },
                EditorView::Flags => view! { <flags::Editor container=container.clone() /> },
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
                <h3 class="font-primary">"Container"</h3>
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
    use super::super::{common, InputDebounce, PopoutPortal};
    use crate::{components, pages::project::state, types};
    use analysis_associations::{AddAssociation, Editor as AnalysisAssociations};
    use description::Editor as Description;
    use has_id::HasId;
    use kind::Editor as Kind;
    use leptos::{
        either::either,
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
    use syre_desktop_lib as lib;
    use syre_local as local;
    use syre_local_database as db;
    use tags::Editor as Tags;

    #[derive(Clone, Copy)]
    enum Widget {
        AddMetadatum,
        AddAnalysisAssociation,
    }

    // TODO: Use enum for popout detail widget.
    #[component]
    pub fn Editor(container: state::Container) -> impl IntoView {
        let project = expect_context::<state::Project>();
        let popout_portal = expect_context::<PopoutPortal>();
        let (widget, set_widget) = signal(None);
        let wrapper_node = NodeRef::<html::Div>::new();
        let metadata_node = NodeRef::<html::Div>::new();
        let analyses_node = NodeRef::<html::Div>::new();

        let db::state::DataResource::Ok(properties) = container.properties().get_untracked() else {
            panic!("invalid state");
        };

        let db::state::DataResource::Ok(analysis_associations) =
            container.analyses().get_untracked()
        else {
            panic!("invalid state");
        };

        let available_analyses =
            move || {
                let db::state::DataResource::Ok(analyses) = project.analyses().get() else {
                    return vec![];
                };

                analyses.with(|analyses| {
                    analyses
                        .iter()
                        .filter_map(move |analysis| {
                            if analysis_associations.with(|associations| {
                                !associations.iter().any(|association| {
                                    analysis.properties().with(|properties| {
                                        association.analysis() == properties.id()
                                    })
                                })
                            }) {
                                analysis.properties().with(|properties| match properties {
                            local::types::AnalysisKind::Script(script) => {
                                let title = if let Some(name) = script.name.as_ref() {
                                    name.clone()
                                } else {
                                    script.path.to_string_lossy().to_string()
                                };

                                Some(common::analysis_associations::AnalysisInfo::script(
                                    script.rid().clone(),
                                    title,
                                ))
                            }

                            local::types::AnalysisKind::ExcelTemplate(template) => {
                                let title = if let Some(name) = template.name.as_ref() {
                                    name.clone()
                                } else {
                                    template.template.path.to_string_lossy().to_string()
                                };

                                Some(common::analysis_associations::AnalysisInfo::excel_template(
                                    template.rid().clone(),
                                    title,
                                ))
                            }
                        })
                            } else {
                                None
                            }
                        })
                        .collect()
                })
            };
        let available_analyses = Signal::derive(available_analyses);

        let show_add_metadatum = move |e: MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }

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
        };

        let show_add_analysis = move |e: MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }

            let wrapper = wrapper_node.get_untracked().unwrap();
            let base = analyses_node.get_untracked().unwrap();
            let portal = popout_portal.get_untracked().unwrap();

            let top = super::detail_popout_top(&portal, &base, &wrapper);
            (*portal)
                .style()
                .set_property("top", &format!("{top}px"))
                .unwrap();

            set_widget.update(|widget| {
                #[allow(unused_must_use)]
                {
                    widget.insert(Widget::AddAnalysisAssociation);
                }
            });
        };

        let scroll = move |_: Event| {
            let wrapper = wrapper_node.get_untracked().unwrap();
            let portal = popout_portal.get_untracked().unwrap();
            let Some(base) = widget.with_untracked(|widget| {
                widget.map(|widget| match widget {
                    Widget::AddMetadatum => metadata_node,
                    Widget::AddAnalysisAssociation => analyses_node,
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

        let on_widget_close = move || {
            set_widget.update(|widget| {
                widget.take();
            });
        };

        view! {
            <div
                node_ref=wrapper_node
                on:scroll=scroll
                class="overflow-y-auto pr-2 h-full scrollbar-thin"
            >
                <super::Header />
                <form on:submit=|e| e.prevent_default()>
                    <div class="pb-1 px-1">
                        <label>
                            <span class="block">"Name"</span>
                            <Name
                                value=properties.name().read_only()
                                container=properties.rid().read_only()
                            />
                        </label>
                    </div>
                    <div class="pb-1 px-1">
                        <label>
                            <span class="block">"Type"</span>
                            <Kind
                                value=properties.kind().read_only()
                                container=properties.rid().read_only()
                            />
                        </label>
                    </div>
                    <div class="pb-1 px-1">
                        <label>
                            <span class="block">"Description"</span>
                            <Description
                                value=properties.description().read_only()
                                container=properties.rid().read_only()
                            />
                        </label>
                    </div>
                    <div class="pb-4 px-1">
                        <label>
                            <span class="block">"Tags"</span>
                            <Tags
                                value=properties.tags().read_only()
                                container=properties.rid().read_only()
                            />
                        </label>
                    </div>
                    <div class="relative py-4 border-t border-t-secondary-200 dark:border-t-secondary-700">
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
                            <Metadata
                                node_ref=metadata_node
                                value=properties.metadata().read_only()
                                container=properties.rid().read_only()
                            />

                        </label>
                    </div>
                    <div
                        node_ref=analyses_node
                        class="relative pt-4 pb-1 border-t border-t-secondary-200 dark:border-t-secondary-700"
                    >
                        <label class="px-1 block">
                            <div class="flex">
                                <span class="grow">"Analyses"</span>
                                <span>
                                    // TODO: Button hover state seems to be triggered by hovering over
                                    // parent section.
                                    <button
                                        on:mousedown=show_add_analysis
                                        class=(
                                            ["bg-primary-400", "dark:bg-primary-700"],
                                            move || {
                                                widget
                                                    .with(|widget| {
                                                        widget
                                                            .map_or(
                                                                false,
                                                                |widget| {
                                                                    matches!(widget, Widget::AddAnalysisAssociation)
                                                                },
                                                            )
                                                    })
                                            },
                                        )

                                        class=(
                                            ["hover:bg-secondary-200", "dark:hover:bg-secondary-700"],
                                            move || {
                                                available_analyses.with(|analyses| !analyses.is_empty())
                                                    && widget
                                                        .with(|widget| {
                                                            widget
                                                                .map_or(
                                                                    false,
                                                                    |widget| {
                                                                        !matches!(widget, Widget::AddAnalysisAssociation)
                                                                    },
                                                                )
                                                        })
                                            },
                                        )

                                        class="aspect-square w-full rounded-sm disabled:opacity-50"
                                        disabled=move || {
                                            available_analyses.with(|analyses| analyses.is_empty())
                                        }
                                    >

                                        <Icon icon=components::icon::Add />
                                    </button>
                                </span>
                            </div>
                            <AnalysisAssociations
                                associations=analysis_associations.read_only()
                                container=properties.rid().read_only()
                            />
                        </label>
                    </div>
                </form>
                <Show
                    when=move || {
                        widget.with(|widget| widget.is_some()) && popout_portal.get().is_some()
                    }
                    fallback=|| view! {}
                >
                    {
                        let metadata = properties.metadata().read_only();
                        let container = properties.rid().read_only();
                        move || {
                            let mount = popout_portal.get_untracked().unwrap();
                            let mount = (*mount).clone();
                            view! {
                                <Portal mount>
                                    {move || {
                                        let widget = widget().unwrap();
                                        either!(
                                            widget,
                                            Widget::AddMetadatum => view! { <AddDatum metadata container onclose=on_widget_close.clone() /> },
                                            Widget::AddAnalysisAssociation => view! { <AddAssociation available_analyses container onclose=on_widget_close.clone() /> },
                                        )
                                    }}
                                </Portal>
                            }
                        }
                    }
                </Show>
            </div>
        }
    }

    mod name {
        use super::InputDebounce;
        use crate::{components::form::debounced::value, pages::project::state, types};
        use leptos::{prelude::*, task::spawn_local};
        use serde::Serialize;
        use std::{ffi::OsString, path::PathBuf};
        use syre_core::types::ResourceId;
        use syre_desktop_lib as lib;
        use syre_local_database as db;

        #[component]
        pub fn Editor(
            /// Initial value.
            value: ReadSignal<String>,
            container: ReadSignal<ResourceId>,
        ) -> impl IntoView {
            let project = expect_context::<state::Project>();
            let graph = expect_context::<state::Graph>();
            let messages = expect_context::<types::Messages>();
            let input_debounce = expect_context::<InputDebounce>();

            let (input_value, set_input_value) = signal(value::State::clean(value.get_untracked()));
            let input_value: Signal<value::State<String>> =
                leptos_use::signal_debounced(input_value, *input_debounce);
            let (error, set_error) = signal(false);

            let _ = Effect::watch(
                value,
                move |value, _, _| {
                    set_input_value(value::State::clean(value.clone()));
                },
                false,
            );

            let _ = Effect::watch(
                input_value,
                {
                    let project = project.clone();
                    let graph = graph.clone();
                    let container = container.clone();
                    let messages = messages.write_only();
                    move |value, _, _| {
                        if value.is_clean() {
                            return;
                        }

                        set_error(false);
                        let name = value.trim().to_string();
                        if name.is_empty() {
                            set_error(true);
                            return;
                        }

                        spawn_local({
                            let project = project.rid().get_untracked();
                            let node =
                                container.with_untracked(|rid| graph.find_by_id(rid).unwrap());
                            let path = graph.path(&node).unwrap();
                            let messages = messages.clone();

                            async move {
                                if let Err(err) = rename_container(project, path, name).await {
                                    let mut msg =
                                        types::message::Builder::error("Could not save container.");
                                    msg.body(format!("{err:?}"));
                                    messages.update(|messages| messages.push(msg.build()));
                                }
                            }
                        });
                    }
                },
                false,
            );

            view! {
                <input
                    name="name"
                    on:input=move |e| {
                        set_input_value(value::State::dirty(event_target_value(&e)));
                    }
                    prop:value=move || input_value.with(|value| value.value().clone())
                    class=("border-red", error)
                    class="input-compact w-full"
                    minlength="1"
                />
            }
        }

        async fn rename_container(
            project: ResourceId,
            container: impl Into<PathBuf>,
            name: impl Into<OsString>,
        ) -> Result<(), lib::command::container::error::Rename> {
            #[derive(Serialize)]
            struct Args {
                project: ResourceId,
                container: PathBuf,
                #[serde(with = "db::serde_os_string")]
                name: OsString,
            }

            tauri_sys::core::invoke_result(
                "container_rename",
                Args {
                    project,
                    container: container.into(),
                    name: name.into(),
                },
            )
            .await
        }
    }

    mod kind {
        use super::{
            super::super::common::kind::Editor as KindEditor, update_properties, InputDebounce,
        };
        use crate::{pages::project::state, types};
        use leptos::{prelude::*, task::spawn_local};
        use syre_core::types::ResourceId;
        use syre_local_database as db;

        #[component]
        pub fn Editor(
            value: ReadSignal<Option<String>>,
            container: ReadSignal<ResourceId>,
        ) -> impl IntoView {
            let project = expect_context::<state::Project>();
            let graph = expect_context::<state::Graph>();
            let messages = expect_context::<types::Messages>();
            let input_debounce = expect_context::<InputDebounce>();

            let oninput = move |value: Option<String>| {
                let messages = messages.write_only();
                let node = container.with_untracked(|rid| graph.find_by_id(rid).unwrap());
                let mut properties = node.properties().with_untracked(|properties| {
                    let db::state::DataResource::Ok(properties) = properties else {
                        panic!("invalid state");
                    };

                    properties.as_properties()
                });
                properties.kind = value;

                spawn_local({
                    let project = project.rid().get_untracked();
                    let path = graph.path(&node).unwrap();
                    let messages = messages.clone();

                    async move {
                        if let Err(err) = update_properties(project, path, properties).await {
                            tracing::error!(?err);
                            let mut msg =
                                types::message::Builder::error("Could not save container.");
                            msg.body(format!("{err:?}"));
                            messages.update(|messages| messages.push(msg.build()));
                        }
                    }
                });
            };

            view! {
                <KindEditor
                    value
                    oninput=Callback::new(oninput)
                    debounce=*input_debounce
                    class="input-compact w-full"
                />
            }
        }
    }

    mod description {
        use super::{
            super::super::common::description::Editor as DescriptionEditor, update_properties,
            InputDebounce,
        };
        use crate::{pages::project::state, types};
        use leptos::{prelude::*, task::spawn_local};
        use syre_core::types::ResourceId;
        use syre_local_database as db;

        #[component]
        pub fn Editor(
            /// Initial value.
            value: ReadSignal<Option<String>>,
            container: ReadSignal<ResourceId>,
        ) -> impl IntoView {
            let project = expect_context::<state::Project>();
            let graph = expect_context::<state::Graph>();
            let messages = expect_context::<types::Messages>();
            let input_debounce = expect_context::<InputDebounce>();

            let oninput = {
                let messages = messages.write_only();
                move |value: Option<String>| {
                    let node = container.with_untracked(|rid| graph.find_by_id(rid).unwrap());
                    let mut properties = node.properties().with_untracked(|properties| {
                        let db::state::DataResource::Ok(properties) = properties else {
                            panic!("invalid state");
                        };

                        properties.as_properties()
                    });
                    properties.description = value;

                    spawn_local({
                        let project = project.rid().get_untracked();
                        let path = graph.path(&node).unwrap();
                        let messages = messages.clone();

                        async move {
                            if let Err(err) = update_properties(project, path, properties).await {
                                tracing::error!(?err);
                                let mut msg =
                                    types::message::Builder::error("Could not save container.");
                                msg.body(format!("{err:?}"));
                                messages.update(|messages| messages.push(msg.build()));
                            }
                        }
                    });
                }
            };

            view! {
                <DescriptionEditor
                    value
                    oninput=Callback::new(oninput)
                    debounce=*input_debounce
                    class="input-compact w-full align-top"
                />
            }
        }
    }

    mod tags {
        use super::{
            super::super::common::tags::Editor as TagsEditor, update_properties, InputDebounce,
        };
        use crate::{pages::project::state, types};
        use leptos::{prelude::*, task::spawn_local};
        use syre_core::types::ResourceId;
        use syre_local_database as db;

        #[component]
        pub fn Editor(
            /// Initial value.
            value: ReadSignal<Vec<String>>,
            container: ReadSignal<ResourceId>,
        ) -> impl IntoView {
            let project = expect_context::<state::Project>();
            let graph = expect_context::<state::Graph>();
            let messages = expect_context::<types::Messages>();
            let input_debounce = expect_context::<InputDebounce>();

            let oninput = {
                let messages = messages.write_only();
                move |value: Vec<String>| {
                    let node = container.with_untracked(|rid| graph.find_by_id(rid).unwrap());
                    let mut properties = node.properties().with_untracked(|properties| {
                        let db::state::DataResource::Ok(properties) = properties else {
                            panic!("invalid state");
                        };

                        properties.as_properties()
                    });
                    properties.tags = value;

                    spawn_local({
                        let project = project.rid().get_untracked();
                        let path = graph.path(&node).unwrap();
                        let messages = messages.clone();

                        async move {
                            if let Err(err) = update_properties(project, path, properties).await {
                                tracing::error!(?err);
                                let mut msg =
                                    types::message::Builder::error("Could not save container.");
                                msg.body(format!("{err:?}"));
                                messages.update(|messages| messages.push(msg.build()));
                            }
                        }
                    });
                }
            };

            view! {
                <TagsEditor
                    value
                    oninput=Callback::new(oninput)
                    debounce=*input_debounce
                    class="input-compact w-full"
                />
            }
        }
    }

    mod metadata {
        use super::{
            super::super::common::metadata::{AddDatum as AddDatumEditor, ValueEditor},
            update_properties, InputDebounce,
        };
        use crate::{
            components::{self, DetailPopout},
            pages::project::state,
            types,
        };
        use leptos::{ev::MouseEvent, html, prelude::*, task::spawn_local};
        use leptos_icons::Icon;
        use syre_core::types::{ResourceId, Value};
        use syre_local_database as db;

        #[derive(Clone, derive_more::Deref)]
        struct ActiveResource(ReadSignal<ResourceId>);

        #[component]
        pub fn Editor(
            #[prop(optional)] node_ref: NodeRef<html::Div>,
            /// Initial value.
            value: ReadSignal<state::Metadata>,
            container: ReadSignal<ResourceId>,
        ) -> impl IntoView {
            provide_context(ActiveResource(container));
            let value_sorted = move || {
                let mut value = value.get();
                value.sort_by_key(|(key, _)| key.clone());
                value
            };

            view! {
                <div node_ref=node_ref class="pt-0.5">
                    <For each=value_sorted key=|(key, _)| key.clone() let:datum>
                        <DatumEditor key=datum.0.clone() value=datum.1.read_only() />
                    </For>
                </div>
            }
        }

        #[component]
        pub fn AddDatum(
            container: ReadSignal<ResourceId>,
            metadata: ReadSignal<state::Metadata>,
            #[prop(optional, into)] onclose: Option<Callback<()>>,
        ) -> impl IntoView {
            let project = expect_context::<state::Project>();
            let graph = expect_context::<state::Graph>();
            let keys = move || {
                metadata.with(|metadata| {
                    metadata
                        .iter()
                        .map(|(key, _)| key.clone())
                        .collect::<Vec<_>>()
                })
            };

            let onadd = move |(key, value): (String, Value)| {
                assert!(!key.is_empty());
                assert!(
                    !metadata.with_untracked(|metadata| metadata.iter().any(|(k, _)| *k == key))
                );

                let node = container.with_untracked(|rid| graph.find_by_id(rid).unwrap());
                let path = graph.path(&node).unwrap();
                let mut properties = node.properties().with_untracked(|properties| {
                    let db::state::DataResource::Ok(properties) = properties else {
                        panic!("invalid state");
                    };

                    properties.as_properties()
                });

                let mut metadata = metadata.with_untracked(|metadata| metadata.as_properties());
                metadata.insert(key, value);
                properties.metadata = metadata;

                spawn_local({
                    let project = project.rid().get_untracked();
                    async move {
                        if let Err(err) = update_properties(project, path, properties).await {
                            tracing::error!(?err);
                            todo!()
                        } else {
                            if let Some(onclose) = onclose {
                                onclose.run(());
                            }
                        }
                    }
                });
            };

            let close = move |_| {
                if let Some(onclose) = onclose {
                    onclose.run(());
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
            let container = expect_context::<ActiveResource>();
            let messages = expect_context::<types::Messages>();
            let input_debounce = expect_context::<InputDebounce>();
            let (input_value, set_input_value) = signal(value.get_untracked());
            let oninput = Callback::new(set_input_value);

            let _ = Effect::watch(
                input_value,
                {
                    let key = key.clone();
                    let project = project.clone();
                    let graph = graph.clone();
                    let container = container.clone();
                    let messages = messages.clone();
                    move |value, _, container_id| -> ResourceId {
                        let messages = messages.write_only();
                        if container.with_untracked(|rid| {
                            if let Some(container_id) = container_id {
                                *rid != container_id
                            } else {
                                false
                            }
                        }) {
                            return container.get_untracked();
                        }

                        let node = container.with_untracked(|rid| graph.find_by_id(rid).unwrap());
                        let path = graph.path(&node).unwrap();
                        let mut properties = node.properties().with_untracked(|properties| {
                            let db::state::DataResource::Ok(properties) = properties else {
                                panic!("invalid state");
                            };

                            properties.as_properties()
                        });

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
                        properties.metadata.insert(key.clone(), value);

                        spawn_local({
                            let project = project.rid().get_untracked();
                            let messages = messages.clone();

                            async move {
                                if let Err(err) = update_properties(project, path, properties).await
                                {
                                    tracing::error!(?err);
                                    let mut msg =
                                        types::message::Builder::error("Could not save container.");
                                    msg.body(format!("{err:?}"));
                                    messages.update(|messages| messages.push(msg.build()));
                                }
                            }
                        });

                        // return the current id to track if the container changed
                        container.get_untracked()
                    }
                },
                false,
            );

            let remove_datum = {
                let project = project.clone();
                let graph = graph.clone();
                let messages = messages.clone();
                let key = key.clone();
                move |e: MouseEvent| {
                    if e.button() != types::MouseButton::Primary {
                        return;
                    }

                    let node = container.with_untracked(|rid| graph.find_by_id(rid).unwrap());
                    let mut properties = node.properties().with_untracked(|properties| {
                        let db::state::DataResource::Ok(properties) = properties else {
                            panic!("invalid state");
                        };

                        properties.as_properties()
                    });
                    properties.metadata.retain(|k, _| k != &key);

                    spawn_local({
                        let project = project.rid().get_untracked();
                        let path = graph.path(&node).unwrap();
                        let messages = messages.clone();

                        async move {
                            if let Err(err) = update_properties(project, path, properties).await {
                                tracing::error!(?err);
                                let mut msg =
                                    types::message::Builder::error("Could not save container.");
                                msg.body(format!("{err:?}"));
                                messages.update(|messages| messages.push(msg.build()));
                            }
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
                            class="aspect-square h-full rounded-sm hover:bg-secondary-200 dark:hover:bg-secondary-700"
                        >
                            <Icon icon=components::icon::Remove />
                        </button>
                    </div>
                    <ValueEditor value=input_value oninput debounce=*input_debounce />
                </div>
            }
        }
    }

    mod analysis_associations {
        use super::super::{
            super::common::analysis_associations::{
                AddAssociation as AddAssociationEditor, AnalysisInfo,
            },
            state,
        };
        use crate::{
            commands,
            components::{self, DetailPopout},
            pages::project::properties::InputDebounce,
            types,
        };
        use has_id::HasId;
        use leptos::{ev::MouseEvent, html, prelude::*};
        use leptos_icons::Icon;
        use syre_core::{project::AnalysisAssociation, types::ResourceId};
        use syre_local as local;
        use syre_local_database as db;

        #[component]
        pub fn AddAssociation(
            available_analyses: Signal<Vec<AnalysisInfo>>,
            container: ReadSignal<ResourceId>,
            #[prop(optional, into)] onclose: Option<Callback<()>>,
        ) -> impl IntoView {
            let project = expect_context::<state::Project>();
            let graph = expect_context::<state::Graph>();
            let messages = expect_context::<types::Messages>();

            let add_association: Action<_, _> =
                Action::new_unsync(move |association: &AnalysisAssociation| {
                    let node = container.with_untracked(|rid| graph.find_by_id(rid).unwrap());
                    let mut associations = node.analyses().with_untracked(|associations| {
                        let db::state::DataResource::Ok(associations) = associations else {
                            panic!("invalid state");
                        };

                        associations.with_untracked(|associations| {
                            associations
                                .iter()
                                .map(|association| association.as_association())
                                .collect::<Vec<_>>()
                        })
                    });
                    assert!(!associations
                        .iter()
                        .any(|assoc| assoc.analysis() == association.analysis()));
                    associations.push(association.clone());

                    let project = project.rid().get_untracked();
                    let container_path = graph.path(&node).unwrap();
                    let messages = messages.clone();
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
                                types::message::Builder::error("Could not save container.");
                            msg.body(format!("{err:?}"));
                            messages.update(|messages| messages.push(msg.build()));
                        };
                    }
                });

            let onadd = move |association: AnalysisAssociation| {
                add_association.dispatch(association);
                if let Some(onclose) = onclose {
                    onclose.run(());
                }
            };

            let close = move |_| {
                if let Some(onclose) = onclose {
                    onclose.run(());
                }
            };

            view! {
                <DetailPopout title="Add analysis" onclose=Callback::new(close)>
                    <AddAssociationEditor
                        available_analyses
                        onadd=Callback::new(onadd)
                        class="px-1"
                    />
                </DetailPopout>
            }
        }

        #[component]
        pub fn Editor(
            #[prop(into)] associations: Signal<Vec<state::AnalysisAssociation>>,
            container: ReadSignal<ResourceId>,
        ) -> impl IntoView {
            let project = expect_context::<state::Project>();
            let graph = expect_context::<state::Graph>();
            let messages = expect_context::<types::Messages>();

            let update_associations: Action<_, _> = Action::new_unsync({
                let project = project.rid().read_only();
                let graph = graph.clone();
                let container = container.clone();
                let messages = messages.clone();
                move |associations: &Vec<AnalysisAssociation>| {
                    let node = container.with_untracked(|rid| graph.find_by_id(rid).unwrap());
                    let container_path = graph.path(&node).unwrap();

                    let project = project.get_untracked();
                    let associations = associations.clone();
                    let messages = messages.clone();
                    async move {
                        if let Err(err) = commands::container::update_analysis_associations(
                            project,
                            container_path,
                            associations,
                        )
                        .await
                        {
                            tracing::error!(?err);
                            let mut msg = types::message::Builder::error(
                                "Could not update analysis associations.",
                            );
                            msg.body(format!("{err:?}"));
                            messages.update(|messages| messages.push(msg.build()));
                        }
                    }
                }
            });

            let remove_association = move |e: MouseEvent, analysis: ResourceId| {
                if e.button() != types::MouseButton::Primary {
                    return;
                }

                let mut associations = associations.get_untracked();
                associations.retain(|association| *association.analysis() != analysis);
                let associations = associations
                    .into_iter()
                    .map(|association| association.as_association())
                    .collect();
                update_associations.dispatch(associations);
            };

            view! {
                <div>
                    <Show
                        when=move || associations.with(|associations| !associations.is_empty())
                        fallback=|| view! { "(no analyses)" }
                    >
                        <For
                            each=associations
                            key=|association| association.analysis().clone()
                            let:association
                        >
                            <div class="relative flex gap-2">
                                <AnalysisAssociationEditor
                                    association=association.clone()
                                    container
                                    class="grow"
                                />
                                <button
                                    type="button"
                                    on:mousedown=move |e| remove_association(
                                        e,
                                        association.analysis().clone(),
                                    )
                                    class="aspect-square h-full rounded-sm hover:bg-secondary-200 dark:hover:bg-secondary-700"
                                >
                                    <Icon icon=components::icon::Remove />
                                </button>
                            </div>
                        </For>
                    </Show>
                </div>
            }
        }

        #[component]
        pub fn AnalysisAssociationEditor(
            association: state::AnalysisAssociation,
            container: ReadSignal<ResourceId>,
            #[prop(into)] class: MaybeProp<String>,
        ) -> impl IntoView {
            let project = expect_context::<state::Project>();
            let graph = expect_context::<state::Graph>();
            let messages = expect_context::<types::Messages>();
            let input_debounce = expect_context::<InputDebounce>();

            let autorun_input_node = NodeRef::<html::Input>::new();
            let (value, set_value) = signal(AnalysisAssociation::with_params(
                association.analysis().clone(),
                association.autorun().get_untracked(),
                association.priority().get_untracked(),
            ));
            let value = leptos_use::signal_debounced(value, *input_debounce);

            let _ = Effect::watch(
                {
                    let autorun = association.autorun().read_only();
                    move || autorun.get()
                },
                move |autorun, _, _| {
                    let input = autorun_input_node.get_untracked().unwrap();
                    input.set_checked(*autorun);
                },
                false,
            );

            let update_association: Action<_, _> = Action::new_unsync({
                let graph = graph.clone();
                let project = project.rid().read_only();
                move |association: &AnalysisAssociation| {
                    let node = container.with_untracked(|rid| graph.find_by_id(rid).unwrap());
                    let mut associations = node.analyses().with_untracked(|associations| {
                        let db::state::DataResource::Ok(associations) = associations else {
                            panic!("invalid state");
                        };

                        associations.with_untracked(|associations| {
                            associations
                                .iter()
                                .map(|association| association.as_association())
                                .collect::<Vec<_>>()
                        })
                    });
                    let association = associations
                        .iter_mut()
                        .find(|assoc| assoc.analysis() == association.analysis())
                        .unwrap();
                    *association = value.get_untracked();

                    let project = project.get_untracked();
                    let container_path = graph.path(&node).unwrap();
                    let messages = messages.write_only();
                    async move {
                        if let Err(err) = commands::container::update_analysis_associations(
                            project,
                            container_path,
                            associations,
                        )
                        .await
                        {
                            tracing::error!(?err);
                            let mut msg = types::message::Builder::error(
                                "Could not update analysis associations.",
                            );
                            msg.body(format!("{err:?}"));
                            messages.update(|messages| messages.push(msg.build()));
                        };
                    }
                }
            });

            let _ = Effect::watch(
                value,
                move |value, _, _| {
                    update_association.dispatch(value.clone());
                },
                false,
            );

            let title = {
                let association = association.clone();
                let analyses = project.analyses();
                move || {
                    analyses.with(|analyses| {
                        let db::state::DataResource::Ok(analyses) = analyses else {
                            return association.analysis().to_string();
                        };

                        analyses
                            .with(|analyses| {
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
                                                    template
                                                        .template
                                                        .path
                                                        .to_string_lossy()
                                                        .to_string()
                                                }
                                            }
                                        };

                                        Some(title)
                                    })
                                })
                            })
                            .unwrap_or_else(|| return association.analysis().to_string())
                    })
                }
            };

            let classes = move || {
                class.with(|class| {
                    if let Some(class) = class {
                        format!("flex flex-wrap {class}")
                    } else {
                        "flex flex-wrap".to_string()
                    }
                })
            };

            view! {
                <div class=classes>
                    <div title=title.clone() class="grow">
                        {title.clone()}
                    </div>
                    <div class="inline-flex gap-2">
                        <input
                            type="number"
                            name="priority"
                            prop:value=move || value.with(|value| value.priority.clone())
                            on:input=move |e| {
                                set_value
                                    .update(|value| {
                                        let priority = event_target_value(&e)
                                            .parse::<i32>()
                                            .unwrap();
                                        value.priority = priority;
                                    })
                            }

                            // TODO: May not want to use hard coded width
                            class="input-compact w-14"
                        />

                        <input
                            node_ref=autorun_input_node
                            type="checkbox"
                            name="autorun"
                            checked=value.with_untracked(|value| value.autorun)
                            on:input=move |e| {
                                set_value.update(|value| value.autorun = event_target_checked(&e))
                            }

                            class="input-compact"
                        />
                    </div>
                </div>
            }
        }
    }

    async fn update_properties(
        project: ResourceId,
        container: impl Into<PathBuf>,
        properties: syre_core::project::ContainerProperties,
    ) -> Result<(), lib::command::container::error::Update> {
        #[derive(Serialize)]
        struct Args {
            project: ResourceId,
            container: PathBuf,
            properties: syre_core::project::ContainerProperties,
        }

        tauri_sys::core::invoke_result(
            "container_properties_update",
            Args {
                project,
                container: container.into(),
                properties,
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
    pub fn Editor(container: state::graph::Node) -> impl IntoView {
        let graph = expect_context::<state::Graph>();
        let flags_state = expect_context::<state::Flags>();
        let container_path = graph.path(&container).unwrap();
        let flags = flags_state.find(&container_path);
        view! {
            <div>
                <super::Header />
                {move || {
                    if flags.read().as_ref().map(|flags| flags.read().is_empty()).unwrap_or(true) {
                        Either::Left(view! { <div class="text-lg text-center">"(No flags)"</div> })
                    } else {
                        Either::Right(
                            view! {
                                <Flags
                                    container=container_path.clone()
                                    flags=flags.read().as_ref().unwrap().read_only()
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
        flags: ArcReadSignal<Vec<local::project::resources::Flag>>,
    ) -> impl IntoView {
        let project = expect_context::<state::Project>();
        let messages = expect_context::<types::Messages>();

        let remove_all_action = Action::new_local({
            let project = project.path().read_only();
            let container = container.clone();
            move |_| {
                let container = container.clone();
                async move {
                    if let Err(err) = commands::flag::remove_all(
                        project.get_untracked(),
                        container,
                        PathBuf::from("/"),
                    )
                    .await
                    {
                        let mut msg = types::message::Builder::error("Could not remove flags.");
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
                        class="px-4 bg-secondary-50 dark:bg-secondary-600 rounded border"
                        disabled=remove_all_action.pending()
                    >
                        "Remove all"
                    </button>
                </div>
                <ul>
                    <For each=flags key=move |flag| flag.id().clone() let:flag>
                        <li class="px-2">
                            <Flag container=container.clone() flag=flag.clone() />
                        </li>
                    </For>
                </ul>
            </div>
        }
    }

    #[component]
    fn Flag(container: PathBuf, flag: local::project::resources::Flag) -> impl IntoView {
        let project = expect_context::<state::Project>();
        let messages = expect_context::<types::Messages>();

        let remove_flag_action = Action::new_local({
            let project = project.path();
            let flag = flag.id().clone();
            let container = container.clone();
            move |_| {
                let flag = flag.clone();
                let container = container.clone();
                async move {
                    if let Err(err) = commands::flag::remove(
                        project.get_untracked(),
                        container,
                        PathBuf::from("/"),
                        flag,
                    )
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
