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
    use std::usize;

    use leptos::*;

    #[derive(Clone, Copy, Debug)]
    pub enum MessageKind {
        Success,
        Warning,
        Error,
        Info,
    }

    pub struct Builder {
        title: String,
        body: Option<View>,
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

        pub fn body(&mut self, body: impl IntoView) -> &mut Self {
            let _ = self.body.insert(body.into_view());
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

    #[derive(Debug)]
    pub struct Message {
        id: usize,
        kind: MessageKind,
        title: String,
        body: Option<View>,
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

        pub fn body(&self) -> Option<&View> {
            self.body.as_ref()
        }
    }

    /// App wide messages.
    #[derive(Clone, derive_more::Deref, Copy)]
    pub struct Messages(RwSignal<Vec<Message>>);
    impl Messages {
        pub fn new() -> Self {
            Self(create_rw_signal(vec![]))
        }
    }
}

pub mod settings {
    use leptos::*;
    use syre_desktop_lib as lib;

    #[derive(derive_more::Deref, Clone, Copy)]
    pub struct User(RwSignal<lib::settings::User>);
    impl User {
        pub fn new(settings: lib::settings::User) -> Self {
            Self(create_rw_signal(settings))
        }
    }
}
