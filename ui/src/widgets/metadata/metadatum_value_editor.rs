//! Editor for a `Metadatum` value.
use super::common;
use super::{type_from_string, type_of_value, MetadatumType};
use serde_json::Value as JsValue;
use yew::prelude::*;

#[derive(Properties, PartialEq, Debug)]
pub struct MetadatumValueEditorProps {
    #[prop_or_default]
    pub class: Classes,

    #[prop_or(JsValue::Null)]
    pub value: JsValue,

    #[prop_or_default]
    pub onchange: Callback<JsValue>,

    #[prop_or_default]
    pub onerror: Callback<String>,
}

#[tracing::instrument]
#[function_component(MetadatumValueEditor)]
pub fn metadatum_value_editor(props: &MetadatumValueEditorProps) -> Html {
    // NOTE `value` are set to default values if they can not be
    // interpreted correctly. It may be better to return an error instead,
    // although this situation should likely never arise due to their types.
    let value = use_state(|| props.value.clone());
    let value_str = use_state(|| {
        serde_json::to_string_pretty(&props.value).expect("could not stringify value")
    });
    let number_step = use_state(|| match value_str.split_once('.') {
        None => 1_f64,
        Some((_, decs)) => 10_f64.powi(-(decs.len() as i32)),
    });
    let kind_ref = use_node_ref();
    let value_ref = use_node_ref();

    {
        // update states if prop value changes
        let value = value.clone();
        let value_str = value_str.clone();

        use_effect_with(props.value.clone(), move |val| {
            let val_str = serde_json::to_string_pretty(val).expect("could not stringify value");
            value_str.set(val_str);
            value.set(val.clone());
        });
    }

    {
        // call onchange whenever the value has changed
        let onchange = props.onchange.clone();
        let value = value.clone();

        use_effect_with(value, move |value| {
            onchange.emit((**value).clone());
        });
    }

    {
        let value_str = value_str.clone();
        let number_step = number_step.clone();

        use_effect_with(value_str, move |value_str| {
            let step = match value_str.split_once('.') {
                None => 1_f64,
                Some((_, decs)) => 10_f64.powi(-(decs.len() as i32)),
            };
        
            number_step.set(step);
        });
    }

    let oninput_number = {
        let value_str = value_str.clone();
        let value_ref = value_ref.clone();

        Callback::from(move |_: InputEvent| {
            let v_in = value_ref
                .cast::<web_sys::HtmlInputElement>()
                .expect("could not convert value node ref into input");

            let val_str = v_in.value().trim().to_owned();
            value_str.set(val_str);
        })
    };

    let onchange_kind = {
        let value = value.clone();
        let kind_ref = kind_ref.clone();
        let onerror = props.onerror.clone();

        Callback::from(move |_: Event| {
            // get kind
            let kind_val = kind_ref
                .cast::<web_sys::HtmlSelectElement>()
                .expect("could not cast kind node ref into select");

            let Some(kind_val) = type_from_string(&kind_val.value()) else {
                // @unreachble
                onerror.emit("Invalid data type".to_string());
                return;
            };

            value.set(common::convert_value((*value).clone(), &kind_val));
        })
    };

    let onchange_value = {
        let value = value.clone();
        let value_ref = value_ref.clone();
        let onerror = props.onerror.clone();

        Callback::from(move |_: Event| {
            let Some(kind) = type_of_value(&*value) else {
                onerror.emit("Invalid data type".to_string());
                return;
            };

            // get value
            if let Ok(val) = common::value_from_input(value_ref.clone(), &kind) {
                if kind == MetadatumType::Number && val == JsValue::Null {
                    onerror.emit("Invalid number".to_string());
                    return;
                }

                value.set(common::convert_value(val, &kind));
            } else {
                // invalid input for type
                onerror.emit("Invalid value".to_string());
            };
        })
    };

    // create <options> for `kind` <select>
    let kind_opts = [
        MetadatumType::String,
        MetadatumType::Number,
        MetadatumType::Bool,
        MetadatumType::Array,
        MetadatumType::Object,
    ];

    let kind = type_of_value(&*value).unwrap_or_default();
    let kind_opts = html! {
        { kind_opts.into_iter().map(|k| { html! {
                <option
                    value={k.clone()}
                    selected={k.clone() == kind}>

                    { Into::<String>::into(k) }
                </option>
            }}).collect::<Html>()
        }
    };

    // ui
    let class = classes!("thot-ui-metadatum-value-editor", props.class.clone());
    html! {
        <span {class}>
            <select ref={kind_ref} onchange={onchange_kind.clone()}>
                { kind_opts }
            </select>

            { match (*value).clone() {
                JsValue::String(value) => html! {
                    <input
                        ref={value_ref}
                        {value}
                        placeholder={"Value"}
                        onchange={onchange_value.clone()} />
                },

                JsValue::Number(_value) => html! {
                    <input
                        ref={value_ref}
                        type={"number"}
                        step={number_step.to_string()}
                        value={(*value_str).clone()}
                        oninput={oninput_number}
                        onchange={onchange_value.clone()} />
                },

                JsValue::Bool(value) => html! {
                    <input
                        ref={value_ref}
                        type={"checkbox"}
                        checked={value}
                        onchange={onchange_value.clone()} />
                },

                JsValue::Array(value) => html! {
                    <textarea
                        ref={value_ref}
                        value={serde_json::to_string_pretty(&value).unwrap_or(String::default())}
                        onchange={onchange_value.clone()}>
                    </textarea>
                },

                JsValue::Object(value) => html! {
                    <textarea
                        ref={value_ref}
                        value={serde_json::to_string_pretty(&value).unwrap_or(String::default())}
                        onchange={onchange_value.clone()}>
                    </textarea>
                },

                JsValue::Null => html! {}
            }}
        </span>
    }
}
