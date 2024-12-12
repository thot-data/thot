use leptos::{either::either, prelude::*};
use leptos_router::components::Redirect;
use syre_local::error::IoSerde;

#[component]
pub fn Logout() -> impl IntoView {
    let status = LocalResource::new(logout);
    view! {
        <Suspense fallback=Pending>
            {move || Suspend::new(async move {
                either!(
                    status.await,
                    Ok(_) => view! { <RedirectHome /> },
                    Err(err) => view! { <LogoutErr err=err.clone() /> },
                )
            })}
        </Suspense>
    }
}

#[component]
fn Pending() -> impl IntoView {
    view! { <div>"Logging out..."</div> }
}

#[component]
fn RedirectHome() -> impl IntoView {
    view! {
        <div>"Redirecting to home page"</div>
        <Redirect path="/" />
    }
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
