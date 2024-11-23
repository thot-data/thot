//! impls for Surreal DB.
use crate::types::ResourceId;
use surrealdb::value::RecordIdKey;

impl ResourceId {
    pub fn into_surreal_id(self) -> RecordIdKey {
        self.into()
    }
}

impl Into<RecordIdKey> for ResourceId {
    fn into(self) -> RecordIdKey {
        self.to_string().into()
    }
}
