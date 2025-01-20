//! Functionality for running Syre projects.
pub mod common;
pub mod runner;
pub mod tree;

pub use env::{ANALYSIS_ID_KEY, CONTAINER_ID_KEY, PROJECT_ID_KEY};
pub use runner::{error, AnalysisExecutionContext, Builder, ErrorResponse, Runner, RunnerHooks};
pub use tree::Tree;

use crate::types::ResourceId;
use has_id::HasId;

pub trait Runnable: HasId<Id = ResourceId> {
    fn command(&self) -> std::process::Command;
}

mod env {
    //! Environment variables for runner.
    pub static PROJECT_ID_KEY: &str = "SYRE_PROJECT_ID";
    pub static CONTAINER_ID_KEY: &str = "SYRE_CONTAINER_ID";
    pub static ANALYSIS_ID_KEY: &str = "SYRE_ANALYSIS_ID";
}
