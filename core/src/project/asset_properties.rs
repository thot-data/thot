//! Standard properties associated with other resources.
use super::Metadata;
use crate::types::{Creator, Value};
use chrono::prelude::*;
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// ************************
// *** Asset Properties ***
// ************************

/// Standard resource properties.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AssetProperties {
    created: DateTime<Utc>,
    pub creator: Creator,

    pub name: Option<String>,
    pub kind: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub metadata: Metadata,
}

impl AssetProperties {
    pub fn new() -> Self {
        Self {
            created: Utc::now(),
            creator: Creator::User(None),

            name: None,
            kind: None,
            description: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn created(&self) -> &DateTime<Utc> {
        &self.created
    }
}

// ***************
// *** Builder ***
// ***************

pub struct Builder {
    created: Option<DateTime<Utc>>,
    creator: Creator,
    name: Option<String>,
    kind: Option<String>,
    description: Option<String>,
    tags: Vec<String>,
    metadata: Metadata,
}

impl Builder {
    pub fn set_created(&mut self, value: DateTime<Utc>) -> &mut Self {
        self.created = Some(value);
        self
    }

    pub fn clear_created(&mut self) -> &mut Self {
        self.created = None;
        self
    }

    pub fn set_creator(&mut self, value: Creator) -> &mut Self {
        self.creator = value;
        self
    }

    pub fn set_name(&mut self, value: impl Into<String>) -> &mut Self {
        self.name = Some(value.into());
        self
    }

    pub fn clear_name(&mut self) -> &mut Self {
        self.name = None;
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
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            created: None,
            creator: Creator::default(),
            name: None,
            kind: None,
            description: None,
            tags: Vec::default(),
            metadata: Metadata::default(),
        }
    }
}

impl Into<AssetProperties> for Builder {
    fn into(self) -> AssetProperties {
        AssetProperties {
            created: self.created.unwrap_or_else(|| Utc::now()),
            creator: self.creator,
            name: self.name,
            kind: self.kind,
            description: self.description,
            tags: self.tags,
            metadata: self.metadata,
        }
    }
}
