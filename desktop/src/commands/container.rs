use serde::Serialize;
use std::path::PathBuf;
use syre_core::{project::AnalysisAssociation, types::ResourceId};
use syre_desktop_lib as lib;

pub async fn update_analysis_associations(
    project: ResourceId,
    container: impl Into<PathBuf>,
    associations: Vec<AnalysisAssociation>,
) -> Result<(), lib::command::container::error::Update> {
    #[derive(Serialize)]
    struct Args {
        project: ResourceId,
        container: PathBuf,
        associations: Vec<AnalysisAssociation>,
    }

    tauri_sys::core::invoke_result::<(), lib::command::container::error::Update>(
        "container_analysis_associations_update",
        Args {
            project,
            container: container.into(),
            associations,
        },
    )
    .await
}
