//! A [`Project`](crate::project::Project) template.
use super::ResourceTree;
use crate::graph::ResourceTree as GraphTree;
use crate::project::Project as PrjProject;
use crate::types::{ResourceId, UserPermissions};
use chrono::prelude::*;
use has_id::HasId;
use has_id::HasIdSerde;
use serde::{Deserialize, Serialize};
use serde_json::{Result as SerdeResult, Value as JsValue};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// ********************
// *** Project Info ***
// ********************

#[derive(Serialize, Deserialize, Default)]
pub struct ProjectInfo {
    pub description: Option<String>,
    pub data_root: Option<PathBuf>,
    pub universal_root: Option<PathBuf>,
    pub analysis_root: Option<PathBuf>,
}

impl ProjectInfo {
    pub fn new() -> Self {
        Self::default()
    }
}

impl From<PrjProject> for ProjectInfo {
    fn from(project: PrjProject) -> Self {
        Self {
            description: project.description,
            data_root: project.data_root,
            universal_root: project.universal_root,
            analysis_root: project.analysis_root,
        }
    }
}

// ************************
// *** Project Template ***
// ************************

#[derive(HasId, Debug, Serialize, Deserialize, HasIdSerde)]
pub struct Project {
    #[id]
    pub rid: ResourceId,

    /// User id of the creator.
    pub creator: Option<ResourceId>,
    pub created: DateTime<Utc>,
    pub permissions: HashMap<ResourceId, UserPermissions>,

    pub name: String,
    pub description: Option<String>,

    /// Project info.
    pub project: ProjectInfo,

    // graph should be stored separately and loaded in
    #[serde(skip)]
    graph: JsValue,

    /// Projects derived from the template.
    pub children: HashSet<ResourceId>,
}

impl Project {
    pub fn new<T>(
        project: ProjectInfo,
        graph: GraphTree<T>,
        name: String,
        path: PathBuf,
    ) -> SerdeResult<Self>
    where
        T: Serialize + HasId<Id = ResourceId>,
    {
        let graph = ResourceTree::to_value(graph)?;

        Ok(Self {
            rid: ResourceId::new(),
            creator: None,
            created: Utc::now(),
            permissions: HashMap::new(),
            name,
            description: None,
            project,
            graph,
            children: HashSet::new(),
        })
    }

    /// Creates a new [`Project`](crate::project::Project) from the template.
    pub fn create_project<T>(&self, path: PathBuf) -> SerdeResult<(PrjProject, GraphTree<T>)>
    where
        T: HasId<Id = ResourceId>,
    {
        let mut project = PrjProject::new(&self.name);
        project.description = self.project.description.clone();
        project.data_root = self.project.data_root.clone();
        project.universal_root = self.universal_root.clone();
        project.analysis_root = self.project.analysis_root.clone();

        let graph = ResourceTree::to_tree(graph);
    }
}

#[cfg(test)]
#[path = "./project_test.rs"]
mod project_test;
