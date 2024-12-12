use super::{Home, Landing};
use crate::commands;
use futures::StreamExt;
use leptos::{prelude::*, task::spawn_local};
use syre_core as core;
use syre_desktop_lib as lib;

#[component]
pub fn Index() -> impl IntoView {
    let active_user = LocalResource::new(commands::user::fetch_user);

    view! {
        <Suspense fallback=Initializing>
            <ErrorBoundary fallback=|errors| {
                view! { <ActiveUserErrors errors /> }
            }>
                {move || Suspend::new(async move {
                    active_user.await.map(|user| view! { <IndexView user /> })
                })}
            </ErrorBoundary>
        </Suspense>
    }
}

#[component]
fn ActiveUserErrors(errors: ArcRwSignal<Errors>) -> impl IntoView {
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
