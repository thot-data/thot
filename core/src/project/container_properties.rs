//! Container properties.
use super::Metadata;
use crate::types::Value;
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerProperties {
    pub name: String,
    pub kind: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub metadata: Metadata,
}

impl ContainerProperties {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: None,
            description: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

// ***************
// *** Builder ***
// ***************

#[derive(Default)]
pub struct Builder {
    name: String,
    kind: Option<String>,
    description: Option<String>,
    tags: Vec<String>,
    metadata: Metadata,
}

impl Builder {
    pub fn set_name(&mut self, value: impl Into<String>) -> &mut Self {
        self.name = value.into();
        self
    }

    pub fn set_kind(&mut self, value: impl Into<String>) -> &mut Self {
        self.kind = Some(value.into());
        self
    }

    pub fn clear_kind(&mut self) -> &mut Self {
        self.kind = None;
        self
    }

    pub fn set_description(&mut self, value: impl Into<String>) -> &mut Self {
        self.description = Some(value.into());
        self
    }

    pub fn clear_description(&mut self) -> &mut Self {
        self.description = None;
        self
    }

    pub fn set_tags(&mut self, value: Vec<impl Into<String>>) -> &mut Self {
        self.tags = value.into_iter().map(|val| val.into()).collect();
        self
    }

    pub fn clear_tags(&mut self) -> &mut Self {
        self.tags.clear();
        self
    }

    pub fn add_tag(&mut self, value: impl Into<String>) -> &mut Self {
        let value = value.into();
        if self.tags.iter().filter(|tag| tag == &&value).count() == 0 {
            self.tags.push(value);
        }

        self
    }

    pub fn remove_tag(&mut self, value: impl Into<String>) -> &mut Self {
        let value = value.into();
        self.tags.retain(|tag| tag != &value);
        self
    }

    pub fn set_metadata(&mut self, value: Metadata) -> &mut Self {
        self.metadata = value;
        self
    }

    pub fn clear_metadata(&mut self) -> &mut Self {
        self.metadata.clear();
        self
    }

    pub fn set_metadatum(&mut self, key: impl Into<String>, value: impl Into<Value>) -> &mut Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn remove_metadatum(&mut self, key: impl Into<String>) -> &mut Self {
        self.metadata.remove(&key.into());
        self
    }
}

impl Builder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: None,
            description: None,
            tags: Vec::default(),
            metadata: Metadata::default(),
        }
    }
}

impl Into<ContainerProperties> for Builder {
    fn into(self) -> ContainerProperties {
        ContainerProperties {
            name: self.name,
            kind: self.kind,
            description: self.description,
            tags: self.tags,
            metadata: self.metadata,
        }
    }
}
