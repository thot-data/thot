use super::tags::TagsBulkEditor;
use super::types::BulkValue;
use crate::widgets::metadata::{MetadataBulk, MetadataBulkEditor, Metadatum};
use serde_json::Value as JsValue;
use std::rc::Rc;
use thot_core::project::ContainerProperties;
use yew::prelude::*;

// ***************
// *** reducer ***
// ***************

enum ContainerPropertiesUpdateStateAction {
    /// Set all values from properties.
    SetValues(Vec<ContainerProperties>),
}

#[derive(PartialEq, Clone)]
struct ContainerPropertiesUpdateState {
    name: BulkValue<String>,
    kind: BulkValue<Option<String>>,
    description: BulkValue<Option<String>>,
    tags: Vec<String>,
    metadata: MetadataBulk,
}

impl ContainerPropertiesUpdateState {
    pub fn new(properties: &Vec<ContainerProperties>) -> Self {
        let n_props = properties.len();
        let mut names = Vec::with_capacity(n_props);
        let mut kinds = Vec::with_capacity(n_props);
        let mut descriptions = Vec::with_capacity(n_props);
        for prop in properties.iter() {
            names.push(Some(prop.name.clone()));
            kinds.push(prop.kind.clone());
            descriptions.push(prop.description.clone());
        }

        names.sort();
        names.dedup();
        kinds.sort();
        kinds.dedup();
        descriptions.sort();
        descriptions.dedup();

        let name = match names.len() {
            1 => match names[0].as_ref() {
                None => panic!("name can not be `None`"),
                Some(name) => BulkValue::Equal(name.clone()),
            },

            _ => BulkValue::Mixed,
        };

        let kind = match kinds.len() {
            1 => BulkValue::Equal(kinds[0].clone()),
            _ => BulkValue::Mixed,
        };

        let description = match descriptions.len() {
            1 => BulkValue::Equal(descriptions[0].clone()),
            _ => BulkValue::Mixed,
        };

        let mut tags = properties
            .iter()
            .map(|props| props.tags.clone())
            .flatten()
            .collect::<Vec<String>>();

        tags.sort();
        tags.dedup();

        let mut metadata = MetadataBulk::new();
        for props in properties.iter() {
            // initialize all keys
            for key in props.metadata.keys() {
                metadata.insert(key.clone(), Vec::new());
            }
        }

        for props in properties {
            // insert values
            for (key, md) in metadata.iter_mut() {
                let value = props.metadata.get(key).unwrap_or(&JsValue::Null);
                if !md.contains(value) {
                    md.push(value.clone());
                }
            }
        }

        Self {
            name,
            kind,
            description,
            tags,
            metadata,
        }
    }

    pub fn name(&self) -> &BulkValue<String> {
        &self.name
    }

    pub fn kind(&self) -> &BulkValue<Option<String>> {
        &self.kind
    }

    pub fn description(&self) -> &BulkValue<Option<String>> {
        &self.description
    }

    pub fn tags(&self) -> &Vec<String> {
        &self.tags
    }

    pub fn metadata(&self) -> &MetadataBulk {
        &self.metadata
    }
}

impl Reducible for ContainerPropertiesUpdateState {
    type Action = ContainerPropertiesUpdateStateAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            ContainerPropertiesUpdateStateAction::SetValues(properties) => {
                Self::new(&properties).into()
            }
        }
    }
}

// *****************
// *** component ***
// *****************

#[derive(Properties, PartialEq)]
pub struct ContainerPropertiesBulkEditorProps {
    pub properties: Vec<ContainerProperties>,

    #[prop_or_default]
    pub onchange_name: Callback<String>,

    #[prop_or_default]
    pub onchange_kind: Callback<Option<String>>,

    #[prop_or_default]
    pub onchange_description: Callback<Option<String>>,

    #[prop_or_default]
    pub onadd_tag: Callback<Vec<String>>,

    #[prop_or_default]
    pub onremove_tag: Callback<String>,

    /// Called when metadatum is added.
    #[prop_or_default]
    pub onadd_metadata: Callback<Metadatum>,

    /// Called when metadatum is removed.
    ///
    /// # Arguments
    /// 1. Key to be removed.
    #[prop_or_default]
    pub onremove_metadata: Callback<String>,

    /// Called when a metadatum value is changed.
    #[prop_or_default]
    pub onchange_metadata: Callback<Metadatum>,
}

