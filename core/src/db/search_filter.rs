//! Search filter functionality.
use crate::project::{Asset, Container, Metadata};
use crate::types::ResourceId;
use std::collections::HashSet;

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize};

pub trait SearchFilter<T> {
    /// Returns `true` if the object matches the filter,
    /// otherwise `false`.
    fn matches(&self, obj: &T) -> bool;
}

#[cfg(feature = "serde")]
pub fn deserialize_possible_empty_string<'de, D>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<String>::deserialize(deserializer)? {
        None => Ok(None),
        Some(val) if val.is_empty() => Ok(Some(None)),
        Some(val) => Ok(Some(Some(val))),
    }
}

/// Search filter for all properties.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Default, Debug, Clone)]
pub struct StandardSearchFilter {
    pub rid: Option<ResourceId>,

    #[cfg_attr(
        feature = "serde",
        serde(default, deserialize_with = "deserialize_possible_empty_string")
    )]
    pub name: Option<Option<String>>,

    #[cfg_attr(
        feature = "serde",
        serde(default, deserialize_with = "deserialize_possible_empty_string")
    )]
    pub kind: Option<Option<String>>,
    pub tags: Option<HashSet<String>>,
    pub metadata: Option<Metadata>,
}

impl StandardSearchFilter {
    pub fn new() -> Self {
        Self::default()
    }
}

// ************************
// *** Container Filter ***
// ************************

impl SearchFilter<Container> for StandardSearchFilter {
    fn matches(&self, container: &Container) -> bool {
        if let Some(s_rid) = self.rid.as_ref() {
            if s_rid != container.rid() {
                return false;
            }
        }

        let props = &container.properties;
        if let Some(s_name) = self.name.as_ref() {
            if let Some(s_name) = s_name.as_ref() {
                if s_name != &props.name {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(s_kind) = self.kind.as_ref() {
            if s_kind != &props.kind {
                return false;
            }
        }

        if let Some(s_tags) = self.tags.as_ref() {
            for s_tag in s_tags {
                if !props.tags.contains(s_tag) {
                    return false;
                }
            }
        }

        if let Some(s_md) = self.metadata.as_ref() {
            for (s_key, s_val) in s_md {
                let Some(f_val) = props.metadata.get(s_key) else {
                    return false;
                };

                // only compare number values, not types
                if f_val.is_number() && s_val.is_number() {
                    if f_val.as_f64() != s_val.as_f64() {
                        return false;
                    }
                } else {
                    if f_val != s_val {
                        return false;
                    }
                }
            }
        }

        // all search criteria matched
        true
    }
}

// ********************
// *** Asset Filter ***
// ********************

impl SearchFilter<Asset> for StandardSearchFilter {
    fn matches(&self, asset: &Asset) -> bool {
        if let Some(s_rid) = self.rid.as_ref() {
            if s_rid != asset.rid() {
                return false;
            }
        }

        let props = &asset.properties;
        if let Some(s_name) = self.name.as_ref() {
            if s_name != &props.name {
                return false;
            }
        }

        if let Some(s_kind) = self.kind.as_ref() {
            if s_kind != &props.kind {
                return false;
            }
        }

        if let Some(s_tags) = self.tags.as_ref() {
            for s_tag in s_tags {
                if !props.tags.contains(s_tag) {
                    return false;
                }
            }
        }

        if let Some(s_md) = self.metadata.as_ref() {
            for (s_key, s_val) in s_md {
                let Some(f_val) = props.metadata.get(s_key) else {
                    return false;
                };

                // only compare number values, not types
                if f_val.is_number() && s_val.is_number() {
                    if f_val.as_f64() != s_val.as_f64() {
                        return false;
                    }
                } else {
                    if f_val != s_val {
                        return false;
                    }
                }
            }
        }

        // all search criteria matched
        true
    }
}

#[cfg(test)]
#[path = "./search_filter_test.rs"]
mod search_filter_test;
