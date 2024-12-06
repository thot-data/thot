use super::InputDebounce;
use crate::{
    components::{ModalDialog, TruncateLeft},
    pages::project::state,
    types::{self},
};
use description::Editor as Description;
use leptos::{
    ev::{MouseEvent, SubmitEvent},
    html,
    prelude::*,
};
use name::Editor as Name;
use serde::Serialize;
use std::{io, path::PathBuf};
use syre_core as core;
use syre_desktop_lib as lib;
use syre_local as local;

#[component]
pub fn Editor() -> impl IntoView {
    let project = expect_context::<state::Project>();
    let delete_project_modal = NodeRef::<html::Dialog>::new();

    let project_path_str = {
        let project = project.clone();
        move || {
            project
                .path()
                .with(|path| path.to_string_lossy().to_string())
        }
    };

    let show_delete_confirmation = move |e: MouseEvent| {
        if e.button() != types::MouseButton::Primary {
            return;
        }

        spawn_local(async move {
            let dialog = delete_project_modal.get_untracked().unwrap();
            dialog.show_modal().unwrap();
        });
    };

    view! {
        <div class="overflow-y-auto pr-2 pb-4 h-full flex flex-col">
            <div class="pb-8">
                <div class="text-center pt-1 pb-2">
                    <h3 class="font-primary">"Project"</h3>
                </div>
                <div>
                    <form on:submit=|e| e.prevent_default()>
                        <div class="pb-1 px-1">
                            <label>
                                <span class="block">"Name"</span>
                                <Name />
                            </label>
                        </div>
                        <div class="pb-1 px-1">
                            <label>
                                <span class="block">"Description"</span>
                                <Description />
                            </label>
                        </div>
                    </form>
                    <div title=project_path_str.clone()>
                        <TruncateLeft>{project_path_str.clone()}</TruncateLeft>
                    </div>
                </div>
            </div>
            <div class="p-2 text-center">
                <button
                    on:mousedown=show_delete_confirmation
                    class="btn bg-syre-red-700 dark:bg-syre-red-500 w-full"
                >
                    "Delete"
                </button>
                <ModalDialog node_ref=delete_project_modal>
                    <DeleteProjectConfirmation />
                </ModalDialog>
            </div>
        </div>
    }
}

#[component]
fn DeleteProjectConfirmation() -> impl IntoView {
    let project = expect_context::<state::Project>();
    let messages = expect_context::<types::Messages>();
    let navigate = leptos_router::use_navigate();
    let (confirmation_text, set_confirmation_text) = signal("".to_string());

    let confirmation_valid = {
        let project = project.clone();
        move || {
            confirmation_text.with(|confirmation_text| {
                project
                    .properties()
                    .name()
                    .with(|name| confirmation_text == name)
            })
        }
    };

    let delete_project_action = {
        let project = project.clone();
        let messages = messages.clone();
        let navigate = navigate.clone();
        let confirmation_valid = confirmation_valid.clone();
        move |e: SubmitEvent| {
            e.prevent_default();
            if !confirmation_valid() {
                return;
            }

            spawn_local({
                let project = project.path().read_only();
                let messages = messages.clone();
                let navigate = navigate.clone();
                async move {
                    if let Err(err) = delete_project(project.get_untracked()).await {
                        let mut msg = types::message::Builder::error("Could not delete project.");
                        msg.body(format!("{err:?}"));
                        messages.update(|messages| messages.push(msg.build()));
                    } else {
                        navigate("/", Default::default());
                    }
                }
            });
        }
    };

    view! {
        <div class="bg-white border border-black rounded dark:bg-secondary-800 dark:border-secondary-400 dark:text-white px-4 py-2">
            <div class="text-2xl pb-2">"Are you sure you want to delete this project?"</div>
            <div class="pb-2">
                <form on:submit=delete_project_action>
                    <div class="pb-4">
                        <label>
                            <div class="pb-2">
                                "Type \""<strong>{project.properties().name().read_only()}</strong>
                                "\" to confirm"
                            </div>
                            <div>
                                <input
                                    prop:value=confirmation_text
                                    on:input=move |e| set_confirmation_text(event_target_value(&e))
                                    type="text"
                                    class="text-black"
                                    required
                                />
                            </div>
                        </label>
                    </div>
                    <div class="flex gap-4 justify-center">
                        <button
                            disabled=move || !confirmation_valid()
                            class="btn bg;syre-red-700 dark:bg-syre-red-500 disabled:btn-secondary disabled:cursor-not-allowed"
                        >
                            "Confirm"
                        </button>
                        <button type="button" class="btn btn-secondary">
                            "Cancel"
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}

mod name {
    use super::{update_properties, InputDebounce};
    use crate::{components::form::debounced::InputText, pages::project::state, types};
    use leptos::prelude::*;

    #[component]
    pub fn Editor() -> impl IntoView {
        let project = expect_context::<state::Project>();
        let messages = expect_context::<types::Messages>();
        let input_debounce = expect_context::<InputDebounce>();

        let oninput = {
            let project = project.clone();
            move |value: String| {
                let value = value.trim();
                if value.is_empty() {
                    return;
                }

                let mut properties = project.as_properties();
                properties.name = value.to_string();

                spawn_local({
                    let messages = messages.write_only();
                    async move {
                        if let Err(err) = update_properties(properties).await {
                            tracing::error!(?err);
                            let mut msg = types::message::Builder::error("Could not save project.");
                            msg.body(format!("{err:?}"));
                            messages.update(|messages| messages.push(msg.build()));
                        }
                    }
                });
            }
        };

        view! {
            <InputText
                value=project.properties().name().read_only()
                oninput=Callback::new(oninput)
                debounce=*input_debounce
                class="input-compact w-full"
            />
        }
    }
}

mod description {
    use super::{
        super::common::description::Editor as DescriptionEditor, update_properties, InputDebounce,
    };
    use crate::{pages::project::state, types};
    use leptos::prelude::*;

    #[component]
    pub fn Editor() -> impl IntoView {
        let project = expect_context::<state::Project>();
        let messages = expect_context::<types::Messages>();
        let input_debounce = expect_context::<InputDebounce>();

        let oninput = {
            let project = project.clone();
            let messages = messages.write_only();
            move |value: Option<String>| {
                let mut properties = project.as_properties();
                properties.description = value;

                spawn_local({
                    let messages = messages.clone();
                    async move {
                        if let Err(err) = update_properties(properties).await {
                            tracing::error!(?err);
                            let mut msg = types::message::Builder::error("Could not save project.");
                            msg.body(format!("{err:?}"));
                            messages.update(|messages| messages.push(msg.build()));
                        }
                    }
                });
            }
        };

        view! {
            <DescriptionEditor
                value=project.properties().description().read_only()
                oninput=Callback::new(oninput)
                debounce=*input_debounce
                class="input-compact w-full align-top"
            />
        }
    }
}

async fn update_properties(update: core::project::Project) -> Result<(), local::error::IoSerde> {
    #[derive(Serialize)]
    struct Args {
        update: core::project::Project,
    }

    tauri_sys::core::invoke_result("project_properties_update", Args { update }).await
}

async fn delete_project(project: PathBuf) -> Result<(), lib::command::error::Trash> {
    #[derive(Serialize)]
    struct Args {
        project: PathBuf,
    }

    tauri_sys::core::invoke_result("delete_project", Args { project }).await
}
