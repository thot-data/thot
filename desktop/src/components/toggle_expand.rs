use crate::{components, types};
use leptos::{ev::MouseEvent, *};
use leptos_icons::Icon;

#[component]
pub fn ToggleExpand(expanded: RwSignal<bool>) -> impl IntoView {
    let toggle = move |e: MouseEvent| {
        if e.button() != types::MouseButton::Primary {
            return;
        }

        e.stop_propagation();
        expanded.set(!expanded());
    };

    // TODO: Center icon vertically.
    view! {
        <button on:mousedown=toggle type="button">
            <span class=("rotate-90", expanded) class="inline-block transition">
                <Icon icon=components::icon::ChevronRight />
            </span>
        </button>
    }
}
