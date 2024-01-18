use super::event::thot::Project as ProjectEvent;
use crate::error::server::LoadUserProjects as LoadUserProjectsError;
use crate::event::{Project as ProjectUpdate, Update};
use crate::server::types::ProjectResources;
use crate::server::Database;
use crate::{Error, Result};
use thot_local::system::collections::projects::Projects;

impl Database {
    pub fn handle_thot_event_project(&mut self, event: ProjectEvent) -> Result {
        match event {
            ProjectEvent::Removed(project) => {
                let ProjectResources { project, graph: _ } = self.store.remove_project(&project);
                if let Some(project) = project {
                    let mut project_manifest = match Projects::load_or_default() {
                        Ok(project_manifest) => project_manifest,
                        Err(err) => {
                            return Err(Error::Database(format!(
                                "{:?}",
                                LoadUserProjectsError::LoadProjectsManifest(err)
                            )))
                        }
                    };

                    project_manifest.remove(&project.rid);
                    project_manifest.save()?;

                    self.publish_update(&Update::Project {
                        project: project.rid.clone(),
                        update: ProjectUpdate::Removed(Some(project.into())),
                    })?;
                }

                Ok(())
            }
        }
    }
}
