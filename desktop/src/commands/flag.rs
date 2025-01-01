use serde::Serialize;
use std::path::PathBuf;
use syre_local as local;

/// Remove a flag from a resource.
pub async fn remove(
    project: impl Into<PathBuf>,
    container: impl Into<PathBuf>,
    resource: impl Into<PathBuf>,
    flag: local::project::resources::flag::Id,
) -> Result<(), local::error::IoSerde> {
    #[derive(Serialize)]
    struct Args {
        project: PathBuf,
        container: PathBuf,
        resource: PathBuf,
        flag: local::project::resources::flag::Id,
    }

    tauri_sys::core::invoke_result(
        "remove_flag",
        Args {
            project: project.into(),
            container: container.into(),
            resource: resource.into(),
            flag,
        },
    )
    .await
}

/// Remove all flags for a given resource.
pub async fn remove_all(
    project: impl Into<PathBuf>,
    container: impl Into<PathBuf>,
    resource: impl Into<PathBuf>,
) -> Result<(), local::error::IoSerde> {
    #[derive(Serialize)]
    struct Args {
        project: PathBuf,
        container: PathBuf,
        resource: PathBuf,
    }

    tauri_sys::core::invoke_result(
        "remove_all_flags",
        Args {
            project: project.into(),
            container: container.into(),
            resource: resource.into(),
        },
    )
    .await
}
