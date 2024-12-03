use super::{InputDebounce, PopoutPortal};
use crate::{components, pages::project, types};
use analysis_associations::{AddAssociation, Editor as AnalysisAssociations};
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
use state::{ActiveResources, State};
use std::path::PathBuf;
use syre_core::types::ResourceId;
use syre_desktop_lib as lib;
use tags::{AddTags, Editor as Tags};

#[derive(Clone, Copy)]
enum Widget {
    AddTags,
    AddMetadatum,
    AddAnalysisAssociation,
}

mod state {
    use super::{super::common::bulk, analysis_associations};
    use crate::pages::project::state;
    use leptos::*;
    use std::collections::HashMap;
    use syre_core::types::ResourceId;
    use syre_local_database as db;

    #[derive(Clone, Debug)]
    pub struct State {
        names: Vec<ReadSignal<String>>,
        kinds: Vec<ReadSignal<Option<String>>>,
        descriptions: Vec<ReadSignal<Option<String>>>,
        tags: Vec<ReadSignal<Vec<String>>>,
        metadata: Vec<ReadSignal<state::Metadata>>,
        analyses: Vec<ReadSignal<state::container::AnalysesState>>,
    }

    impl State {
        pub fn from_states(states: Vec<state::graph::Node>) -> Self {
            let mut names = Vec::with_capacity(states.len());
            let mut kinds = Vec::with_capacity(states.len());
            let mut descriptions = Vec::with_capacity(states.len());
            let mut tags = Vec::with_capacity(states.len());
            let mut metadata = Vec::with_capacity(states.len());
            states
                .iter()
                .map(|state| {
                    state.properties().with_untracked(|properties| {
                        let db::state::DataResource::Ok(properties) = properties else {
                            panic!("invalid state");
                        };

                        (
                            properties.name().read_only(),
                            properties.kind().read_only(),
                            properties.description().read_only(),
                            properties.tags().read_only(),
                            properties.metadata().read_only(),
                        )
                    })
                })
                .fold((), |(), (name, kind, description, tag, metadatum)| {
                    names.push(name);
                    kinds.push(kind);
                    descriptions.push(description);
                    tags.push(tag);
                    metadata.push(metadatum);
                });

            let analyses = states
                .iter()
                .map(|state| state.analyses().read_only())
                .collect();

            Self {
                names,
                kinds,
                descriptions,
                tags,
                metadata,
                analyses,
            }
        }
    }

    impl State {
        pub fn name(&self) -> Signal<bulk::Value<String>> {
            Signal::derive({
                let names = self.names.clone();
                move || {
                    let mut values = names.iter().map(|name| name.get()).collect::<Vec<_>>();
                    values.sort();
                    values.dedup();

                    match &values[..] {
                        [value] => bulk::Value::Equal(value.clone()),
                        _ => bulk::Value::Mixed,
                    }
                }
            })
        }

