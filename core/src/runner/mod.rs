//! Functionality for running Syre projects.
pub mod common;
pub mod env;
pub mod runner;
pub mod tree;

pub use env::{CONTAINER_ID_KEY, PROJECT_ID_KEY};
pub use runner::{error, AnalysisExecutionContext, Builder, ErrorResponse, Runner, RunnerHooks};
pub use tree::Tree;

use crate::types::ResourceId;
use has_id::HasId;

pub trait Runnable: HasId<Id = ResourceId> {
    fn command(&self) -> std::process::Command;
}
