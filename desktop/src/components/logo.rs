use leptos::*;

#[component]
pub fn Logo(#[prop(into, optional)] class: MaybeProp<TextProp>) -> impl IntoView {
    let prefers_dark = leptos_use::use_preferred_dark();
    let home_icon_src = move || {
        if prefers_dark() {
            "/public/logos/logo-white-icon.svg"
        } else {
            "/public/logos/logo-black-icon.svg"
        }
    };

    view! { <img src=home_icon_src class=class.get()/> }
}