        pub fn kind(&self) -> Signal<bulk::Value<Option<String>>> {
            Signal::derive({
                let kinds = self.kinds.clone();
                move || {
                    let mut values = kinds.iter().map(|kind| kind.get()).collect::<Vec<_>>();
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

        /// Intersection of analyses associations.
        pub fn analyses(&self) -> Signal<Vec<analysis_associations::State>> {
            Signal::derive({
                let states = self.analyses.clone();
                move || {
                    let mut analyses = HashMap::new();
                    states.iter().for_each(|state| {
                        state.with(|state| {
                            let db::state::DataResource::Ok(state) = state else {
                                unreachable!("invalid state");
                            };

                            state.with(|associations| {
                                associations.iter().for_each(|association| {
                                    let entry = analyses
                                        .entry(association.analysis().clone())
                                        .or_insert(vec![]);
                                    entry.push((association.priority(), association.autorun()));
                                });
                            });
                        });
                    });

                    analyses
                        .into_iter()
                        .filter_map(|(analysis, run_params)| {
                            if run_params.len() != states.len() {
                                return None;
                            }

                            let (priorities, autoruns): (Vec<_>, Vec<_>) =
                                run_params.into_iter().unzip();

                            let priorities = priorities
                                .into_iter()
                                .map(|priority| priority.read_only())
                                .collect();

                            let autoruns = autoruns
                                .into_iter()
                                .map(|autorun| autorun.read_only())
                                .collect();

                            Some(analysis_associations::State::new(
                                analysis, priorities, autoruns,
                            ))
                        })
                        .collect()
                }
            })
        }
    }

    #[derive(derive_more::Deref, Clone)]
    pub struct ActiveResources(Signal<Vec<ResourceId>>);
    impl ActiveResources {
        pub fn new(resources: Signal<Vec<ResourceId>>) -> Self {
            Self(resources)
        }
    }
}

#[component]
pub fn Editor(containers: Signal<Vec<ResourceId>>) -> impl IntoView {
    assert!(containers.with(|containers| containers.len()) > 1);
    let graph = expect_context::<project::state::Graph>();
    let popout_portal = expect_context::<PopoutPortal>();
    let (widget, set_widget) = create_signal(None);
    let wrapper_node = NodeRef::<html::Div>::new();
    let tags_node = NodeRef::<html::Div>::new();
    let metadata_node = NodeRef::<html::Div>::new();
    let analyses_node = NodeRef::<html::Div>::new();

    provide_context(Signal::derive(move || {
        let states = containers.with(|containers| {
            containers
                .iter()
                .map(|rid| graph.find_by_id(rid).unwrap())
                .collect::<Vec<_>>()
        });

        State::from_states(states)
    }));

    provide_context(ActiveResources::new(containers.clone()));

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
        let wrapper = wrapper_node.get_untracked().unwrap();
        let base = metadata_node.get_untracked().unwrap();
        let portal = popout_portal.get_untracked().unwrap();

        let top = super::detail_popout_top(&portal, &base, &wrapper);
        (*portal)
            .style()
            .set_property("top", &format!("{top}px"))
            .unwrap();

        if e.button() == types::MouseButton::Primary {
            set_widget.update(|widget| {
                #[allow(unused_must_use)]
                {
                    widget.insert(Widget::AddMetadatum);
                }
            });
        }
    };

    let show_add_analysis = move |e: MouseEvent| {
        let wrapper = wrapper_node.get_untracked().unwrap();
        let base = analyses_node.get_untracked().unwrap();
        let portal = popout_portal.get_untracked().unwrap();

        let top = super::detail_popout_top(&portal, &base, &wrapper);
        (*portal)
            .style()
            .set_property("top", &format!("{top}px"))
            .unwrap();

        if e.button() == types::MouseButton::Primary {
            set_widget.update(|widget| {
                #[allow(unused_must_use)]
                {
                    widget.insert(Widget::AddAnalysisAssociation);
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
                Widget::AddAnalysisAssociation => todo!(),
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
                <h3 class="font-primary">"Bulk containers"</h3>
                <span class="text-sm text-secondary-500 dark:text-secondary-400">
                    "Editing " {move || containers.with(|containers| containers.len())}
                    " containers"
                </span>
            </div>
            <form on:submit=move |e| e.prevent_default()>
                <div class="px-1 pb-1">
                    <label>
                        <span class="block">"Name"</span>
                        <Name />
                    </label>
                </div>
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
                    class="py-4 border-t border-t-secondary-200 dark:border-t-secondary-700"
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
                    class="py-4 border-t border-t-secondary-200 dark:border-t-secondary-700"
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
                <div
                    ref=analyses_node
                    class="py-4 border-t border-t-secondary-200 dark:border-t-secondary-700"
                >
                    <label class="px-1 block">
                        <div class="flex">
                            <span class="grow">"Analyses"</span>
                            <span>
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
                                                            |widget| matches!(widget, Widget::AddAnalysisAssociation),
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
                                                            |widget| !matches!(widget, Widget::AddAnalysisAssociation),
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
                        <AnalysisAssociations />
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
                                Widget::AddAnalysisAssociation => {
                                    view! { <AddAssociation onclose=on_widget_close.clone() /> }
                                }
                            }}
                        </Portal>
                    }
                }}
            </Show>

        </div>
    }
}

mod name {
    use super::{super::common::bulk::Value, ActiveResources, InputDebounce, State};
    use crate::{pages::project::state, types};
    use leptos::*;
    use serde::Serialize;
    use std::{ffi::OsString, path::PathBuf};
    use syre_core::types::ResourceId;
    use syre_desktop_lib as lib;
    use syre_local_database as db;

    #[component]
    pub fn Editor() -> impl IntoView {
        let project = expect_context::<state::Project>();
        let graph = expect_context::<state::Graph>();
        let messages = expect_context::<types::Messages>();
        let containers = expect_context::<ActiveResources>();
        let state = expect_context::<Signal<State>>();
        let input_debounce = expect_context::<InputDebounce>();

        let (input_error, set_input_error) = create_signal(false);
        let (input_value, set_input_value) = create_signal({
            state.with(|state| match state.name().get() {
                Value::Mixed => String::new(),
                Value::Equal(value) => value.clone(),
            })
        });
        let input_value = leptos_use::signal_debounced(input_value, *input_debounce);

        let _ = watch(
            input_value,
            move |input_value, _, _| {
                set_input_error(false);
                spawn_local({
                    let project = project.rid().get_untracked();
                    let containers = containers.with_untracked(|containers| {
                        containers
                            .iter()
                            .map(|container| {
                                let node = graph.find_by_id(container).unwrap();
                                graph.path(&node).unwrap()
                            })
                            .collect::<Vec<_>>()
                    });

                    let messages = messages.clone();
                    let input_value = input_value.clone();
                    async move {
                        match rename_containers(project, containers.clone(), input_value.clone())
                            .await
                        {
                            Ok(rename_results) => {
                                assert_eq!(containers.len(), rename_results.len());
                                let rename_errors = rename_results
                                    .into_iter()
                                    .enumerate()
                                    .filter_map(|(idx, result)| {
                                        if let Err(err) = result {
                                            Some((containers[idx].clone(), err))
                                        } else {
                                            None
                                        }
                                    })
                                    .collect::<Vec<_>>();

                                if rename_errors.len() > 0 {
                                    messages.update(|messages| {
                                        let mut msg = types::message::Builder::error(
                                            "An error ocurred when renaming container folders.",
                                        );
                                        msg.body(
                                            view! { <ErrRenameIoMessage errors=rename_errors /> },
                                        );
                                        messages.push(msg.build());
                                    });
                                }
                            }
                            Err(err) => match err {
                                lib::command::container::bulk::error::Rename::ProjectNotFound => {
                                    todo!()
                                }
                                lib::command::container::bulk::error::Rename::NameCollision(
                                    paths,
                                ) => {
                                    set_input_error(true);
                                    messages.update(|messages| {
                                        let mut msg = types::message::Builder::error(
                                            "Could not rename containers",
                                        );
                                        msg.body(view! { <ErrNameCollisionMessage paths /> });
                                        messages.push(msg.build());
                                    });
                                }
                            },
                        }
                    }
                });
            },
            false,
        );

        let placeholder = move || {
            state.with(|state| {
                state.name().with(|state| match state {
                    Value::Mixed => "(mixed)".to_string(),
                    Value::Equal(_) => "(empty)".to_string(),
                })
            })
        };

        view! {
            <input
                type="text"
                prop:value=Signal::derive(input_value)
                on:input=move |e| {
                    set_input_value(event_target_value(&e));
                }

                debounce=*input_debounce
                placeholder=placeholder
                minlength="1"
                class=(["border-red-600", "border-solid", "border-2"], input_error)
                class="input-compact w-full"
            />
        }
    }

