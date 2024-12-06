use crate::types::MouseButton;
use leptos::{ev::MouseEvent, *};
use wasm_bindgen::{prelude::Closure, JsCast};

/// Which side the drawer is docked on.
#[derive(Copy, Clone)]
pub enum Dock {
    East,
    West,
}

/// A drawer that can be resized.
#[component]
pub fn Drawer(
    #[prop(into)] dock: MaybeSignal<Dock>,
    #[prop(into)] class: MaybeSignal<String>,

    /// `absolute`ly position the drawer, if `true`,
    /// otherwise `relative`ly position it.
    #[prop(into, optional)]
    absolute: MaybeSignal<bool>,
    children: Children,
) -> impl IntoView {
    let root_node = NodeRef::<html::Div>::new();
    let classes = move || {
        let mut classes = class.get();
        if absolute() {
            match dock() {
                Dock::East => classes.push_str(" absolute left-0 top-0 bottom-0"), // NB: prefixed space needed for concat
                Dock::West => classes.push_str(" absolute right-0 top-0 bottom-0 "), // NB: prefixed space needed for concat
            }
        } else {
            classes.push_str(" relative"); // NB: prefixed space needed for concat
        }
        classes
    };

    let drawer_class = move || {
        dock.with(|dock| match dock {
            Dock::East => "absolute left-full -right-0.5 hover:-right-1 transition-width hover:delay-150 hover:duration-200 top-0 bottom-0 cursor-ew-resize hover:bg-primary-700",
            Dock::West => "absolute -left-0.5 right-full hover:-left-1 transition-width hover:delay-150 hover:duration-200 top-0 bottom-0 cursor-ew-resize hover:bg-primary-700",
        })
    };

    let resize_start = move |e: MouseEvent| {
        if e.button() != MouseButton::Primary {
            return;
        }

        e.stop_propagation();
        let resize_cb = Closure::<dyn Fn(MouseEvent)>::new({
            move |e: MouseEvent| {
                let root_node = root_node.get_untracked().unwrap();
                dock.with(|dock| match dock {
                    Dock::East => {
                        let root_bb = root_node.get_bounding_client_rect();
                        let root_base = root_bb.x();
                        let width = e.x() - root_base as i32;
                        // root_node.style("width", &format!("{width}px"));
                        (*root_node)
                            .style()
                            .set_property("width", &format!("{width}px"))
                            .unwrap();
                    }

                    Dock::West => {
                        let root_bb = root_node.get_bounding_client_rect();
                        let root_base = root_bb.x() + root_bb.width();
                        let width = root_base as i32 - e.x();
                        // root_node.style("width", &format!("{width}px"));
                        (*root_node)
                            .style()
                            .set_property("width", &format!("{width}px"))
                            .unwrap();
                    }
                });
            }
        });

        let document = web_sys::window().unwrap().document().unwrap();
        document
            .add_event_listener_with_callback("mousemove", resize_cb.as_ref().unchecked_ref())
            .unwrap();

        let resize_end = Closure::<dyn Fn(MouseEvent)>::new({
            let document = document.clone();
            move |_e: MouseEvent| {
                document
                    .remove_event_listener_with_callback(
                        "mousemove",
                        resize_cb.as_ref().unchecked_ref(),
                    )
                    .unwrap();
            }
        });

        document
            .add_event_listener_with_callback("mouseup", resize_end.as_ref().unchecked_ref())
            .unwrap();

        resize_end.forget();
    };

    view! {
        <div ref=root_node class=classes>
            <div on:mousedown=resize_start class=drawer_class></div>
            {children()}
        </div>
    }
}
