use leptos::*;

/// Autofocuses the given element.
///
/// # Panics
/// If more than one child element is provided.
///
/// # Notes
/// + Undefined behavior if the child element is already bound to a NodeRef.
#[component]
pub fn Autofocus(children: Children) -> impl IntoView {
    let mut children = children().nodes;
    assert_eq!(children.len(), 1, "<Autofocus> only accepts one child");

    let node_ref = create_node_ref();
    let child = children.remove(0);
    let child = child.into_html_element().unwrap();
    let child = child.node_ref(node_ref);

    create_effect(move |_| {
        if let Some(node) = node_ref.get() {
            node.focus().unwrap();
        }
    });

    child
}
