pub use message::{Message, Messages};

/// Enum for different mouse buttons
/// for use with `MouseEvent::button`.
/// See https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/button#value.
#[derive(Clone, Copy, Debug)]
pub enum MouseButton {
    Primary = 0,
    // Auxillary = 1,
    // Secondary = 2,
    // Fourth = 3,
    // Fifth = 4,
}

impl PartialEq<i16> for MouseButton {
    fn eq(&self, other: &i16) -> bool {
        (*self as i16).eq(other)
    }
}

impl PartialEq<MouseButton> for i16 {
    fn eq(&self, other: &MouseButton) -> bool {
        other.eq(self)
    }
}

pub mod message {
    use leptos::prelude::*;
    use std::sync::Arc;

    #[derive(Clone, Copy, Debug)]
    pub enum MessageKind {
        Success,
        Warning,
        Error,
        Info,
    }

    /// Allows display as a [`Message`] body.
    pub trait MessageBody {
        /// Dispay as a message body.
        fn to_message_body(&self) -> AnyView;
    }

    impl<T> MessageBody for T
    where
        T: IntoAny + Clone,
    {
        fn to_message_body(&self) -> AnyView {
            self.clone().into_any()
        }
    }

    pub struct Builder {
        title: String,
        body: Option<Arc<dyn MessageBody>>,
        kind: MessageKind,
    }

    impl Builder {
        fn new(title: impl Into<String>, kind: MessageKind) -> Self {
            Self {
                title: title.into(),
                body: None,
                kind,
            }
        }

        pub fn success(title: impl Into<String>) -> Self {
            Self::new(title, MessageKind::Success)
        }

        pub fn warning(title: impl Into<String>) -> Self {
            Self::new(title, MessageKind::Warning)
        }

        pub fn error(title: impl Into<String>) -> Self {
            Self::new(title, MessageKind::Error)
        }

        pub fn info(title: impl Into<String>) -> Self {
            Self::new(title, MessageKind::Info)
        }

        pub fn body(&mut self, body: impl MessageBody + 'static) -> &mut Self {
            let _ = self.body.insert(Arc::new(body));
            self
        }

        pub fn build(self) -> Message {
            self.into()
        }
    }

    impl Into<Message> for Builder {
        fn into(self) -> Message {
            let id = (js_sys::Math::random() * (usize::MAX as f64)) as usize;
            Message {
                id,
                kind: self.kind,
                title: self.title,
                body: self.body,
            }
        }
    }

    #[derive(derive_more::Debug, Clone)]
    pub struct Message {
        id: usize,
        kind: MessageKind,
        title: String,

        #[debug(skip)]
        body: Option<Arc<dyn MessageBody>>,
    }

    impl Message {
        pub fn id(&self) -> usize {
            self.id
        }

        pub fn kind(&self) -> MessageKind {
            self.kind
        }

        pub fn title(&self) -> &String {
            &self.title
        }

        pub fn body(&self) -> Option<AnyView> {
            self.body.as_ref().map(|body| body.to_message_body())
        }
    }

    /// App wide messages.
    #[derive(Clone, derive_more::Deref, Copy)]
    pub struct Messages(RwSignal<Vec<Message>, LocalStorage>);
    impl Messages {
        pub fn new() -> Self {
            Self(RwSignal::new_local(vec![]))
        }
    }
}

pub mod settings {
    use leptos::prelude::*;
    use syre_desktop_lib as lib;

    #[derive(derive_more::Deref, Clone, Copy)]
    pub struct User(RwSignal<lib::settings::User>);
    impl User {
        pub fn new(settings: lib::settings::User) -> Self {
            Self(RwSignal::new(settings))
        }
    }

    #[derive(derive_more::Deref, Clone, Copy)]
    pub struct Project(RwSignal<lib::settings::Project>);
    impl Project {
        pub fn new(settings: lib::settings::Project) -> Self {
            Self(RwSignal::new(settings))
        }
    }
}
