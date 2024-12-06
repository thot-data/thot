//! Project template.
use crate::file_resource::UserResource;
use crate::system::common::config_dir_path;
use crate::Result;
use has_id::{HasId, HasIdSerde};
use std::{
    io,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    result::Result as StdResult,
};
use syre_core::graph::ResourceTree;
use syre_core::project::Project as CoreProject;
use syre_core::system::template::Project as ProjectTemplate;
use syre_core::types::ResourceId;

pub struct Project {
    rel_path: PathBuf,
    project: ProjectTemplate,
}

impl Project {
    /// Creates a new [`Project`](crate::project::Project) from the template.
    pub fn create_project<T>(&self, path: PathBuf) -> Result<(CoreProject, ResourceTree<T>)>
    where
        T: HasId<Id = ResourceId> + HasIdSerde<'static, Id = ResourceId>,
    {
        todo!();
        // let mut project = CoreProject::new(&self.name);
        // project.description = self.project.description.clone();
        // project.data_root = self.project.data_root.clone();
        // project.universal_root = self.universal_root.clone();
        // project.analysis_root = self.project.analysis_root.clone();

        // let graph = ResourceTree::to_tree(graph);

        // Ok((project, graph))
    }
}

impl Deref for Project {
    type Target = ProjectTemplate;

    fn deref(&self) -> &Self::Target {
        &self.project
    }
}

impl DerefMut for Project {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.project
    }
}

impl UserResource<ProjectTemplate> for Project {
    /// Returns the base path to the settings file.
    fn base_path() -> StdResult<PathBuf, io::Error> {
        let mut path = config_dir_path()?;
        path.push("templates");
        Ok(path)
    }

    /// Returns the relative path for the settings.
    fn rel_path(&self) -> &Path {
        &self.rel_path
    }
}
