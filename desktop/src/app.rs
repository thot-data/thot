use crate::{
    pages::{
        auth::{Login, Logout, Register},
        project::Workspace,
        Index,
    },
    types,
};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use message::Messages;

/// For Tailwind to include classes
/// they must appear as string literals in at least one place.
/// This array is used to include them when needed.
static _TAILWIND_CLASSES: &'static [&'static str] = &["hidden", "invisible"];

/// User prefers dark theme.
#[derive(derive_more::Deref, Clone, Copy)]
pub struct PrefersDarkTheme(RwSignal<bool>);
impl PrefersDarkTheme {
    pub fn new(prefers_dark: bool) -> Self {
        Self(create_rw_signal(prefers_dark))
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    provide_context(types::Messages::new()); // TODO: Only provide after user is logged in?
    let (stored_prefers_dark, set_stored_prefers_dark, _) = leptos_use::storage::use_local_storage::<
        bool,
        codee::string::FromToStringCodec,
    >("dark_mode");
    let prefers_dark_theme = PrefersDarkTheme::new(stored_prefers_dark.get_untracked());
    provide_context(prefers_dark_theme);
    create_effect(move |_| {
        set_stored_prefers_dark(prefers_dark_theme());
    });

    let class_html = move || {
        if prefers_dark_theme() {
            "dark"
        } else {
            ""
        }
    };

    view! {
        <Title formatter=|text| text text="Syre" />
        <Html class=class_html />
        <Body class="h-screen font-secondary overflow-hidden dark:bg-secondary-800 dark:text-white" />

        <Router>
            <Routes>
                <Route path="" view=Index />
                <Route path="register" view=move || view! { <Register /> } />
                <Route path="login" view=Login />
                <Route path="logout" view=Logout />
                <Route path=":id" view=Workspace />
            </Routes>
        </Router>
        <Messages />
    }
}

mod message {
    use crate::{
        components::{self, ToggleExpand},
        types,
    };
    use leptos::{ev::MouseEvent, *};
    use leptos_icons::Icon;

    #[component]
    pub fn Messages() -> impl IntoView {
        let messages = expect_context::<types::Messages>();
        view! {
            <div class="absolute bottom-0 right-2 w-1/2 max-w-md max-h-[75%] overflow-auto flex flex-col gap-2 scrollbar-thin z-50">
                {move || {
                    messages
                        .with(|messages| {
                            messages
                                .iter()
                                .rev()
                                .map(|message| {
                                    view! { <Message message /> }
                                })
                                .collect::<Vec<_>>()
                        })
                }}
            </div>
        }
    }

    #[component]
    fn Message<'a>(message: &'a types::Message) -> impl IntoView {
        let messages = expect_context::<types::Messages>();
        let show_body = create_rw_signal(false);

        let close = {
            let message_id = message.id();
            move |e: MouseEvent| {
                if e.button() != types::MouseButton::Primary {
                    return;
                }

                messages.update(|messages| messages.retain(|msg| msg.id() != message_id));
            }
        };

        let (class_main, class_btn) = match message.kind() {
            types::message::MessageKind::Info => (
                "flex bg-primary-500 border border-primary-600 rounded",
                "border-l border-l-primary-600 flex",
            ),
            types::message::MessageKind::Success => (
                "flex bg-syre-green-600 border border-syre-green-700 rounded",
                "border-l border-l-green-700 flex",
            ),
            types::message::MessageKind::Warning => (
                "flex bg-syre-yellow-600 border border-syre-yellow-700 rounded",
                "border-l border-l-yellow-700 flex",
            ),
            types::message::MessageKind::Error => (
                "flex bg-syre-red-500 border border-syre-red-700 rounded",
                "border-l border-l-red-700 flex",
            ),
        };

        view! {
            <div class=class_main>
                <div class="grow px-2">
                    <div class="relative flex gap-2">
                        <div class="text-lg grow">{message.title()}</div>
                        {message
                            .body()
                            .map(|_| {
                                view! {
                                    <div>
                                        <ToggleExpand expanded=show_body />
                                    </div>
                                }
                            })}
                    </div>
                    {message
                        .body()
                        .map(|body| {
                            view! {
                                <div
                                    class:hidden=move || !show_body()
                                    class="pt-4 max-h-48 overflow-auto select-text scrollbar-thin"
                                >
                                    {body}
                                </div>
                            }
                        })}
                </div>
                <div class=class_btn>
                    <button on:mousedown=close class="px-2 w-full h-full">
                        <Icon icon=components::icon::Close />
                    </button>
                </div>
            </div>
        }
    }
}
