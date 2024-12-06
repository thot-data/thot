use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use syre_core::types::ResourceId;
use syre_local::types::FsResourceAction;

/// Info for adding a file system resource to the analysis of a project.
///
/// # Notes
/// + `path` should be absolute from the file system root.
/// + `parent` should be absolute from the analysis root.
///     i.e. The analysis root has path `/`.
#[derive(Serialize, Deserialize, Debug)]
pub struct AddFsAnalysisResourceData {
    /// Absolute path to the file system resource.
    pub path: PathBuf,

    /// Relative path within the analysis root in which to insert the resources.
    pub parent: PathBuf,
    pub action: FsResourceAction,
}

/// Info for adding a file system resource to a data graph.
///
/// # Arguments
/// + `path`: Absolute path from system root of resource.
/// + `parent`: Absolute path from data root of parent container in which to place the resource.
#[derive(Serialize, Deserialize, Debug)]
pub struct AddFsGraphResourceData {
    pub project: ResourceId,
    pub path: PathBuf,
    pub parent: PathBuf,
    pub action: FsResourceAction,
}