#[function_component(ContainerPropertiesBulkEditor)]
pub fn standard_properties_bulk_editor(props: &ContainerPropertiesBulkEditorProps) -> Html {
    assert!(
        props.properties.len() > 1,
        "bulk editor should not be used with fewer than two items."
    );

    let updater_state = use_reducer(|| ContainerPropertiesUpdateState::new(&props.properties));
    let name_ref = use_node_ref();
    let kind_ref = use_node_ref();
    let description_ref = use_node_ref();

    {
        let properties = props.properties.clone();
        let updater_state = updater_state.clone();

        use_effect_with(properties, move |properties| {
            updater_state.dispatch(ContainerPropertiesUpdateStateAction::SetValues(
                properties.clone(),
            ));
        });
    }

    // -----------------------
    // --- change handlers ---
    // -----------------------

    let onchange_name = {
        let onchange_name = props.onchange_name.clone();
        let elm = name_ref.clone();

        Callback::from(move |_: Event| {
            // update state
            let elm = elm
                .cast::<web_sys::HtmlInputElement>()
                .expect("could not cast `NodeRef` into element");

            let value = elm.value().trim().to_string();
            if !value.is_empty() {
                onchange_name.emit(value);
            }
        })
    };

    let onchange_kind = {
        let onchange_kind = props.onchange_kind.clone();
        let elm = kind_ref.clone();

        Callback::from(move |_: Event| {
            // update state
            let elm = elm
                .cast::<web_sys::HtmlInputElement>()
                .expect("could not cast `NodeRef` into element");

            let value = elm.value().trim().to_string();
            let value = Some(value).filter(|value| !value.is_empty());
            onchange_kind.emit(value.clone());
        })
    };

    let onchange_description = {
        let onchange_description = props.onchange_description.clone();
        let elm = description_ref.clone();

        Callback::from(move |_: Event| {
            // update state
            let elm = elm
                .cast::<web_sys::HtmlTextAreaElement>()
                .expect("could not cast `NodeRef` into element");

            let value = elm.value().trim().to_string();
            let value = Some(value).filter(|value| !value.is_empty());
            onchange_description.emit(value);
        })
    };

    let onadd_tag = {
        let onadd_tag = props.onadd_tag.clone();
        Callback::from(move |tags: Vec<String>| {
            onadd_tag.emit(tags);
        })
    };

    let onremove_tag = {
        let onremove_tag = props.onremove_tag.clone();
        Callback::from(move |tag: String| {
            onremove_tag.emit(tag);
        })
    };

    let onadd_metadata = {
        let onadd_metadata = props.onadd_metadata.clone();
        Callback::from(move |metadata: Metadatum| {
            onadd_metadata.emit(metadata);
        })
    };

    let onremove_metadata = {
        let onremove_metadata = props.onremove_metadata.clone();
        Callback::from(move |key: String| {
            onremove_metadata.emit(key);
        })
    };

    let onchange_metadata = {
        let onchange_metadata = props.onchange_metadata.clone();
        Callback::from(move |metadatum: Metadatum| {
            onchange_metadata.emit(metadatum);
        })
    };

    // ------------
    // --- html ---
    // ------------

    let onsubmit = Callback::from(|e: SubmitEvent| {
        e.prevent_default();
    });

    html! {
        <form class={classes!("thot-ui-bulk-container-properties-editor")} {onsubmit}>
            <div class={classes!("form-field", "name")}>
                <label>
                    <h3> { "Name" } </h3>
                    <input
                        ref={name_ref}
                        placeholder={value_placeholder(updater_state.name())}
                        value={value_name(updater_state.name())}
                        onchange={onchange_name} />
                </label>
            </div>

            <div class={classes!("form-field", "kind")}>
                <label>
                    <h3> { "Type" } </h3>
                    <input
                        ref={kind_ref}
                        placeholder={value_placeholder(updater_state.kind())}
                        value={value_string(updater_state.kind())}
                        onchange={onchange_kind} />
                </label>
            </div>

            <div class={classes!("form-field", "description")}>
                <label>
                    <h3> { "Description" } </h3>
                    <textarea
                        ref={description_ref}
                        placeholder={value_placeholder(updater_state.description())}
                        value={value_string(updater_state.description())}
                        onchange={onchange_description}></textarea>
                </label>
            </div>

            <div class={classes!("form-field", "tags")}>
                <label>
                    <h3> { "Tags" } </h3>
                    <TagsBulkEditor
                        value={updater_state.tags().clone()}
                        onadd={onadd_tag}
                        onremove={onremove_tag} />
                </label>
            </div>

            <div class={classes!("form-field", "metadata")}>
                <MetadataBulkEditor
                    value={updater_state.metadata().clone()}
                    onadd={onadd_metadata}
                    onremove={onremove_metadata}
                    onchange={onchange_metadata} />
            </div>
    </form>
    }
}

// ***************
// *** helpers ***
// ***************

fn value_name(value: &BulkValue<String>) -> Option<String> {
    match value {
        BulkValue::Equal(val) => Some(val.clone()),
        BulkValue::Mixed => None,
    }
}

fn value_string(value: &BulkValue<Option<String>>) -> Option<String> {
    match value {
        BulkValue::Equal(val) => val.clone(),
        BulkValue::Mixed => None,
    }
}

fn value_placeholder<T>(value: &BulkValue<T>) -> &'static str
where
    T: PartialEq + Clone,
{
    match value {
        BulkValue::Equal(_) => "",
        BulkValue::Mixed => "(mixed)",
    }
}
