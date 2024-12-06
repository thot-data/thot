use crate::{
    components::{self, Logo},
    pages::{Dashboard, Settings},
    types,
};
use leptos::*;
use leptos_icons::Icon;
use leptos_router::*;
use syre_core::system::User;
use syre_desktop_lib as lib;

#[derive(Clone, Copy, derive_more::Deref, derive_more::From)]
struct ShowSettings(RwSignal<bool>);
impl ShowSettings {
    pub fn new() -> Self {
        Self(create_rw_signal(false))
    }
}

#[component]
pub fn Home(user: User) -> impl IntoView {
    provide_context(user);
    let user_settings = create_resource(|| (), move |_| fetch_user_settings());

    view! {
        <Suspense fallback=move || {
            view! { <Loading /> }
        }>

            {move || {
                user_settings
                    .map(|user_settings| match user_settings {
                        None => view! { <NoSettings /> },
                        Some(user_settings) => {
                            view! { <HomeView user_settings=user_settings.clone() /> }
                        }
                    })
            }}
        </Suspense>
    }
}

#[component]
fn Loading() -> impl IntoView {
    view! { <div class="text-center pt-4">"Loading home"</div> }
}

#[component]
fn NoSettings() -> impl IntoView {
    let messages = expect_context::<types::Messages>();
    let navigate = leptos_router::use_navigate();

    let msg = types::message::Builder::error("Could not get user settings.");
    let msg = msg.build();
    messages.update(|messages| messages.push(msg));
    navigate("/login", Default::default());

    view! {
        <div class="text-center pt-4">
            <p>"Could not get user settings."</p>
            <p>"Redirecting to login."</p>
        </div>
    }
}

#[component]
fn HomeView(user_settings: lib::settings::User) -> impl IntoView {
    let messages = expect_context::<types::Messages>();
    provide_context(types::settings::User::new(user_settings.clone()));
    let show_settings = ShowSettings::new();
    provide_context(show_settings);

    match (user_settings.desktop, user_settings.runner) {
        (Ok(_), Ok(_)) => {}
        (Err(err), Ok(_)) => {
            let mut msg = types::message::Builder::error("Could not load desktop settings.");
            msg.body(format!("{err:?}"));
            messages.update(|messages| messages.push(msg.build()));
        }
        (Ok(_), Err(err)) => {
            let mut msg = types::message::Builder::error("Could not load runner settings.");
            msg.body(format!("{err:?}"));
            messages.update(|messages| messages.push(msg.build()));
        }
        (Err(err_desktop), Err(err_runner)) => {
            let mut msg = types::message::Builder::error("Could not load settings.");
            msg.body(view! {
                <ul>
                    <li>"Desktop: " {format!("{err_desktop:?}")}</li>
                    <li>"Runner: " {format!("{err_runner:?}")}</li>
                </ul>
            });
            messages.update(|messages| messages.push(msg.build()));
        }
    }

    view! {
        <div class="relative">
            <MainNav />
            <main>
                <div>
                    <Dashboard />
                </div>
                <div
                    class=(["-right-full", "left-full"], move || !show_settings())
                    class=(["right-0", "left-0"], move || show_settings())
                    class="absolute top-0 bottom-0 transition-absolute-position"
                >
                    <Settings onclose=move |_| show_settings.set(false) />
                </div>
            </main>
        </div>
    }
}

#[component]
fn MainNav() -> impl IntoView {
    let show_settings = expect_context::<ShowSettings>();
    let open_settings = move |e: ev::MouseEvent| {
        if e.button() != types::MouseButton::Primary {
            return;
        }

        show_settings.set(true);
    };

    view! {
        <nav class="px-2 border-b dark:bg-secondary-900 flex justify-between">
            <ol>
                <li>
                    <A href="/" class="inline-block align-middle">
                        <Logo class="h-4" />
                    </A>
                </li>
            </ol>

            <ol class="inline-flex gap-2 items-center">
                <li>
                    <button
                        on:mousedown=open_settings
                        type="button"
                        class="align-middle p-1 hover:bg-secondary-100 dark:hover:bg-secondary-800 rounded border border-transparent hover:border-secondary-200 dark:hover:border-white"
                    >
                        <Icon icon=components::icon::Settings />
                    </button>
                </li>
                <li>
                    <A
                        href="/logout"
                        class="inline-block align-middle p-1 hover:bg-secondary-100 dark:hover:bg-secondary-800 rounded border border-transparent hover:border-secondary-200 dark:hover:border-white"
                    >
                        <Icon icon=icondata::IoLogOutOutline class="[&_*]:dark:!stroke-white h-4" />
                    </A>
                </li>
            </ol>
        </nav>
    }
}

async fn fetch_user_settings() -> Option<lib::settings::User> {
    tauri_sys::core::invoke("user_settings", ()).await
}
