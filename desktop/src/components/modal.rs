use leptos::{html, prelude::*};
use wasm_bindgen::JsCast;
use web_sys::{EventTarget, HtmlDialogElement, MouseEvent};

#[component]
pub fn ModalDialog(
    #[prop(optional)] node_ref: NodeRef<html::Dialog>,
    children: Children,
) -> impl IntoView {
    let close_dialog = move |e: MouseEvent| {
        let dialog = NodeRef::<html::Dialog>::new();
        let target = e.target().unwrap();
        let Ok(target): Result<HtmlDialogElement, EventTarget> = target.dyn_into() else {
            return;
        };

        if let Some(dialog) = dialog.get() {
            if target == *dialog {
                dialog.close();
            }
        }
    };

    view! {
        <dialog
            node_ref=node_ref
            on:mousedown=close_dialog
            class="bg-transparent dark:backdrop:bg-black dark:backdrop:opacity-50"
        >
            {children()}
        </dialog>
    }
}
