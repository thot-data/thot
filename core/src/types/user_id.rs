use super::resource_id::ResourceId;
use std::error;
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;
use validator::ValidateEmail;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// **************
// *** Errors ***
// **************

#[derive(Debug)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ParseError: {}", self.0)
    }
}

impl error::Error for ParseError {}

// **************
// *** UserId ***
// **************

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Eq)]
pub enum UserId {
    Email(String),
    Id(ResourceId),
}

impl UserId {
    /// Parses a string to a UserId.
    /// First attempts to parse as a uuid, then validate as an email.
    /// If neither is successful an error is returned.
    pub fn from_string(id: String) -> Result<UserId, ParseError> {
        // attempt to parse as uuid
        if let Ok(uid) = Uuid::parse_str(&id) {
            return Ok(UserId::Id(ResourceId::from(uid)));
        }

        // validate as email
        if id.validate_email() {
            return Ok(UserId::Email(id));
        }

        Err(ParseError(String::from("Invalid id")))
    }
}

impl PartialEq for UserId {
    fn eq(&self, other: &UserId) -> bool {
        match (self, other) {
            (UserId::Email(me), UserId::Email(you)) => me == you,
            (UserId::Id(me), UserId::Id(you)) => me == you,
            _ => false,
        }
    }
}

impl From<ResourceId> for UserId {
    fn from(rid: ResourceId) -> Self {
        Self::Id(rid)
    }
}
impl FromStr for UserId {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        UserId::from_string(String::from(s))
    }
}

#[cfg(test)]
#[path = "./user_id_test.rs"]
mod user_id_test;
