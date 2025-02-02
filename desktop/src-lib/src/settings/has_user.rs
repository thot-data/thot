//! Indicates an object is associated to a user.
use thot_core::types::ResourceId;

pub trait HasUser {
    fn new(user: ResourceId) -> Self;
    fn user(&self) -> &ResourceId;
}
