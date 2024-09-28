use super::{properties, state, workspace::PropertiesEditor};
use crate::types;
use leptos::{ev::MouseEvent, *};
use leptos_icons::Icon;
use std::path::PathBuf;
use syre_core::types::ResourceId;
use wasm_bindgen::{closure::Closure, JsCast};

#[component]
pub fn ProjectBar() -> impl IntoView {
    view! {
        <div class="flex px-2 py-1">
            <div class="grow inline-flex gap-2">
                <PreviewSelector />
                <Analyze />
            </div>
            <div>
                <ProjectInfo />
            </div>
            <div class="grow"></div>
        </div>
    }
}

#[component]
fn PreviewSelector() -> impl IntoView {
    const MENU_ID: &str = "workspace-preview-menu";

    let workspace_state = expect_context::<state::Workspace>();
    let state = workspace_state.preview().clone();
    let (active, set_active) = create_signal::<Option<Closure<dyn FnMut(MouseEvent)>>>(None);

    let preview_list = move || {
        let mut out = vec![];
        state.with(|state| {
            if state.assets {
                out.push("Data");
            }
            if state.analyses {
                out.push("Analyses");
            }
            if state.kind {
                out.push("Type");
            }
            if state.description {
                out.push("Description");
            }
            if state.tags {
                out.push("Tags");
            }
            if state.metadata {
                out.push("Metadata");
            }
        });

        if out.is_empty() {
            "(no preview)".to_string()
        } else {
            out.join(", ").to_string()
        }
    };

    let activate = move |e: MouseEvent| {
        if e.button() != types::MouseButton::Primary {
            return;
        }
        if active.with(|active| active.is_some()) {
            return;
        }

        let cb = Closure::new(move |e: MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }

            let target = e.target().unwrap();
            if let Some(target) = target.dyn_ref::<web_sys::HtmlElement>() {
                if target.closest(&format!("#{MENU_ID}")).unwrap().is_some() {
                    return;
                }
            } else if let Some(target) = target.dyn_ref::<web_sys::SvgElement>() {
                if target.closest(&format!("#{MENU_ID}")).unwrap().is_some() {
                    return;
                }
            };

            let window = web_sys::window().unwrap();
            active.with_untracked(|active| {
                let cb = active.as_ref().unwrap();
                window
                    .remove_event_listener_with_callback("mousedown", cb.as_ref().unchecked_ref())
                    .unwrap();
            });

            set_active.update(|active| {
                active.take();
            });
        });

        let window = web_sys::window().unwrap();
        window
            .add_event_listener_with_callback("mousedown", cb.as_ref().unchecked_ref())
            .unwrap();

        set_active.update(|active| {
            let _ = active.insert(cb);
        });
    };

    let clear = move |e: MouseEvent| {
        if e.button() != types::MouseButton::Primary {
            return;
        }

        state.update(|state| {
            state.clear();
        })
    };

    const CLASS_FORM_DIV: &str = "px-2 w-full";
    const CLASS_CHECKBOX: &str = "w-4 h-4 rounded";
    const CLASS_LABEL: &str = "pl-2";
    view! {
        <div class="relative z-10">
            <div
                on:mousedown=activate
                class="cursor-pointer inline-flex w-40 px-2 rounded border border-secondary-600 dark:border-secondary-200"
                class=("rounded-b-none", move || active.with(|active| active.is_some()))
            >
                <span class="grow truncate">{preview_list}</span>
                <span class="pl-2 inline-flex items-center">
                    <Icon icon=icondata::FaChevronDownSolid />
                </span>
            </div>
            <div
                id=MENU_ID
                class:hidden=move || active.with(|active| active.is_none())
                class="absolute w-40 rounded-b bg-white dark:bg-secondary-900 border border-t-0 border-secondary-600 dark:border-secondary-200"
            >
                <form on:submit=move |e| e.prevent_default()>
                    <div class=CLASS_FORM_DIV>
                        <label>
                            <input
                                type="checkbox"
                                name="assets"
                                on:input=move |_| {
                                    state.update(|state| state.assets = !state.assets)
                                }

                                prop:checked=move || state.with(|state| state.assets)
                                class=CLASS_CHECKBOX
                            />

                            <span class=CLASS_LABEL>"Data"</span>
                        </label>
                    </div>

                    <div class=CLASS_FORM_DIV>
                        <label>
                            <input
                                type="checkbox"
                                name="analyses"
                                on:input=move |_| {
                                    state.update(|state| state.analyses = !state.analyses)
                                }

                                prop:checked=move || { state.with(|state| state.analyses) }

                                class=CLASS_CHECKBOX
                            />

                            <span class=CLASS_LABEL>"Analyses"</span>
                        </label>
                    </div>

                    <div class=CLASS_FORM_DIV>
                        <label>
                            <input
                                type="checkbox"
                                name="kind"
                                on:input=move |_| { state.update(|state| state.kind = !state.kind) }

                                prop:checked=move || state.with(|state| state.kind)
                                class=CLASS_CHECKBOX
                            />

                            <span class=CLASS_LABEL>"Type"</span>
                        </label>
                    </div>

                    <div class=CLASS_FORM_DIV>
                        <label>
                            <input
                                type="checkbox"
                                name="description"
                                on:input=move |_| {
                                    state.update(|state| state.description = !state.description)
                                }

                                prop:checked=move || { state.with(|state| state.description) }

                                class=CLASS_CHECKBOX
                            />

                            <span class=CLASS_LABEL>"Description"</span>
                        </label>
                    </div>

                    <div class=CLASS_FORM_DIV>
                        <label>
                            <input
                                type="checkbox"
                                name="tags"
                                on:input=move |_| { state.update(|state| state.tags = !state.tags) }

                                prop:checked=move || state.with(|state| state.tags)
                                class=CLASS_CHECKBOX
                            />

                            <span class=CLASS_LABEL>"Tags"</span>
                        </label>
                    </div>

                    <div class=CLASS_FORM_DIV>
                        <label>
                            <input
                                type="checkbox"
                                name="metadata"
                                on:input=move |_| {
                                    state.update(|state| state.metadata = !state.metadata)
                                }

                                prop:checked=move || { state.with(|state| state.metadata) }
                                class=CLASS_CHECKBOX
                            />

                            <span class=CLASS_LABEL>"Metadata"</span>
                        </label>
                    </div>
                    <hr class="border-secondary-900 dark:border-secondary-200" />
                    <div class="px-2 text-center dark:border-secondary-200">
                        <button on:mousedown=clear class="w-full h-full">
                            "Clear"
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}

