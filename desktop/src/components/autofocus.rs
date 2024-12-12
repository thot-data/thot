use leptos::{attr::Attribute, children::TypedChildren, html, prelude::*};

/// Autofocus the child element.
///
/// # Notes
/// + Undefined behavior if the child element is already bound to a NodeRef.
#[component]
pub fn Autofocus<At>(
    children: TypedChildren<html::HtmlElement<html::Input, At, ()>>,
) -> impl IntoView
where
    At: Attribute,
{
    let node_ref = NodeRef::new();
    let child = children.into_inner();
    let child = child().into_inner().node_ref(node_ref);

    Effect::new(move |_| {
        if let Some(node) = node_ref.get() {
            node.focus().unwrap();
        }
    });

    child
}
