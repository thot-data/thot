use crate::{components::icon, types};
use leptos::{ev::MouseEvent, *};
use leptos_icons::*;
use syre_desktop_lib as lib;

#[component]
pub fn Settings(
    /// Called when the user requests to close the page.
    #[prop(into)]
    onclose: Callback<()>,
) -> impl IntoView {
    let user_settings = expect_context::<types::settings::User>();
    let settings = create_resource(|| (), |_| async move { fetch_user_settings().await });
    view! {
        <Suspense fallback=Loading>
            {settings()
                .map(|settings| {
                    if let Some(settings) = settings {
                        user_settings.set(settings);
                    }
                    view! { <SettingsView onclose /> }
                })}
        </Suspense>
    }
}

#[component]
fn Loading() -> impl IntoView {
    view! { <div class="text-center">"Loading"</div> }
}

#[component]
fn SettingsView(
    /// Called when the user requests to close the page.
    #[prop(into)]
    onclose: Callback<()>,
) -> impl IntoView {
    let trigger_close = move |e: MouseEvent| {
        if e.button() == types::MouseButton::Primary {
            onclose(());
        }
    };

    view! {
        <div class="relative bg-white dark:bg-secondary-800 dark:text-white h-full w-full">
            <div>
                <button
                    on:mousedown=trigger_close
                    type="button"
                    class="absolute top-2 right-2 rounded hover:bg-secondary-100 dark:hover:bg-secondary-700"
                >
                    <Icon icon=icon::Close />
                </button>
            </div>
            <h1 class="text-lg font-primary pt-2 pb-4 px-2">"Settings"</h1>
            <div class="px-2 pb-4">
                <h2 class="text-md font-primary pb-2">"Desktop"</h2>
                <desktop::Settings />
            </div>
            <div class="px-2">
                <h2 class="text-md font-primary pb-2">"Runner"</h2>
                <runner::Settings />
            </div>
        </div>
    }
}

mod desktop {
    use crate::{app::PrefersDarkTheme, types};
    use leptos::{
        ev::{Event, MouseEvent},
        *,
    };
    use leptos_icons::*;
    use serde::{Deserialize, Serialize};
    use std::io;
    use syre_core::{self as core, types::ResourceId};
    use syre_desktop_lib as lib;
    use syre_local::error::IoSerde;

    #[component]
    pub fn Settings() -> impl IntoView {
        let user = expect_context::<core::system::User>();
        let messages = expect_context::<types::Messages>();
        let prefers_dark_theme = expect_context::<PrefersDarkTheme>();
        let user_settings = expect_context::<types::settings::User>();
        let (input_debounce, set_input_debounce) =
            create_signal(user_settings.with_untracked(|settings| {
                settings
                    .desktop
                    .clone()
                    .unwrap_or_default()
                    .input_debounce_ms
            }));
        let input_debounce = leptos_use::signal_debounced(
            input_debounce,
            Signal::derive(move || input_debounce.with(|ms| *ms as f64)),
        );

        let _ = {
            let user = user.rid().clone();
            watch(
                input_debounce,
                move |input_debounce, _, _| {
                    let update = user_settings.with_untracked(|settings| match &settings.desktop {
                        Ok(settings) => Ok(settings.clone()),
                        Err(err) if matches!(err, IoSerde::Io(io::ErrorKind::NotFound)) => {
                            Ok(lib::settings::Desktop::default())
                        }
                        Err(err) => Err(err.clone()),
                    });

                    let mut update = match update {
                        Ok(update) => update,
                        Err(err) => {
                            let mut msg =
                                types::message::Builder::error("Can not update settings.");
                            msg.body(format!("{err:?}"));
                            messages.update(|messages| messages.push(msg.build()));
                            return;
                        }
                    };

                    update.input_debounce_ms = *input_debounce;
                    user_settings.update(|settings| {
                        settings.desktop = Ok(update.clone());
                    });

                    let user = user.clone();
                    spawn_local(async move {
                        if let Err(err) = update_settings(user, update).await {
                            let mut msg =
                                types::message::Builder::error("Could not update settings.");
                            msg.body(format!("{err:?}"));
                            messages.update(|messages| messages.push(msg.build()));
                        }
                    });
                },
                false,
            )
        };

        let toggle_theme = move |e: MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }

            prefers_dark_theme.set(!prefers_dark_theme());
        };

        let update_input_debounce = move |e: Event| {
            let value = event_target_value(&e);
            if let Ok(value) = value.parse::<usize>() {
                set_input_debounce(value);
            }
        };

        view! {
            <form on:submit=move |e| e.prevent_default()>
                <div class="pb-2">
                    <label>
                        {move || {
                            if prefers_dark_theme() {
                                view! {
                                    <button
                                        type="button"
                                        on:mousedown=toggle_theme
                                        class="text-2xl p-2 border rounded"
                                        title="Light mode"
                                    >
                                        <Icon icon=icondata::BsSun />
                                    </button>
                                }
                            } else {
                                view! {
                                    <button
                                        type="button"
                                        on:mousedown=toggle_theme
                                        class="text-2xl p-2 border border-black rounded"
                                        title="Dark mode"
                                    >
                                        <Icon icon=icondata::BsMoon />
                                    </button>
                                }
                            }
                        }}
                    </label>
                </div>
                <div>
                    <label>
                        "Input debounce"
                        <input
                            type="number"
                            min="250"
                            max="1000"
                            step="50"
                            prop:value=input_debounce
                            on:input=update_input_debounce
                            class="input-simple"
                        /> <small>"250 - 1000 ms"</small>
                    </label>
                </div>
            </form>
        }
    }

    async fn update_settings(
        user: ResourceId,
        update: lib::settings::Desktop,
    ) -> Result<(), lib::command::error::IoErrorKind> {
        #[derive(Serialize, Deserialize)]
        struct Args {
            user: ResourceId,
            update: lib::settings::Desktop,
        }

        tauri_sys::core::invoke_result("user_settings_desktop_update", Args { user, update }).await
    }
}

mod runner {
    use crate::{commands, types};
    use leptos::{
        ev::{Event, MouseEvent},
        *,
    };
    use leptos_icons::*;
    use serde::{Deserialize, Serialize};
    use std::{io, path::PathBuf};
    use syre_core::{self as core, types::ResourceId};
    use syre_desktop_lib as lib;
    use syre_local::error::IoSerde;

    #[component]
    pub fn Settings() -> impl IntoView {
        let user = expect_context::<core::system::User>();
        let messages = expect_context::<types::Messages>();
        let user_settings = expect_context::<types::settings::User>();
        let input_debounce = move || {
            user_settings.with(|settings| {
                let debounce = match &settings.desktop {
                    Ok(settings) => settings.input_debounce_ms,
                    Err(_) => lib::settings::Desktop::default().input_debounce_ms,
                };

                debounce as f64
            })
        };

        let (python_path, set_python_path) =
            create_signal(user_settings.with_untracked(|settings| {
                settings
                    .runner
                    .as_ref()
                    .map(|settings| settings.python_path.as_ref())
                    .ok()
                    .flatten()
                    .cloned()
            }));
        let python_path =
            leptos_use::signal_debounced(python_path, Signal::derive(input_debounce.clone()));

        let (r_path, set_r_path) = create_signal(user_settings.with_untracked(|settings| {
            settings
                .runner
                .as_ref()
                .map(|settings| settings.r_path.as_ref())
                .ok()
                .flatten()
                .cloned()
        }));
        let r_path = leptos_use::signal_debounced(r_path, Signal::derive(input_debounce.clone()));

        let (continue_on_error, set_continue_on_error) =
            create_signal(user_settings.with_untracked(|settings| {
                settings
                    .runner
                    .as_ref()
                    .map(|settings| settings.continue_on_error)
                    .unwrap_or(false)
            }));
        let continue_on_error =
            leptos_use::signal_debounced(continue_on_error, Signal::derive(input_debounce.clone()));

        let _ = {
            let user = user.rid().clone();
            watch(
                move || (python_path.get(), r_path.get(), continue_on_error.get()),
                move |(python_path, r_path, continue_on_error), _, _| {
                    let update = user_settings.with_untracked(|settings| match &settings.runner {
                        Ok(settings) => Ok(settings.clone()),
                        Err(err) if matches!(err, IoSerde::Io(io::ErrorKind::NotFound)) => {
                            Ok(lib::settings::Runner::default())
                        }
                        Err(err) => Err(err.clone()),
                    });

                    let mut update = match update {
                        Ok(update) => update,
                        Err(err) => {
                            let mut msg =
                                types::message::Builder::error("Can not update settings.");
                            msg.body(format!("{err:?}"));
                            messages.update(|messages| messages.push(msg.build()));
                            return;
                        }
                    };

                    update.python_path = python_path.clone();
                    update.r_path = r_path.clone();
                    update.continue_on_error = *continue_on_error;
                    user_settings.update(|settings| {
                        settings.runner = Ok(update.clone());
                    });
                    let user = user.clone();
                    spawn_local(async move {
                        if let Err(err) = update_settings(user, update).await {
                            let mut msg =
                                types::message::Builder::error("Could not update settings.");
                            msg.body(format!("{err:?}"));
                            messages.update(|messages| messages.push(msg.build()));
                        }
                    });
                },
                false,
            )
        };

        view! {
            <form on:submit=move |e| e.prevent_default()>
                <div class="pb-2">
                    <PythonPath value=python_path set_value=set_python_path />
                </div>
                <div class="pb-2">
                    <RPath value=r_path set_value=set_r_path />
                </div>
                <div class="pb-2">
                    <ContinueOnError value=continue_on_error set_value=set_continue_on_error />
                </div>
            </form>
        }
    }