    async fn rename_containers(
        project: ResourceId,
        containers: Vec<PathBuf>,
        name: impl Into<OsString>,
    ) -> Result<
        Vec<Result<(), lib::command::error::IoErrorKind>>,
        lib::command::container::bulk::error::Rename,
    > {
        #[derive(Serialize)]
        struct RenameContainerArgs {
            project: ResourceId,
            containers: Vec<PathBuf>,
            #[serde(with = "db::serde_os_string")]
            name: OsString,
        }

        tauri_sys::core::invoke_result(
            "container_rename_bulk",
            RenameContainerArgs {
                project,
                containers,
                name: name.into(),
            },
        )
        .await
    }

    #[component]
    fn ErrNameCollisionMessage(paths: Vec<PathBuf>) -> impl IntoView {
        view! {
            <div>
                <p>
                    "The paths"
                    <ul>
                        {paths
                            .iter()
                            .map(|path| {
                                view! { <li>{path.to_string_lossy().to_string()}</li> }
                            })
                            .collect::<Vec<_>>()}

                    </ul> "resulted in a name collistion."
                </p>
                <p>"No containers were renamed."</p>
            </div>
        }
    }

    #[component]
    fn ErrRenameIoMessage(
        errors: Vec<(PathBuf, lib::command::error::IoErrorKind)>,
    ) -> impl IntoView {
        view! {
            <div>
                <p>
                    <ul>
                        {errors
                            .iter()
                            .map(|(path, err)| {
                                view! {
                                    <li>
                                        <strong>{path.to_string_lossy().to_string()}:</strong>
                                        {format!("{err:?}")}
                                    </li>
                                }
                            })
                            .collect::<Vec<_>>()}

                    </ul>
                </p>
                <p>"All other containers were renamed."</p>
            </div>
        }
    }
}

mod kind {
    use super::{
        super::{common::bulk::kind::Editor as KindEditor, errors_to_list_view},
        update_properties, ActiveResources, InputDebounce, State,
    };
    use crate::{pages::project::state, types};
    use leptos::*;
    use syre_desktop_lib::command::container::bulk::PropertiesUpdate;

    #[component]
    pub fn Editor() -> impl IntoView {
        let project = expect_context::<state::Project>();
        let graph = expect_context::<state::Graph>();
        let messages = expect_context::<types::Messages>();
        let containers = expect_context::<ActiveResources>();
        let state = expect_context::<Signal<State>>();
        let input_debounce = expect_context::<InputDebounce>();

        let oninput = Callback::new(move |input_value: Option<String>| {
            let containers_len = containers.with_untracked(|containers| containers.len());
            let mut update = PropertiesUpdate::default();
            let _ = update.kind.insert(input_value.clone());
            spawn_local({
                let project = project.rid().get_untracked();
                let containers = containers.with_untracked(|containers| {
                    containers
                        .iter()
                        .map(|container| {
                            let node = graph.find_by_id(container).unwrap();
                            graph.path(&node).unwrap()
                        })
                        .collect::<Vec<_>>()
                });

                async move {
                    match update_properties(project, containers, update).await {
                        Err(err) => {
                            tracing::error!(?err);
                            todo!();
                        }

                        Ok(container_results) => {
                            assert_eq!(container_results.len(), containers_len);
                            let errors = container_results
                                .into_iter()
                                .filter_map(|err| err.err())
                                .collect::<Vec<_>>();

                            if !errors.is_empty() {
                                let mut msg =
                                    types::message::Builder::error("Could not save properties.");
                                msg.body(errors_to_list_view(errors));
                                messages.update(|messages| messages.push(msg.build()));
                            }
                        }
                    }
                }
            });
        });

        view! { <KindEditor value=state.with(|state| state.kind()) oninput debounce=*input_debounce /> }
    }
}

mod description {
    use super::{
        super::{common::bulk::description::Editor as DescriptionEditor, errors_to_list_view},
        update_properties, ActiveResources, InputDebounce, State,
    };
    use crate::{
        pages::project::state,
        types::{self, Messages},
    };
    use leptos::*;
    use syre_desktop_lib::command::container::bulk::PropertiesUpdate;

