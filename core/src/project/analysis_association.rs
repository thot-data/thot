use crate::types::ResourceId;
use std::cmp::{Eq, Ordering, PartialEq};

pub type Priority = i32;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Represents an association between an analysis  and a Container.
/// Contains information on the analysis to run,
/// whether the analysis should be run,
/// and the order of its execution relative to the current Container.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct AnalysisAssociation {
    analysis: ResourceId,
    pub autorun: bool,
    pub priority: Priority,
}

impl AnalysisAssociation {
    /// Creates a new analysis association with
    /// `autorun` `true` and `priority` `0`.
    pub fn new(analysis: ResourceId) -> Self {
        AnalysisAssociation {
            analysis,
            autorun: true,
            priority: 0,
        }
    }

    pub fn with_params(analysis: ResourceId, autorun: bool, priority: Priority) -> Self {
        AnalysisAssociation {
            analysis,
            autorun,
            priority,
        }
    }

    pub fn analysis(&self) -> &ResourceId {
        &self.analysis
    }
}

impl PartialOrd for AnalysisAssociation {
    /// Ordering is based on the `priority` field.
    /// If the `priority` fields are equal and `autorun` state is equal,
    /// results in the two objects being equal.
    /// If the `priority` fields are equal, but the `autorun` fields are not
    /// equal, results in the two being uncomparable.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let me = self.priority;
        let you = other.priority;
        if me < you {
            return Some(Ordering::Less);
        } else if me > you {
            return Some(Ordering::Greater);
        } else if self.autorun == other.autorun {
            // priorities equal
            return Some(Ordering::Equal);
        }

        // priorities equal, autorun not
        // can not compare
        None
    }
}

#[cfg(test)]
#[path = "./analysis_association_test.rs"]
mod analysis_association_test;
