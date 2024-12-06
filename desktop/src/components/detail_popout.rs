use crate::{components, types};
use leptos::{ev::MouseEvent, prelude::*};
use leptos_icons::Icon;

#[component]
pub fn DetailPopout(
    #[prop(into)] title: TextProp,
    #[prop(optional, into)] onclose: Option<Callback<()>>,
    children: Children,
) -> impl IntoView {
    let close = move |e: MouseEvent| {
        if e.button() == types::MouseButton::Primary {
            if let Some(onclose) = onclose {
                onclose(());
            }
        }
    };

    view! {
        <div class="rounded border border-secondary-300 dark:border-secondary-600 bg-white dark:bg-secondary-700 shadow shadow-primary-700 dark:shadow-none">
            <div class="flex p-1 border-b dark:border-secondary-500">
                <span class="grow">{title}</span>
                <span>
                    <button
                        type="button"
                        on:mousedown=close
                        class="hover:bg-secondary-200 dark:hover:bg-secondary-600"
                    >
                        <Icon icon=components::icon::Close />
                    </button>
                </span>
            </div>
            <div class="pt-1">{children()}</div>
        </div>
    }
}