    #[component]
    pub fn Editor() -> impl IntoView {
        let project = expect_context::<state::Project>();
        let graph = expect_context::<state::Graph>();
        let messages = expect_context::<Messages>();
        let containers = expect_context::<ActiveResources>();
        let state = expect_context::<Signal<State>>();
        let input_debounce = expect_context::<InputDebounce>();

        let oninput = Callback::new(move |input_value: Option<String>| {
            let containers_len = containers.with_untracked(|containers| containers.len());
            let mut update = PropertiesUpdate::default();
            let _ = update.description.insert(input_value.clone());
            spawn_local({
                let project = project.rid().get_untracked();
                let containers = containers.with_untracked(|containers| {
                    containers
                        .iter()
                        .map(|container| {
                            let node = graph.find_by_id(container).unwrap();
                            graph.path(&node).unwrap()
                        })
                        .collect::<Vec<_>>()
                });

                async move {
                    match update_properties(project, containers, update).await {
                        Err(err) => {
                            tracing::error!(?err);
                            todo!();
                        }

                        Ok(container_results) => {
                            assert_eq!(container_results.len(), containers_len);
                            let errors = container_results
                                .into_iter()
                                .filter_map(|err| err.err())
                                .collect::<Vec<_>>();

                            if !errors.is_empty() {
                                let mut msg =
                                    types::message::Builder::error("Could not save properties.");
                                msg.body(errors_to_list_view(errors));
                                messages.update(|messages| messages.push(msg.build()));
                            }
                        }
                    }
                }
            });
        });

        view! {
            <DescriptionEditor
                value=state.with(|state| state.description())
                oninput
                debounce=*input_debounce
                class="input-compact w-full align-top"
            />
        }
    }
}

mod tags {
    use super::{
        super::{
            common::bulk::tags::{AddTags as AddTagsEditor, Editor as TagsEditor},
            errors_to_list_view,
        },
        update_properties, ActiveResources, State,
    };
    use crate::{components::DetailPopout, pages::project::state, types};
    use leptos::*;
    use syre_desktop_lib::command::{bulk::TagsAction, container::bulk::PropertiesUpdate};

    #[component]
    pub fn Editor() -> impl IntoView {
        let project = expect_context::<state::Project>();
        let graph = expect_context::<state::Graph>();
        let messages = expect_context::<types::Messages>();
        let containers = expect_context::<ActiveResources>();
        let state = expect_context::<Signal<State>>();
        let onremove = Callback::new({
            let graph = graph.clone();
            let project = project.clone();
            let containers = containers.clone();
            move |value: String| {
                if value.is_empty() {
                    return;
                };

                let containers_len = containers.with_untracked(|containers| containers.len());
                let mut update = PropertiesUpdate::default();
                update.tags = TagsAction {
                    insert: vec![],
                    remove: vec![value.clone()],
                };
                spawn_local({
                    let project = project.rid().get_untracked();
                    let containers = containers.with_untracked(|containers| {
                        containers
                            .iter()
                            .map(|container| {
                                let node = graph.find_by_id(container).unwrap();
                                graph.path(&node).unwrap()
                            })
                            .collect::<Vec<_>>()
                    });

                    async move {
                        match update_properties(project, containers, update).await {
                            Err(err) => {
                                tracing::error!(?err);
                                todo!();
                            }

                            Ok(container_results) => {
                                assert_eq!(container_results.len(), containers_len);
                                let errors = container_results
                                    .into_iter()
                                    .filter_map(|err| err.err())
                                    .collect::<Vec<_>>();

                                if !errors.is_empty() {
                                    let mut msg = types::message::Builder::error(
                                        "Could not save properties.",
                                    );
                                    msg.body(errors_to_list_view(errors));
                                    messages.update(|messages| messages.push(msg.build()));
                                }
                            }
                        }
                    }
                });
            }
        });

        view! { <TagsEditor value=state.with(|state| { state.tags() }) onremove /> }
    }

