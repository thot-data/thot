use serde::Serialize;
use std::path::PathBuf;
use syre_core::types::ResourceId;
use syre_local::project::container;

/// Create a new child container at the given path.
/// The path should be absolute from the project's data root.
pub async fn create_child(
    project: ResourceId,
    path: impl Into<PathBuf>,
) -> Result<ResourceId, container::error::Build> {
    #[derive(Serialize)]
    struct CreateChildArgs {
        project: ResourceId,
        path: PathBuf,
    }

    tauri_sys::core::invoke_result::<ResourceId, container::error::Build>(
        "create_child_container",
        CreateChildArgs {
            project,
            path: path.into(),
        },
    )
    .await
    .map_err(|err| match err {
        container::error::Build::Load | container::error::Build::NotADirectory => unreachable!(),
        container::error::Build::Save(_) | container::error::Build::AlreadyResource => err,
    })
}