#[component]
fn Analyze() -> impl IntoView {
    let project = expect_context::<state::Project>();
    let messages = expect_context::<types::Messages>();

    let trigger_analysis = create_action(move |_| analyze(project.rid().get_untracked(), "/"));

    let mousedown = move |e: MouseEvent| {
        if e.button() != types::MouseButton::Primary {
            return;
        }

        trigger_analysis.dispatch(());
    };

    let _ = watch(
        move || trigger_analysis.value().get(),
        move |value, _, _| {
            let Some(value) = value else {
                return;
            };
            if let Err(err) = value {
                let mut msg =
                    crate::components::message::Builder::error("Could not complete analysis.");
                msg.body(err);
                messages.update(|messages| messages.push(msg.build()));
            }
        },
        false,
    );

    view! {
        <button
            on:mousedown=mousedown
            class="btn-primary rounded px-4"
            disabled={
                let pending = trigger_analysis.pending();
                move || pending.get()
            }
        >
            "Analyze"
        </button>
    }
}

async fn analyze(project: ResourceId, root: impl Into<PathBuf>) -> Result<(), String> {
    #[derive(serde::Serialize)]
    struct Args {
        project: ResourceId,
        root: PathBuf,
        max_tasks: Option<usize>,
    }

    tauri_sys::core::invoke_result(
        "analyze_project",
        Args {
            project,
            root: root.into(),
            max_tasks: None,
        },
    )
    .await
}

#[component]
fn ProjectInfo() -> impl IntoView {
    let project = expect_context::<state::Project>();
    let properties_editor = expect_context::<RwSignal<PropertiesEditor>>();

    let mousedown = move |e: MouseEvent| {
        properties_editor.set(properties::EditorKind::Project.into());
    };

    view! {
        <div on:mousedown=mousedown class="grow text-center font-primary cursor-pointer">
            {project.properties().name()}
        </div>
    }
}
