//! Package constants.
pub use crate::types::PortNumber;
use std::net::Ipv4Addr;

pub const LOCALHOST: Ipv4Addr = Ipv4Addr::LOCALHOST;

/// Local port for REP-REQ communication.
pub const REQ_REP_PORT: PortNumber = 7047;

/// Local port for PUB-SUB communication.
pub const PUB_SUB_PORT: PortNumber = 7048;

/// Local port for PUB-SUB communication.
pub const DATASTORE_PORT: PortNumber = 7049;

/// PUB-SUB topic
pub const PUB_SUB_TOPIC: &str = "syre://local-database";

/// Identifier string for the database
pub const DATABASE_ID: &str = "syre local database";

pub mod pub_sub_topic {
    pub const APP_USER_MANIFEST: &str = "app/user_manifest";
    pub const APP_PROJECT_MANIFEST: &str = "app/project_manifest";
    pub const APP_LOCAL_CONFIG: &str = "app/local_config";
    pub const PROJECT_PREFIX: &str = "project";
    pub const PROJECT_UNKNOWN: &str = "project/unknown";
}
