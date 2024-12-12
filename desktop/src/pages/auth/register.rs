use crate::components::{Autofocus, Logo};
use leptos::prelude::*;
use leptos_router::{components::A, hooks::use_navigate};
use serde::Serialize;
use syre_core::system::User;
use web_sys::{FormData, SubmitEvent};

#[component]
pub fn Register() -> impl IntoView {
    let navigate = use_navigate();
    let (error, set_error) = signal(None);
    let form_ref = NodeRef::new();

    let register_user_action: Action<_, _> =
        Action::new_unsync(move |(email, name): &(String, Option<String>)| {
            let email = email.clone();
            let name = name.clone();
            let navigate = navigate.clone();
            async move {
                match register(email, name).await {
                    Ok(_user) => {
                        navigate("/", Default::default());
                    }

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
        <div class="h-screen w-screen flex flex-col justify-center items-center gap-y-4">
            <div class="flex flex-col items-center w-20">
                <Logo attr:class="w-full" />
                <h1 class="font-primary text-4xl">"Syre"</h1>
            </div>
            <div class="w-1/2">
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
                    <div class="pt-4 flex justify-center gap-x-4">
                        <button disabled=register_user_action.pending() class="btn btn-primary">
                            "Sign up"
                        </button>
                        <A href="/login" attr:class="btn btn-secondary">
                            "Log in"
                        </A>
                    </div>
                    <div>{error}</div>
                </form>
            </div>
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
