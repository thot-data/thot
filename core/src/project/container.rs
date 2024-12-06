//! Container.
use super::{
    container_properties::{Builder as PropertiesBuilder, ContainerProperties},
    AnalysisAssociation, Asset, Metadata,
};
use crate::{
    db::Resource,
    types::{ResourceId, Value},
};
use has_id::HasId;
use std::hash::{Hash, Hasher};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Debug, HasId)]
pub struct Container {
    #[id]
    rid: ResourceId,
    pub properties: ContainerProperties,
    pub assets: Vec<Asset>,
    pub analyses: Vec<AnalysisAssociation>,
}

impl Container {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            rid: ResourceId::new(),
            properties: ContainerProperties::new(name),
            assets: vec![],
            analyses: vec![],
        }
    }

    pub fn with_id(name: impl Into<String>, rid: ResourceId) -> Self {
        Self {
            rid,
            properties: ContainerProperties::new(name),
            assets: vec![],
            analyses: vec![],
        }
    }

    pub fn from_parts(
        rid: ResourceId,
        properties: ContainerProperties,
        assets: Vec<Asset>,
        analyses: Vec<AnalysisAssociation>,
    ) -> Self {
        Self {
            rid,
            properties,
            assets,
            analyses,
        }
    }

    pub fn rid(&self) -> &ResourceId {
        &self.rid
    }
}

impl Hash for Container {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.rid.hash(state);
    }
}

impl Resource for Container {}

// ***************
// *** Builder ***
// ***************

#[derive(Default)]
pub struct Builder {
    properties: PropertiesBuilder,
    assets: Vec<Asset>,
    analyses: Vec<AnalysisAssociation>,
}

impl Builder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            properties: PropertiesBuilder::new(name),
            assets: vec![],
            analyses: vec![],
        }
    }

    pub fn set_name(&mut self, value: impl Into<String>) -> &mut Self {
        self.properties.set_name(value);
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

    /// Inserts the `Asset`.
    /// If an `Asset` with the same resource id already exists,
    /// it is replaced.
    pub fn insert_asset(&mut self, asset: Asset) -> &mut Self {
        self.assets.retain(|a| a.rid() != asset.rid());
        self.assets.push(asset);
        self
    }

    pub fn remove_asset(&mut self, rid: &ResourceId) -> &mut Self {
        self.assets.retain(|asset| asset.rid() != rid);
        self
    }

    /// Inserts an analysis association.
    /// If an association with the same analysis already exists,
    /// it is replaced.
    pub fn insert_analysis(&mut self, association: AnalysisAssociation) -> &mut Self {
        self.analyses
            .retain(|a| a.analysis() != association.analysis());
        self.analyses.push(association);
        self
    }

    pub fn remove_analysis(&mut self, rid: &ResourceId) -> &mut Self {
        self.analyses
            .retain(|association| association.analysis() != rid);
        self
    }

    pub fn build(self) -> Container {
        self.into()
    }
}

impl Into<Container> for Builder {
    fn into(self) -> Container {
        Container {
            rid: ResourceId::new(),
            properties: self.properties.into(),
            assets: self.assets,
            analyses: self.analyses,
        }
    }
}
