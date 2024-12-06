/// Asset.
use super::{asset_properties::Builder as PropertiesBuilder, AssetProperties, Metadata};
use crate::{
    db::Resource,
    types::{Creator, ResourceId, Value},
};
use chrono::prelude::*;
use has_id::HasId;
use std::{
    hash::{Hash, Hasher},
    path::PathBuf,
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "serde")]
use has_id::HasIdSerde;

/// Assets represent a consumable or producable resource.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, HasIdSerde))]
#[derive(HasId, Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    #[id]
    rid: ResourceId,
    pub properties: AssetProperties,

    /// Path to the `Asset`'s resource file.
    pub path: PathBuf,
}

impl Asset {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            rid: ResourceId::new(),
            properties: AssetProperties::new(),
            path: path.into(),
        }
    }

    pub fn with_properties(path: impl Into<PathBuf>, properties: AssetProperties) -> Self {
        Self {
            rid: ResourceId::new(),
            properties,
            path: path.into(),
        }
    }

    pub fn rid(&self) -> &ResourceId {
        &self.rid
    }

    /// Returns the `bucket` path of the `Asset`
    /// if it is in one, otherwise `None`.
    ///
    /// # Notes
    /// + An `Asset` is in a `bucket` if its `path` is
    /// a [`ResourcePath::Relative`] with no ancestors (i.e. `..`)
    /// after canonicalization.
    /// + An `Asset` in its [`Container`](super::Container)'s root
    /// is considered to be in the `root` `bucket`.
    pub fn bucket(&self) -> Option<PathBuf> {
        self.path.parent().map(|p| p.to_path_buf())
    }
}

impl Hash for Asset {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.rid.hash(state);
    }
}

impl Resource for Asset {}

// ***************
// *** Builder ***
// ***************

pub struct NoPath;
pub struct Path(PathBuf);

pub struct Builder<P> {
    properties: PropertiesBuilder,
    path: P,
}

impl<P> Builder<P> {
    pub fn set_created(&mut self, value: DateTime<Utc>) -> &mut Self {
        self.properties.set_created(value);
        self
    }

    pub fn clear_created(&mut self) -> &mut Self {
        self.properties.clear_created();
        self
    }

    pub fn set_creator(&mut self, value: Creator) -> &mut Self {
        self.properties.set_creator(value);
        self
    }

    pub fn set_name(&mut self, value: impl Into<String>) -> &mut Self {
        self.properties.set_name(value);
        self
    }

    pub fn clear_name(&mut self) -> &mut Self {
        self.properties.clear_name();
        self
    }

    pub fn set_kind(&mut self, value: impl Into<String>) -> &mut Self {
        self.properties.set_kind(value);
        self
    }

    pub fn clear_kind(&mut self) -> &mut Self {
        self.properties.clear_kind();
        self
    }

    pub fn set_description(&mut self, value: impl Into<String>) -> &mut Self {
        self.properties.set_description(value);
        self
    }

    pub fn clear_description(&mut self) -> &mut Self {
        self.properties.clear_description();
        self
    }

    pub fn set_tags(&mut self, value: Vec<impl Into<String>>) -> &mut Self {
        self.properties.set_tags(value);
        self
    }

    pub fn clear_tags(&mut self) -> &mut Self {
        self.properties.clear_tags();
        self
    }

    pub fn add_tag(&mut self, value: impl Into<String>) -> &mut Self {
        self.properties.add_tag(value);
        self
    }

    pub fn remove_tag(&mut self, value: impl Into<String>) -> &mut Self {
        self.properties.remove_tag(value);
        self
    }

    pub fn set_metadata(&mut self, value: Metadata) -> &mut Self {
        self.properties.set_metadata(value);
        self
    }

    pub fn clear_metadata(&mut self) -> &mut Self {
        self.properties.clear_metadata();
        self
    }

    pub fn set_metadatum(&mut self, key: impl Into<String>, value: impl Into<Value>) -> &mut Self {
        self.properties.set_metadatum(key, value);
        self
    }

    pub fn remove_metadatum(&mut self, key: impl Into<String>) -> &mut Self {
        self.properties.remove_metadatum(key);
        self
    }

    pub fn set_path(self, value: PathBuf) -> Builder<Path> {
        Builder {
            properties: self.properties,
            path: Path(value),
        }
    }
}

impl Builder<NoPath> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Builder<Path> {
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            properties: PropertiesBuilder::default(),
            path: Path(path.into()),
        }
    }

    pub fn clear_path(self) -> Builder<NoPath> {
        Builder {
            properties: self.properties,
            path: NoPath,
        }
    }

    pub fn build(self) -> Asset {
        self.into()
    }
}

impl Default for Builder<NoPath> {
    fn default() -> Self {
        Self {
            properties: PropertiesBuilder::default(),
            path: NoPath,
        }
    }
}

impl Into<Asset> for Builder<Path> {
    fn into(self) -> Asset {
        Asset {
            rid: ResourceId::new(),
            properties: self.properties.into(),
            path: self.path.0,
        }
    }
}

#[cfg(test)]
#[path = "./asset_test.rs"]
mod asset_test;