    #[component]
    pub fn AddTags(#[prop(optional, into)] onclose: Option<Callback<()>>) -> impl IntoView {
        let project = expect_context::<state::Project>();
        let graph = expect_context::<state::Graph>();
        let messages = expect_context::<types::Messages>();
        let containers = expect_context::<ActiveResources>();
        let state = expect_context::<Signal<State>>();
        let (reset_form, set_reset_form) = create_signal(());
        let onadd = Callback::new(move |tags: Vec<String>| {
            if tags.is_empty() {
                return;
            };

            let containers_len = containers.with_untracked(|containers| containers.len());
            let mut update = PropertiesUpdate::default();
            update.tags = TagsAction {
                insert: tags.clone(),
                remove: vec![],
            };
            spawn_local({
                let project = project.rid().get_untracked();
                let containers = containers.with_untracked(|containers| {
                    containers
                        .iter()
                        .map(|container| {
                            let node = graph.find_by_id(container).unwrap();
                            graph.path(&node).unwrap()
                        })
                        .collect::<Vec<_>>()
                });

                async move {
                    match update_properties(project, containers, update).await {
                        Err(err) => {
                            let mut msg =
                                types::message::Builder::error("Could not save properties.");
                            msg.body(format!("{err:?}"));
                            messages.update(|messages| messages.push(msg.build()));
                        }

                        Ok(container_results) => {
                            assert_eq!(container_results.len(), containers_len);
                            let errors = container_results
                                .into_iter()
                                .filter_map(|err| err.err())
                                .collect::<Vec<_>>();

                            if errors.is_empty() {
                                if let Some(onclose) = onclose {
                                    onclose(());
                                }
                            } else {
                                let mut msg =
                                    types::message::Builder::error("Could not save properties.");
                                msg.body(errors_to_list_view(errors));
                                messages.update(|messages| messages.push(msg.build()));
                            }
                        }
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
                <AddTagsEditor onadd=Callback::new(onadd) reset=reset_form class="w-full px-1" />
            </DetailPopout>
        }
    }
}

mod metadata {
    use super::{
        super::{
            common::{
                bulk::metadata::Editor as MetadataEditor, metadata::AddDatum as AddDatumEditor,
            },
            errors_to_list_view,
        },
        update_properties, ActiveResources, InputDebounce, State,
    };
    use crate::{components::DetailPopout, pages::project::state, types};
    use leptos::*;
    use syre_core::types::data;
    use syre_desktop_lib::command::{bulk::MetadataAction, container::bulk::PropertiesUpdate};

    #[component]
    pub fn Editor() -> impl IntoView {
        let project = expect_context::<state::Project>();
        let graph = expect_context::<state::Graph>();
        let messages = expect_context::<types::Messages>();
        let containers = expect_context::<ActiveResources>();
        let state = expect_context::<Signal<State>>();
        let input_debounce = expect_context::<InputDebounce>();
        let (modifications, set_modifications) = create_signal(vec![]);
        let modifications = leptos_use::signal_debounced(modifications, *input_debounce);

        let onremove = Callback::new({
            let project = project.rid().read_only();
            let graph = graph.clone();
            let containers = containers.clone();
            move |value: String| {
                let containers_len = containers.with_untracked(|containers| containers.len());
                let mut update = PropertiesUpdate::default();
                update.metadata = MetadataAction {
                    add: vec![],
                    update: vec![],
                    remove: vec![value.clone()],
                };

                spawn_local({
                    let project = project.get_untracked();
                    let containers = containers.with_untracked(|containers| {
                        containers
                            .iter()
                            .map(|container| {
                                let node = graph.find_by_id(container).unwrap();
                                graph.path(&node).unwrap()
                            })
                            .collect::<Vec<_>>()
                    });

                    async move {
                        match update_properties(project, containers, update).await {
                            Err(err) => {
                                tracing::error!(?err);
                                todo!();
                            }

                            Ok(container_results) => {
                                assert_eq!(container_results.len(), containers_len);
                                let errors = container_results
                                    .into_iter()
                                    .filter_map(|err| err.err())
                                    .collect::<Vec<_>>();

                                if !errors.is_empty() {
                                    let mut msg = types::message::Builder::error(
                                        "Could not save properties.",
                                    );
                                    msg.body(errors_to_list_view(errors));
                                    messages.update(|messages| messages.push(msg.build()));
                                }
                            }
                        }
                    }
                });
            }
        });

        let onmodify = Callback::new(move |value: (String, data::Value)| {
            set_modifications.update(|modifications| modifications.push(value));
        });

        let _ = watch(
            modifications,
            {
                let project = project.rid();
                let graph = graph.clone();
                let containers = containers.clone();
                move |modifications, _, _| {
                    let containers_len = containers.with_untracked(|containers| containers.len());
                    let mut update = PropertiesUpdate::default();
                    update.metadata = MetadataAction {
                        add: vec![],
                        update: modifications.clone(),
                        remove: vec![],
                    };
                    set_modifications.update_untracked(|modifications| modifications.clear());

                    spawn_local({
                        let project = project.get_untracked();
                        let containers = containers.with_untracked(|containers| {
                            containers
                                .iter()
                                .map(|container| {
                                    let node = graph.find_by_id(container).unwrap();
                                    graph.path(&node).unwrap()
                                })
                                .collect::<Vec<_>>()
                        });

                        async move {
                            match update_properties(project, containers, update).await {
                                Err(err) => {
                                    let mut msg = types::message::Builder::error(
                                        "Could not save properties.",
                                    );
                                    msg.body(format!("{err:?}"));
                                    messages.update(|messages| messages.push(msg.build()));
                                }

                                Ok(container_results) => {
                                    assert_eq!(container_results.len(), containers_len);
                                    let errors = container_results
                                        .into_iter()
                                        .filter_map(|err| err.err())
                                        .collect::<Vec<_>>();

                                    if !errors.is_empty() {
                                        let mut msg = types::message::Builder::error(
                                            "Could not save properties.",
                                        );
                                        msg.body(errors_to_list_view(errors));
                                        messages.update(|messages| messages.push(msg.build()));
                                    }
                                }
                            }
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
        let messages = expect_context::<types::Messages>();
        let containers = expect_context::<ActiveResources>();
        let state = expect_context::<Signal<State>>();
        let onadd = Callback::new({
            let project = project.clone();
            let graph = graph.clone();
            let containers = containers.clone();
            move |value: (String, data::Value)| {
                let containers_len = containers.with_untracked(|containers| containers.len());
                let mut update = PropertiesUpdate::default();
                update.metadata = MetadataAction {
                    add: vec![value.clone()],
                    update: vec![],
                    remove: vec![],
                };

                spawn_local({
                    let project = project.rid().get_untracked();
                    let containers = containers.with_untracked(|containers| {
                        containers
                            .iter()
                            .map(|container| {
                                let node = graph.find_by_id(container).unwrap();
                                graph.path(&node).unwrap()
                            })
                            .collect::<Vec<_>>()
                    });

                    async move {
                        match update_properties(project, containers, update).await {
                            Err(err) => {
                                let mut msg =
                                    types::message::Builder::error("Could not save properties.");
                                msg.body(format!("{err:?}"));
                                messages.update(|messages| messages.push(msg.build()));
                            }

                            Ok(container_results) => {
                                assert_eq!(container_results.len(), containers_len);
                                let errors = container_results
                                    .into_iter()
                                    .filter_map(|err| err.err())
                                    .collect::<Vec<_>>();

                                if errors.is_empty() {
                                    if let Some(onclose) = onclose {
                                        onclose(());
                                    }
                                } else {
                                    let mut msg = types::message::Builder::error(
                                        "Could not save properties.",
                                    );
                                    msg.body(errors_to_list_view(errors));
                                    messages.update(|messages| messages.push(msg.build()));
                                }
                            }
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

mod analysis_associations {
    use super::{
        super::{
            common::{self, analysis_associations::AddAssociation as AddAssociationEditor},
            errors_to_list_view, InputDebounce,
        },
        ActiveResources,
    };
    use crate::{
        components::{self, DetailPopout},
        pages::project::{properties::common::bulk, state},
        types,
    };
    use has_id::HasId;
    use leptos::{ev::MouseEvent, *};
    use leptos_icons::Icon;
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    use syre_core::{project::AnalysisAssociation, types::ResourceId};
    use syre_desktop_lib::{
        self as lib,
        command::container::bulk::{AnalysisAssociationAction, AnalysisAssociationUpdate},
    };
    use syre_local as local;
    use syre_local_database::{self as db};

    #[derive(Clone)]
    pub struct State {
        analysis: ResourceId,
        priorities: Vec<ReadSignal<i32>>,
        autoruns: Vec<ReadSignal<bool>>,
    }

    impl State {
        pub fn new(
            analysis: ResourceId,
            priorities: Vec<ReadSignal<i32>>,
            autoruns: Vec<ReadSignal<bool>>,
        ) -> Self {
            Self {
                analysis,
                autoruns,
                priorities,
            }
        }

        pub fn analysis(&self) -> &ResourceId {
            &self.analysis
        }

        pub fn priority(&self) -> Signal<bulk::Value<i32>> {
            Signal::derive({
                let priorities = self.priorities.clone();
                move || {
                    let priority_ref = priorities[0].get();
                    if priorities
                        .iter()
                        .skip(1)
                        .all(|priority| priority.with(|pi| *pi == priority_ref))
                    {
                        bulk::Value::Equal(priority_ref)
                    } else {
                        bulk::Value::Mixed
                    }
                }
            })
        }

        pub fn autorun(&self) -> Signal<bulk::Value<bool>> {
            Signal::derive({
                let autoruns = self.autoruns.clone();
                move || {
                    let autorun_ref = autoruns[0].get();
                    if autoruns
                        .iter()
                        .skip(1)
                        .all(|autorun| autorun.with(|ai| *ai == autorun_ref))
                    {
                        bulk::Value::Equal(autorun_ref)
                    } else {
                        bulk::Value::Mixed
                    }
                }
            })
        }
    }

    #[component]
    pub fn Editor() -> impl IntoView {
        let project = expect_context::<state::Project>();
        let graph = expect_context::<state::Graph>();
        let messages = expect_context::<types::Messages>();
        let containers = expect_context::<ActiveResources>();
        let state = expect_context::<Signal<super::State>>();

        let remove_association = create_action({
            let project = project.rid().read_only();
            let graph = graph.clone();
            let containers = containers.clone();
            move |analysis: &ResourceId| {
                let containers_len = containers.with_untracked(|containers| containers.len());
                let mut update = AnalysisAssociationAction::default();
                update.remove.push(analysis.clone());

                let project = project.get_untracked();
                let containers = containers.with_untracked(|containers| {
                    containers
                        .iter()
                        .map(|container| {
                            let node = graph.find_by_id(container).unwrap();
                            graph.path(&node).unwrap()
                        })
                        .collect::<Vec<_>>()
                });

                async move {
                    match update_analysis_associations(project, containers, update).await {
                        Err(err) => {
                            let mut msg =
                                types::message::Builder::error("Could not save properties.");
                            msg.body(format!("{err:?}"));
                            messages.update(|messages| messages.push(msg.build()));
                        }

                        Ok(container_results) => {
                            assert_eq!(container_results.len(), containers_len);
                            let errors = container_results
                                .into_iter()
                                .filter_map(|err| err.err())
                                .collect::<Vec<_>>();

                            if !errors.is_empty() {
                                let mut msg =
                                    types::message::Builder::error("Could not save properties.");
                                msg.body(errors_to_list_view(errors));
                                messages.update(|messages| messages.push(msg.build()));
                            }
                        }
                    }
                }
            }
        });

        let onremove = move |analysis: ResourceId| {
            move |e: MouseEvent| {
                if e.button() != types::MouseButton::Primary {
                    return;
                }

                remove_association.dispatch(analysis.clone());
            }
        };

        view! {
            <div>
                <For
                    each=state.with_untracked(|state| state.analyses())
                    key=|association| association.analysis.clone()
                    let:association
                >
                    <div class="flex gap-2">
                        <AssociationEditor association=association.clone() class="grow" />
                        <div>
                            <button on:mousedown=onremove(association.analysis.clone())>
                                <Icon icon=components::icon::Remove />
                            </button>
                        </div>
                    </div>
                </For>
            </div>
        }
    }

    #[component]
    fn AssociationEditor(
        association: State,
        #[prop(optional, into)] class: MaybeSignal<String>,
    ) -> impl IntoView {
        let project = expect_context::<state::Project>();
        let graph = expect_context::<state::Graph>();
        let messages = expect_context::<types::Messages>();
        let containers = expect_context::<ActiveResources>();
        let input_debounce = expect_context::<InputDebounce>();

        let autorun_input_node = NodeRef::<html::Input>::new();
        let (value, set_value) = create_signal({
            let association = association.clone();
            let mut value = AnalysisAssociationUpdate::new(association.analysis.clone());
            value.autorun = association.autorun().get_untracked().equal();
            value.priority = association.priority().get_untracked().equal();
            value
        });
        let value = leptos_use::signal_debounced(value, *input_debounce);

        let _ = watch(
            move || value.get(),
            {
                let project = project.rid().read_only();
                let graph = graph.clone();
                let containers = containers.clone();
                move |value: &AnalysisAssociationUpdate, _, _| {
                    let containers_len = containers.with_untracked(|containers| containers.len());
                    let mut update = AnalysisAssociationAction::default();
                    update.update.push(value.clone());

                    spawn_local({
                        let project = project.get_untracked();
                        let containers = containers.with_untracked(|containers| {
                            containers
                                .iter()
                                .map(|container| {
                                    let node = graph.find_by_id(container).unwrap();
                                    graph.path(&node).unwrap()
                                })
                                .collect::<Vec<_>>()
                        });

                        async move {
                            match update_analysis_associations(project, containers, update).await {
                                Err(err) => {
                                    let mut msg = types::message::Builder::error(
                                        "Could not save properties.",
                                    );
                                    msg.body(format!("{err:?}"));
                                    messages.update(|messages| messages.push(msg.build()));
                                }

                                Ok(container_results) => {
                                    assert_eq!(container_results.len(), containers_len);
                                    let errors = container_results
                                        .into_iter()
                                        .filter_map(|err| err.err())
                                        .collect::<Vec<_>>();

                                    if !errors.is_empty() {
                                        let mut msg = types::message::Builder::error(
                                            "Could not save properties.",
                                        );
                                        msg.body(errors_to_list_view(errors));
                                        messages.update(|messages| messages.push(msg.build()));
                                    }
                                }
                            }
                        }
                    });
                }
            },
            false,
        );

        let title = {
            let association = association.clone();
            let analyses = project.analyses();
            move || {
                analyses.with(|analyses| {
                    let db::state::DataResource::Ok(analyses) = analyses else {
                        return association.analysis.to_string();
                    };

                    analyses
                        .with(|analyses| {
                            analyses.iter().find_map(|analysis| {
                                analysis.properties().with(|properties| {
                                    if *properties.id() != association.analysis {
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
                        .unwrap_or_else(|| return association.analysis.to_string())
                })
            }
        };

        create_effect({
            let autorun = association.autorun();
            move |_| {
                let Some(autorun_input) = autorun_input_node.get() else {
                    return;
                };

                autorun.with(|autorun| match autorun {
                    bulk::Value::Mixed => autorun_input.set_indeterminate(true),
                    bulk::Value::Equal(autorun) => {
                        autorun_input.set_indeterminate(false);
                        autorun_input.set_checked(*autorun)
                    }
                });
            }
        });

        let classes = move || format!("flex flex-wrap {}", class.get());
        view! {
            <div class=classes>
                <div title=title.clone() class="grow">
                    {title}
                </div>
                <div class="inline-flex gap-2">
                    <input
                        type="number"
                        name="priority"
                        prop:value=move || { association.priority().get().equal() }
                        on:input=move |e| {
                            set_value
                                .update(|value| {
                                    let priority = event_target_value(&e).parse::<i32>().unwrap();
                                    #[allow(unused_must_use)]
                                    {
                                        value.priority.insert(priority);
                                    }
                                })
                        }
                        placeholder="(mixed)"

                        // TODO: May not want to use hard coded width
                        class="input-compact w-14"
                    />

                    <input
                        ref=autorun_input_node
                        type="checkbox"
                        name="autorun"
                        on:input=move |e| {
                            set_value
                                .update(|value| {
                                    #[allow(unused_must_use)]
                                    {
                                        value.autorun.insert(event_target_checked(&e));
                                    }
                                })
                        }

                        class="input-compact"
                    />
                </div>
            </div>
        }
    }

    #[component]
    pub fn AddAssociation(#[prop(optional, into)] onclose: Option<Callback<()>>) -> impl IntoView {
        let project = expect_context::<state::Project>();
        let graph = expect_context::<state::Graph>();
        let messages = expect_context::<types::Messages>();
        let containers = expect_context::<ActiveResources>();
        let state = expect_context::<Signal<super::State>>();

        let available_analyses = {
            let analyses = project.analyses().read_only();
            move || {
                let db::state::DataResource::Ok(analyses) = analyses.get() else {
                    return vec![];
                };

                analyses.with(|analyses| {
                    analyses
                        .iter()
                        .filter_map(move |analysis| {
                            if state.with(|state| {
                                state.analyses().with(|associations| {
                                    !associations.iter().any(|association| {
                                        analysis.properties().with(|properties| {
                                            association.analysis == *properties.id()
                                        })
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
            }
        };
        let available_analyses = Signal::derive(available_analyses);

        let add_association = create_action({
            let project = project.rid().read_only();
            move |association: &AnalysisAssociation| {
                let container_paths = containers.with_untracked(|containers| {
                    containers
                        .iter()
                        .map(|container| {
                            let node = graph.find_by_id(container).unwrap();
                            graph.path(&node).unwrap()
                        })
                        .collect::<Vec<_>>()
                });

                let project = project.get_untracked();
                let messages = messages.clone();
                let association = association.clone();
                async move {
                    if let Err(err) =
                        add_analysis_association(project, container_paths, association).await
                    {
                        tracing::error!(?err);
                        let mut msg = types::message::Builder::error("Could not save container.");
                        msg.body(format!("{err:?}"));
                        messages.update(|messages| messages.push(msg.build()));
                    };
                }
            }
        });

        let onadd = move |association: AnalysisAssociation| {
            add_association.dispatch(association);
            if let Some(onclose) = onclose {
                onclose(());
            }
        };

        let close = move |_| {
            if let Some(onclose) = onclose {
                onclose(());
            }
        };

        view! {
            <DetailPopout title="Add metadata" onclose=Callback::new(close)>
                <AddAssociationEditor
                    available_analyses
                    onadd=Callback::new(onadd)
                    class="w-full px-1"
                />
            </DetailPopout>
        }
    }

    async fn add_analysis_association(
        project: ResourceId,
        containers: Vec<PathBuf>,
        analysis: AnalysisAssociation,
    ) -> Result<Vec<Result<(), local::error::IoSerde>>, lib::command::error::ProjectNotFound> {
        let mut update = AnalysisAssociationAction::default();
        update.add.push(analysis);
        update_analysis_associations(project, containers, update).await
    }

    async fn update_analysis_associations(
        project: ResourceId,
        containers: Vec<PathBuf>,
        update: AnalysisAssociationAction,
    ) -> Result<Vec<Result<(), local::error::IoSerde>>, lib::command::error::ProjectNotFound> {
        #[derive(Serialize, Deserialize)]
        struct Args {
            project: ResourceId,
            containers: Vec<PathBuf>,
            update: AnalysisAssociationAction,
        }

        tauri_sys::core::invoke_result(
            "container_analysis_associations_update_bulk",
            Args {
                project,
                containers,
                update,
            },
        )
        .await
    }
}

/// # Returns
/// Result of each Container's update.
async fn update_properties(
    project: ResourceId,
    containers: Vec<PathBuf>,
    update: lib::command::container::bulk::PropertiesUpdate,
) -> Result<
    Vec<Result<(), lib::command::container::bulk::error::Update>>,
    lib::command::error::ProjectNotFound,
> {
    #[derive(Serialize)]
    struct Args {
        project: ResourceId,
        containers: Vec<PathBuf>,
        // update: lib::command::container::bulk::PropertiesUpdate,
        update: String, // TODO: Issue with serializing enum with Option. perform manually.
                        // See: https://github.com/tauri-apps/tauri/issues/5993
    }

    tauri_sys::core::invoke_result(
        "container_properties_update_bulk",
        Args {
            project,
            containers,
            update: serde_json::to_string(&update).unwrap(),
        },
    )
    .await
}
