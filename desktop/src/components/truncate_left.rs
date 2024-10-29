use leptos::*;

// #[component]
// pub fn TruncateLeft(children: Children, #[prop(optional, into)] class: TextProp) -> impl IntoView {
//     let content_ref = NodeRef::<html::Div>::new();

//     let content_overflow = move || {
//         let Some(content) = content_ref.get() else {
//             return false;
//         };

//         tracing::debug!("{:?}, {:?}", content.scroll_width(), content.client_width());
//         content.scroll_width() >= content.client_width()
//     };

//     let classes = {
//         let class = class.clone();
//         move || format!("truncate-rtl text-clip {}", class.get())
//     };

//     view! {
//         <div class="flex">
//             <span class:hidden=move || !content_overflow()>"..."</span>
//             <div class=classes>
//                 <div class="inline-block ltr" ref=content_ref>
//                     {children()}
//                 </div>
//             </div>
//         </div>
//     }
// }

#[component]
pub fn TruncateLeft(
    children: Children,
    #[prop(optional, into)] class: TextProp,
    #[prop(optional, into)] inner_class: TextProp,
) -> impl IntoView {
    let classes = {
        let class = class.clone();
        move || format!("truncate-rtl {}", class.get())
    };

    let inner_classes = {
        let class = inner_class.clone();
        move || format!("ltr inline-block {}", class.get())
    };

    view! {
        <div class=classes>
            <span class=inner_classes>{children()}</span>
        </div>
    }
}
