use leptos::{
    ev::{Event, FocusEvent, InputEvent},
    html,
    prelude::*,
};
use std::str::FromStr;
use wasm_bindgen::JsCast;

/// Similar to `<input type="number" ... />`.
/// Handles validation.
#[component]
pub fn InputNumber(
    #[prop(optional)] node_ref: NodeRef<html::Input>,

    /// Read signal.
    /// Attached to `prop:value`.
    #[prop(into)]
    value: Signal<String>,

    /// Signal to indicate if the current value is valid or not.
    #[prop(optional, into)]
    set_is_valid: Option<WriteSignal<bool>>,

    #[prop(into, optional)] oninput: Option<Callback<String>>,
    #[prop(into, optional)] onblur: Option<Callback<FocusEvent>>,
    #[prop(optional)] min: Option<f64>,
    #[prop(optional)] max: Option<f64>,
    #[prop(into, optional)] placeholder: MaybeProp<String>,
    #[prop(default = false)] required: bool,
    #[prop(into, optional)] class: Signal<String>,
) -> impl IntoView {
    const DECIMAL_MARKER: &'static str = ".";

    let _ = Effect::watch(
        value,
        move |value, _, prev_validity| {
            let Some(set_is_valid) = set_is_valid else {
                return true;
            };

            // NB: For JSON, leading zeros (`0`) are not valid unless it is
            // the only character.
            let validity = if value == "0" {
                true
            } else {
                let value = value.trim_start_matches("0");
                serde_json::Number::from_str(value).is_ok()
            };

            if let Some(prev_validity) = prev_validity.as_ref() {
                if validity != *prev_validity {
                    set_is_valid.set(validity);
                }
            } else {
                set_is_valid.set(validity);
            }

            validity
        },
        true,
    );

    let handle_oninput = move |e: Event| {
        let value = event_target_value(&e);
        if let Some(e) = e.dyn_ref::<InputEvent>() {
            if let Some(key) = e.data() {
                if key == DECIMAL_MARKER && !value.contains(DECIMAL_MARKER) {
                    return;
                }
            }
        }
        if let Some(oninput) = oninput {
            oninput(value);
        }
    };

    let handle_onblur = move |e: FocusEvent| {
        if let Some(onblur) = onblur {
            onblur(e);
        }
    };

    // NB: Must check if the typed character was a period with nothing following it.
    // If input ends in a perdiod (`.`) the whole number part is reported
    // **without** the period, so the value maybe updated and the period erased,
    // making it impossible to type a period as the next character.
    view! {
        <input
            node_ref=node_ref
            type="text"
            inputmode="decimal"
            prop:value=value
            on:input=handle_oninput
            on:blur=handle_onblur
            placeholder=placeholder
            class=class
            required=required
        />
    }
}

pub mod debounced {
    use leptos::prelude::*;

    #[component]
    pub fn InputText(
        #[prop(into)] value: Signal<String>,
        #[prop(into)] oninput: Callback<String>,
        #[prop(into)] debounce: Signal<f64>,
        #[prop(into, optional)] placeholder: MaybeProp<String>,
        #[prop(into, optional)] minlength: MaybeProp<usize>,
        #[prop(optional, into)] class: MaybeProp<String>,
    ) -> impl IntoView {
        let (input_value, set_input_value) = signal(value::State::clean(value()));
        let input_value = leptos_use::signal_debounced(input_value, debounce);

        let _ = Effect::watch(
            value.clone(),
            move |value, _, _| {
                set_input_value(value::State::clean(value.clone()));
            },
            false,
        );

        Effect::new(move |_| {
            input_value.with(|value| {
                if value.is_dirty() {
                    oninput(value.value().clone());
                }
            })
        });

        view! {
            <input
                prop:value=move || { input_value.with(|value| { value.value().clone() }) }
                on:input=move |e| {
                    let v = event_target_value(&e);
                    set_input_value(value::State::dirty(v))
                }
                on:blur=move |e| {
                    let v = event_target_value(&e);
                    if value.with(|value| *value != v) {
                        oninput(v);
                    }
                }
                placeholder=placeholder
                minlength=minlength
                class=class
            />
        }
    }

    #[component]
    pub fn InputCheckbox(
        #[prop(into)] value: Signal<bool>,
        #[prop(into)] oninput: Callback<bool>,
        #[prop(into)] debounce: Signal<f64>,
        #[prop(optional, into)] class: MaybeProp<String>,
    ) -> impl IntoView {
        let (input_value, set_input_value) = signal(value::State::clean(value()));
        let input_value = leptos_use::signal_debounced(input_value, debounce);

        let _ = Effect::watch(
            value.clone(),
            move |value, _, _| {
                set_input_value(value::State::clean(*value));
            },
            false,
        );

        let _ = Effect::watch(
            input_value,
            move |input_value, _, _| {
                if input_value.is_dirty()
                    && value.with_untracked(|value| input_value.value() != value)
                {
                    oninput(*input_value.value());
                }
            },
            false,
        );

        view! {
            <input
                type="checkbox"
                prop:value=move || { input_value.with(|value| { value.value().clone() }) }
                on:input=move |e| {
                    let v = event_target_checked(&e);
                    set_input_value(value::State::dirty(v))
                }
                on:blur=move |e| {
                    let v = event_target_checked(&e);
                    if value.with(|value| *value != v) {
                        oninput(v);
                    }
                }
                class=class
            />
        }
    }

    #[component]
    pub fn TextArea(
        #[prop(into)] value: Signal<String>,
        #[prop(into)] oninput: Callback<String>,
        #[prop(into)] debounce: Signal<f64>,
        #[prop(into, optional)] placeholder: MaybeProp<String>,
        #[prop(into, optional)] class: MaybeProp<String>,
    ) -> impl IntoView {
        let (input_value, set_input_value) = signal(value::State::clean(value.get_untracked()));
        let input_value = leptos_use::signal_debounced(input_value, debounce);

        let _ = Effect::watch(
            value.clone(),
            move |value, _, _| {
                set_input_value(value::State::clean(value.clone()));
            },
            false,
        );

        Effect::new(move |_| {
            input_value.with(|value| {
                if value.is_dirty() {
                    oninput(value.value().clone());
                }
            })
        });

        // TODO: Update from source does not update value.
        view! {
            <textarea
                on:input=move |e| {
                    let v = event_target_value(&e);
                    set_input_value(value::State::dirty(v))
                }
                on:blur=move |e| {
                    let v = event_target_value(&e);
                    if value.with(|value| *value != v) {
                        oninput(v);
                    }
                }
                placeholder=placeholder
                class=class
            >

                {move || input_value.with(|value| value.value().clone())}
            </textarea>
        }
    }

    pub mod value {
        /// Value and source.
        #[derive(derive_more::Deref, Clone, Debug)]
        pub struct State<T> {
            status: Status,

            #[deref]
            value: T,
        }

        impl<T> State<T> {
            pub fn clean(value: T) -> Self {
                Self {
                    status: Status::Clean,
                    value,
                }
            }

            pub fn dirty(value: T) -> Self {
                Self {
                    status: Status::Dirty,
                    value,
                }
            }

            pub fn status(&self) -> &Status {
                &self.status
            }

            pub fn value(&self) -> &T {
                &self.value
            }

            pub fn is_clean(&self) -> bool {
                matches!(self.status, Status::Clean)
            }

            pub fn is_dirty(&self) -> bool {
                matches!(self.status, Status::Dirty)
            }
        }

        /// Source of current value.
        #[derive(PartialEq, Clone, Debug)]
        pub enum Status {
            Clean,
            Dirty,
        }
    }
}