    #[component]
    fn PythonPath(
        value: Signal<Option<PathBuf>>,
        set_value: WriteSignal<Option<PathBuf>>,
    ) -> impl IntoView {
        let update_path = move |e: Event| {
            let value = event_target_value(&e);
            let value = value.trim();
            if value.is_empty() {
                set_value.update(|path| {
                    let _ = path.take();
                });
            } else {
                set_value.update(|path| {
                    let _ = path.insert(PathBuf::from(value));
                });
            }
        };

        let select_path = move |e: MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }

            spawn_local(async move {
                let init_dir = value.with_untracked(|path| match path {
                    None => PathBuf::new(),
                    Some(path) => path
                        .parent()
                        .map(|path| path.to_path_buf())
                        .unwrap_or(PathBuf::new()),
                });

                if let Some(p) =
                    commands::fs::pick_file_with_location("Python path", init_dir).await
                {
                    set_value.update(|path| {
                        let _ = path.insert(p);
                    });
                }
            });
        };

        view! {
            <label class="flex gap-2 items-center">
                <span>
                    <Icon icon=icondata::FaPythonBrands />
                </span>
                <span class="text-nowrap">"Python path"</span>
                <button
                    type="button"
                    on:mousedown=select_path
                    class="aspect-square p-1 rounded-sm border border-black dark:border-white"
                >
                    <Icon icon=icondata::FaFolderOpenRegular />
                </button>
                <input
                    type="text"
                    prop:value=move || {
                        value
                            .with(|path| {
                                path.as_ref()
                                    .map(|path| path.to_string_lossy().to_string())
                                    .unwrap_or("".to_string())
                            })
                    }
                    on:input=update_path
                    class="input-simple grow"
                    placeholder="Path to Python executable"
                />
            </label>
        }
    }

    #[component]
    fn RPath(
        value: Signal<Option<PathBuf>>,
        set_value: WriteSignal<Option<PathBuf>>,
    ) -> impl IntoView {
        let update_path = move |e: Event| {
            let value = event_target_value(&e);
            let value = value.trim();
            if value.is_empty() {
                set_value.update(|path| {
                    let _ = path.take();
                });
            } else {
                set_value.update(|path| {
                    let _ = path.insert(PathBuf::from(value));
                });
            }
        };

        let select_path = move |e: MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }

            spawn_local(async move {
                let init_dir = value.with_untracked(|path| match path {
                    None => PathBuf::new(),
                    Some(path) => path
                        .parent()
                        .map(|path| path.to_path_buf())
                        .unwrap_or(PathBuf::new()),
                });

                if let Some(p) = commands::fs::pick_file_with_location("R path", init_dir).await {
                    set_value.update(|path| {
                        let _ = path.insert(p);
                    });
                }
            });
        };

        view! {
            <label class="flex gap-2 items-center">
                <span>
                    <Icon icon=icondata::FaRProjectBrands />
                </span>
                <span class="text-nowrap">"R path"</span>
                <button
                    type="button"
                    on:mousedown=select_path
                    class="aspect-square p-1 rounded-sm border border-black dark:border-white"
                >
                    <Icon icon=icondata::FaFolderOpenRegular />
                </button>
                <input
                    type="text"
                    prop:value=move || {
                        value
                            .with(|path| {
                                path.as_ref()
                                    .map(|path| path.to_string_lossy().to_string())
                                    .unwrap_or("".to_string())
                            })
                    }
                    on:input=update_path
                    class="input-simple grow"
                    placeholder="Path to R executable"
                />
            </label>
        }
    }

    #[component]
    fn ContinueOnError(value: Signal<bool>, set_value: WriteSignal<bool>) -> impl IntoView {
        view! {
            <label class="flex gap-2 items-center">
                <input
                    type="checkbox"
                    prop:checked=value
                    on:input=move |e| set_value(event_target_checked(&e))
                    class="input-simple"
                />
                "Continue analysis on error."
            </label>
        }
    }

    async fn update_settings(
        user: ResourceId,
        update: lib::settings::Runner,
    ) -> Result<(), lib::command::error::IoErrorKind> {
        #[derive(Serialize, Deserialize)]
        struct Args {
            user: ResourceId,
            update: lib::settings::Runner,
        }

        tauri_sys::core::invoke_result("user_settings_runner_update", Args { user, update }).await
    }
}

async fn fetch_user_settings() -> Option<lib::settings::User> {
    tauri_sys::core::invoke("user_settings", ()).await
}
