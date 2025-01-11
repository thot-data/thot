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
    pub use project::Settings as Project;
    pub use user::Settings as User;

    pub mod user {
        use reactive_stores::Store;
        use std::{num::NonZeroUsize, path::PathBuf};
        use syre_desktop_lib as lib;
        use syre_local as local;

        /// # Notes
        /// If using as a [`Store`] must scope the `*StoreFields` trait.
        #[derive(Store, Clone)]
        pub struct Settings {
            pub desktop: Result<Desktop, local::error::IoSerde>,
            pub runner: Result<Runner, local::error::IoSerde>,
        }

        impl Settings {
            pub fn new_store(settings: lib::settings::User) -> Store<Self> {
                Store::new(settings.into())
            }
        }

        impl From<lib::settings::User> for Settings {
            fn from(value: lib::settings::User) -> Self {
                let lib::settings::User { desktop, runner } = value;
                Self {
                    desktop: desktop.map(|desktop| desktop.into()),
                    runner: runner.map(|runner| runner.into()),
                }
            }
        }

        impl Into<lib::settings::User> for Settings {
            fn into(self) -> lib::settings::User {
                let Self { desktop, runner } = self;
                lib::settings::User {
                    desktop: desktop.map(|desktop| desktop.into()),
                    runner: runner.map(|runner| runner.into()),
                }
            }
        }

        /// # Notes
        /// If using as a [`Store`] must scope the `*StoreFields` trait.
        #[derive(Store, Clone, Debug)]
        pub struct Desktop {
            /// Form input debounce in milliseconds.
            pub input_debounce_ms: usize,
        }

        impl Default for Desktop {
            fn default() -> Self {
                lib::settings::user::Desktop::default().into()
            }
        }

        impl From<lib::settings::user::Desktop> for Desktop {
            fn from(value: lib::settings::user::Desktop) -> Self {
                let lib::settings::user::Desktop { input_debounce_ms } = value;
                Self { input_debounce_ms }
            }
        }

        impl Into<lib::settings::user::Desktop> for Desktop {
            fn into(self) -> lib::settings::user::Desktop {
                let Self { input_debounce_ms } = self;
                lib::settings::user::Desktop { input_debounce_ms }
            }
        }

        /// # Notes
        /// If using as a [`Store`] must scope the `*StoreFields` trait.
        #[derive(Store, Clone, Debug)]
        pub struct Runner {
            pub python_path: Option<PathBuf>,
            pub r_path: Option<PathBuf>,
            pub continue_on_error: bool,
            pub max_tasks: Option<NonZeroUsize>,
        }

        impl From<lib::settings::user::Runner> for Runner {
            fn from(value: lib::settings::user::Runner) -> Self {
                let lib::settings::user::Runner {
                    python_path,
                    r_path,
                    continue_on_error,
                    max_tasks,
                } = value;

                Self {
                    python_path,
                    r_path,
                    continue_on_error,
                    max_tasks,
                }
            }
        }

        impl Into<lib::settings::user::Runner> for Runner {
            fn into(self) -> lib::settings::user::Runner {
                let Self {
                    python_path,
                    r_path,
                    continue_on_error,
                    max_tasks,
                } = self;

                lib::settings::user::Runner {
                    python_path,
                    r_path,
                    continue_on_error,
                    max_tasks,
                }
            }
        }
    }

    pub mod project {
        use reactive_stores::Store;
        use std::{num::NonZeroUsize, path::PathBuf};
        use syre_desktop_lib as lib;
        use syre_local as local;

        /// # Notes
        /// If using as a [`Store`] must scope the `*StoreFields` trait.
        #[derive(Store, Clone)]
        pub struct Settings {
            pub desktop: Result<Desktop, local::error::IoSerde>,
            pub runner: Result<Runner, local::error::IoSerde>,
        }

        impl Settings {
            pub fn new_store(settings: lib::settings::Project) -> Store<Self> {
                Store::new(settings.into())
            }
        }

        impl From<lib::settings::Project> for Settings {
            fn from(value: lib::settings::Project) -> Self {
                let lib::settings::Project { desktop, runner } = value;
                Self {
                    desktop: desktop.map(|settings| settings.into()),
                    runner: runner.map(|settings| settings.into()),
                }
            }
        }

        impl Into<lib::settings::Project> for Settings {
            fn into(self) -> lib::settings::Project {
                let Self { desktop, runner } = self;
                lib::settings::Project {
                    desktop: desktop.map(|settings| settings.into()),
                    runner: runner.map(|settings| settings.into()),
                }
            }
        }

        #[derive(Store, Clone, Debug)]
        pub struct Desktop {
            /// When an asset is drag-dropped, set its `type` property.
            pub asset_drag_drop_kind: Option<String>,
        }

        impl From<lib::settings::project::Desktop> for Desktop {
            fn from(value: lib::settings::project::Desktop) -> Self {
                let lib::settings::project::Desktop {
                    asset_drag_drop_kind,
                } = value;

                Self {
                    asset_drag_drop_kind,
                }
            }
        }

        impl Into<lib::settings::project::Desktop> for Desktop {
            fn into(self) -> lib::settings::project::Desktop {
                let Self {
                    asset_drag_drop_kind,
                } = self;

                lib::settings::project::Desktop {
                    asset_drag_drop_kind,
                }
            }
        }

        /// # Notes
        /// If using as a [`Store`] must scope the `*StoreFields` trait.
        #[derive(Store, Clone, Debug)]
        pub struct Runner {
            pub python_path: Option<PathBuf>,
            pub r_path: Option<PathBuf>,
            pub continue_on_error: Option<bool>,
            pub max_tasks: Option<NonZeroUsize>,
        }

        impl From<lib::settings::project::Runner> for Runner {
            fn from(value: lib::settings::project::Runner) -> Self {
                let lib::settings::project::Runner {
                    python_path,
                    r_path,
                    continue_on_error,
                    max_tasks,
                } = value;

                Self {
                    python_path,
                    r_path,
                    continue_on_error,
                    max_tasks,
                }
            }
        }

        impl Into<lib::settings::project::Runner> for Runner {
            fn into(self) -> lib::settings::project::Runner {
                let Self {
                    python_path,
                    r_path,
                    continue_on_error,
                    max_tasks,
                } = self;

                lib::settings::project::Runner {
                    python_path,
                    r_path,
                    continue_on_error,
                    max_tasks,
                }
            }
        }
    }
}
