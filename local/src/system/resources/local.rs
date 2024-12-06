//! Local config.
use serde::{Deserialize, Serialize};
use syre_core::types::ResourceId;

/// Local config data.
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Config {
    /// Active user.
    pub user: Option<ResourceId>,
}
