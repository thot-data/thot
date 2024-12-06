//! Resource ids.
use std::fmt::{self, Display};
use std::ops::Deref;
use std::result::Result as StdResult;
use std::str::FromStr;
use uuid::Uuid;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Holds a unique id for a resource.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResourceId(Uuid);

impl ResourceId {
    pub fn new() -> ResourceId {
        ResourceId(Uuid::new_v4())
    }
}

impl Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> StdResult<(), fmt::Error> {
        Display::fmt(&self.0, f)
    }
}

impl Deref for ResourceId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for ResourceId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        Ok(Uuid::parse_str(s)?.into())
    }
}

impl From<Uuid> for ResourceId {
    fn from(id: Uuid) -> ResourceId {
        ResourceId(id)
    }
}

impl Into<Uuid> for ResourceId {
    fn into(self) -> Uuid {
        self.0
    }
}

#[cfg(test)]
#[path = "./resource_id_test.rs"]
mod resource_id_test;
