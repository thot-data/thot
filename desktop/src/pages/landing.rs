use crate::components::{Autofocus, Logo};
use leptos::*;
use leptos_router::*;
use serde::Serialize;
use std::io;
use syre_core::system::User;
use syre_local::error::IoSerde;
use web_sys::{FormData, SubmitEvent};

#[component]
pub fn Landing() -> impl IntoView {
    let user_count = create_resource(|| (), |_| async move { fetch_user_count().await });
    let fallback = move |errors: RwSignal<Errors>| {
        errors.with(|errors| {
            let errors = errors
                .iter()
                .map(|(_, error)| (*error).clone())
                .collect::<Vec<_>>();

            let [error] = &errors[..] else {
                panic!("invalid errors");
            };

            view! {
                <Show
                    when={
                        let error = error.clone();
                        move || {
                            matches!(
                                error.downcast_ref::<IoSerde>().unwrap(),
                                IoSerde::Io(io::ErrorKind::NotFound)
                            )
                        }
                    }

                    fallback=|| view! { <div>"The user manifest is corrupt."</div> }
                >
                    <Register />
                </Show>
            }
        })
    };

    view! {
        <div class="h-screen w-screen flex flex-col justify-center items-center gap-y-4">
            <div class="flex flex-col items-center w-20">
                <Logo class="w-full" />
                <h1 class="font-primary text-4xl">"Syre"</h1>
            </div>
            <div>
                <Suspense fallback=Loading>
                    <ErrorBoundary fallback>
                        {move || {
                            user_count
                                .get()
                                .map(|count| {
                                    count
                                        .map(|count| {
                                            view! {
                                                <Show
                                                    when=move || { count > 0 }
                                                    fallback=|| view! { <Register /> }
                                                >
                                                    <div class="flex gap-x-4">
                                                        <A href="/register" class="btn btn-primary">
                                                            "Sign up"
                                                        </A>
                                                        <A href="/login" class="btn btn-secondary">
                                                            "Log in"
                                                        </A>
                                                    </div>
                                                </Show>
                                            }
                                        })
                                })
                        }}

                    </ErrorBoundary>
                </Suspense>
            </div>
        </div>
    }
}

#[component]
fn Loading() -> impl IntoView {
    view! { <div>"Loading users..."</div> }
}

#[component]
pub fn Register() -> impl IntoView {
    let (error, set_error) = create_signal(None);
    let form_ref = NodeRef::new();

    let register_user_action = create_action(move |(email, name): &(String, Option<String>)| {
        let email = email.clone();
        let name = name.clone();
        async move {
            match register(email, name).await {
                Ok(_user) => {}

                Err(err) => {
                    set_error(Some(err));
                }
            }
        }
    });

    let register_user = {
        move |e: SubmitEvent| {
            e.prevent_default();
            set_error(None);

            let data = FormData::new_with_form(&form_ref.get().unwrap()).unwrap();
            let name = data.get("name").as_string().unwrap();
            let email = data.get("email").as_string().unwrap();

            let name = if name.trim().is_empty() {
                None
            } else {
                Some(name.trim().to_string())
            };

            if email.trim().is_empty() {
                set_error(Some("Email is required.".to_string()));
                return;
            }
            let email = email.trim().to_string();

            register_user_action.dispatch((email, name));
        }
    };

    view! {
        <div>
            <form node_ref=form_ref on:submit=register_user>
                <div>
                    <label>
                        <span class="block">"Email"</span>
                        <Autofocus>
                            <input
                                name="email"
                                type="email"
                                class="input-simple"
                                required
                                autofocus
                            />
                        </Autofocus>
                    </label>
                </div>
                <div class="pt-4">
                    <label>
                        <span class="block">"Name"</span>
                        <input name="name" class="input-simple" />
                    </label>
                </div>
                <div class="pt-4 text-center">
                    <button disabled=register_user_action.pending() class="btn btn-primary">
                        "Sign up"
                    </button>
                </div>
            </form>
            <div class="pt-2">{error}</div>
        </div>
    }
}

async fn register(email: String, name: Option<String>) -> Result<User, String> {
    #[derive(Serialize)]
    struct Args {
        email: String,
        name: Option<String>,
    }

    tauri_sys::core::invoke_result("register_user", Args { email, name })
        .await
        .map_err(|err| match err {
            syre_local::Error::IoSerde(err) => {
                tracing::debug!(?err);
                "Could not load user manifest.".to_string()
            }
            syre_local::Error::Users(syre_local::error::Users::InvalidEmail(_)) => {
                "Invalid email.".to_string()
            }
            syre_local::Error::Users(syre_local::error::Users::DuplicateEmail(_)) => {
                "Email is already registered.".to_string()
            }
            err => {
                tracing::debug!(?err);
                "Could not create user.".to_string()
            }
        })
}

async fn fetch_user_count() -> Result<usize, IoSerde> {
    tauri_sys::core::invoke_result("user_count", ()).await
}
