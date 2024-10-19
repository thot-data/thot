use leptos::*;
use std::str::FromStr;
use wasm_bindgen::JsCast;

/// Similar to `<input type="number" ... />`.
/// Handles validation.
#[component]
pub fn InputNumber(
    /// Read signal.
    /// Attached to `prop:value`.
    #[prop(into)]
    value: Signal<String>,

    /// Signal to indicate if the current value is valid or not.
    #[prop(optional, into)]
    set_is_valid: Option<WriteSignal<bool>>,

    #[prop(into)] oninput: Callback<String>,
    #[prop(optional)] min: Option<f64>,
    #[prop(optional)] max: Option<f64>,
    #[prop(optional, into)] placeholder: MaybeProp<String>,
    #[prop(default = false)] required: bool,
    #[prop(optional, into)] class: MaybeSignal<String>,
) -> impl IntoView {
    const DECIMAL_MARKER: &'static str = ".";

    let _ = watch(
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

    // NB: Must check if the typed character was a period with nothing follwing it.
    // If input ends in a perdiod (`.`) the whole number part is reported
    // **without** the period, so the value maybe updated and the period erased,
    // making it impossible to type a period as the next character.
    view! {
        <input
            type="text"
            inputmode="decimal"
            prop:value=value
            on:input=move |e: ev::Event| {
                let value = event_target_value(&e);
                if let Some(e) = e.dyn_ref::<ev::InputEvent>() {
                    if let Some(key) = e.data() {
                        if key == DECIMAL_MARKER && !value.contains(DECIMAL_MARKER) {
                            return;
                        }
                    }
                }
                oninput(value);
            }
            placeholder=placeholder
            class=class
            required=required
        />
    }
}

pub mod debounced {
    use leptos::*;

    #[component]
    pub fn InputText(
        #[prop(into)] value: MaybeSignal<String>,
        #[prop(into)] oninput: Callback<String>,
        #[prop(into)] debounce: MaybeSignal<f64>,
        #[prop(into, optional)] placeholder: MaybeProp<String>,
        #[prop(into, optional)] minlength: MaybeProp<usize>,
        #[prop(optional, into)] class: MaybeProp<String>,
    ) -> impl IntoView {
        let (input_value, set_input_value) = create_signal(value::State::set_from_state(value()));
        let input_value = leptos_use::signal_debounced(input_value, debounce);

        let _ = watch(
            value,
            move |value, _, _| {
                set_input_value(value::State::set_from_state(value.clone()));
            },
            false,
        );

        create_effect(move |_| {
            input_value.with(|value| {
                if value.was_set_from_input() {
                    oninput(value.value().clone());
                }
            })
        });

        view! {
            <input
                prop:value=move || { input_value.with(|value| { value.value().clone() }) }

                on:input=move |e| {
                    let v = event_target_value(&e);
                    set_input_value(value::State::set_from_input(v))
                }

                placeholder=placeholder
                minlength=minlength
                class=class
            />
        }
    }

    #[component]
    pub fn TextArea(
        #[prop(into)] value: MaybeSignal<String>,
        #[prop(into)] oninput: Callback<String>,
        #[prop(into)] debounce: MaybeSignal<f64>,
        #[prop(into, optional)] placeholder: MaybeProp<String>,
        #[prop(into, optional)] class: MaybeProp<String>,
    ) -> impl IntoView {
        let (input_value, set_input_value) = create_signal(value::State::set_from_state(value()));
        let input_value = leptos_use::signal_debounced(input_value, debounce);

        let _ = watch(
            value,
            move |value, _, _| {
                set_input_value(value::State::set_from_state(value.clone()));
            },
            false,
        );

        create_effect(move |_| {
            input_value.with(|value| {
                if value.was_set_from_input() {
                    oninput(value.value().clone());
                }
            })
        });

        // TODO: Update from source does not update value.
        view! {
            <textarea
                on:input=move |e| {
                    let v = event_target_value(&e);
                    set_input_value(value::State::set_from_input(v))
                }

                placeholder=placeholder
                class=class
            >

                {input_value.with(|value| value.value().clone())}
            </textarea>
        }
    }

    pub mod value {
        /// Value and source.
        #[derive(derive_more::Deref, Clone, Debug)]
        pub struct State<T> {
            /// Source of the value.
            source: Source,

            #[deref]
            value: T,
        }

        impl<T> State<T> {
            pub fn set_from_state(value: T) -> Self {
                Self {
                    source: Source::State,
                    value,
                }
            }

            pub fn set_from_input(value: T) -> Self {
                Self {
                    source: Source::Input,
                    value,
                }
            }

            pub fn source(&self) -> &Source {
                &self.source
            }

            pub fn value(&self) -> &T {
                &self.value
            }

            pub fn was_set_from_state(&self) -> bool {
                self.source == Source::State
            }

            pub fn was_set_from_input(&self) -> bool {
                self.source == Source::Input
            }
        }

        /// Source of current value.
        #[derive(PartialEq, Clone, Debug)]
        pub enum Source {
            /// Value state.
            State,

            /// User input.
            Input,
        }
    }
}
