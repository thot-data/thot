use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use serde::Serialize;
use syre_local::error::IoSerde;

#[component]
pub fn Logout() -> impl IntoView {
    let status = Resource::new(|| (), |_| async move { logout().await });

    move || {
        status.with(|status| match status {
            None => view! { <Pending /> },
            Some(Ok(_)) => view! { <Redirecting /> },
            Some(Err(err)) => view! { <LogoutErr err=err.clone() /> },
        })
    }
}

#[component]
fn Pending() -> impl IntoView {
    view! { <div>"Logging out..."</div> }
}

#[component]
fn Redirecting() -> impl IntoView {
    let navigate = use_navigate();
    navigate("/", Default::default());

    view! { <div>"Redirecting to home page"</div> }
}

#[component]
fn LogoutErr(err: IoSerde) -> impl IntoView {
    view! {
        <div>
            <h3>"An error ocurred"</h3>
            <div>"You could not be logged out."</div>
            <div>{format!("{err:?}")}</div>
        </div>
    }
}

async fn logout() -> Result<(), IoSerde> {
    tauri_sys::core::invoke_result("logout", ()).await
}
