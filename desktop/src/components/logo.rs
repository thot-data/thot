use crate::app::PrefersDarkTheme;
use leptos::prelude::*;

#[component]
pub fn Logo() -> impl IntoView {
    let prefers_dark = expect_context::<PrefersDarkTheme>();
    let home_icon_src = move || {
        if prefers_dark() {
            "/public/logos/logo-white-icon.svg"
        } else {
            "/public/logos/logo-black-icon.svg"
        }
    };

    view! { <img src=home_icon_src /> }
}
