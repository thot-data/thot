use super::{Home, Landing};
use crate::{commands, types::Messages};
use futures::StreamExt;
use leptos::{prelude::*, task::spawn_local};
use syre_core as core;
use syre_desktop_lib as lib;

#[component]
pub fn Index() -> impl IntoView {
    let active_user = Resource::new(|| (), |_| async move { commands::user::fetch_user().await });

    view! {
        <Suspense fallback=Initializing>
            <ErrorBoundary fallback=|errors| {
                view! { <ActiveUserErrors errors /> }
            }>
                {move || {
                    active_user().map(|user| user.map(|user| view! { <IndexView user /> }))
                }}
            </ErrorBoundary>
        </Suspense>
    }
}

#[component]
fn ActiveUserErrors(errors: RwSignal<Errors>) -> impl IntoView {
    view! {
        <div class="text-center">
            <div class="text-lg p-4">"An error occurred."</div>
            <div>{format!("{errors:?}")}</div>
        </div>
    }
}

#[component]
fn IndexView(user: Option<core::system::User>) -> impl IntoView {
    let (user, set_user) = signal(user);
    spawn_local(async move {
        let mut listener = tauri_sys::event::listen::<Vec<lib::Event>>(lib::event::topic::USER)
            .await
            .unwrap();

        while let Some(events) = listener.next().await {
            tracing::debug!(?events);
            for event in events.payload {
                let lib::EventKind::User(user) = event.kind() else {
                    panic!("invalid event kind");
                };

                set_user(user.clone());
            }
        }
    });

    view! {
        <Show when=move || { user.with(|user| user.is_some()) } fallback=|| view! { <Landing /> }>
            <Home user=user().unwrap() />
        </Show>
    }
}

#[component]
fn Initializing() -> impl IntoView {
    view! { <div class="text-center pt-4">"Initializing app"</div> }
}
