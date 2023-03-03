//! Projects state.
// use crate::commands::settings::UserAppStateArgs;
use indexmap::IndexSet;
use std::rc::Rc;
use thot_core::project::{Project as CoreProject, Scripts as ProjectScripts};
use thot_core::types::{ResourceId, ResourceMap};
use yew::prelude::*;

pub type ProjectMap = ResourceMap<CoreProject>;

/// Map from a `Project` to its `Scripts`.
pub type ProjectScriptsMap = ResourceMap<ProjectScripts>;

/// Actions for [`ProjectsState`].
pub enum ProjectsStateAction {
    /// Insert a project.
    InsertProject(CoreProject),

    /// Inserts multiple projects.
    InsertProjects(Vec<CoreProject>),

    /// Add an open project.
    AddOpenProject(ResourceId),

    /// Remove an open project.
    ///
    /// # Fields
    /// 1. Project to remove.
    /// 2. New project to set as active, if needed.
    RemoveOpenProject(ResourceId, Option<ResourceId>),

    /// Set the active `Project`.
    SetActiveProject(ResourceId),

    /// Update the [`Project`](CoreProject).
    UpdateProject(CoreProject),

    /// Insert a `Script`.
    ///
    /// # Fields
    /// 1. `Project`'s id.
    /// 2. `Project`'s `Script`s.
    InsertProjectScripts(ResourceId, ProjectScripts),

    /// Update a `Project` `Script`s.
    ///
    /// # Fields
    /// 1. `Project`'s id.
    /// 2. `Project`'s `Script`s.
    UpdateProjectScripts(ResourceId, ProjectScripts),
}

#[derive(Debug, Default, PartialEq, Clone)]
/// Maintains the state of application [`Projects`].
pub struct ProjectsState {
    /// All user [`Projects`].
    pub projects: ProjectMap,

    /// `Project` `Script`s.
    pub project_scripts: ProjectScriptsMap,

    /// Open [`Projects`].
    pub open_projects: IndexSet<ResourceId>,

    /// The active [`Project`].
    pub active_project: Option<ResourceId>,
}

impl Reducible for ProjectsState {
    type Action = ProjectsStateAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut current = (*self).clone();
        match action {
            ProjectsStateAction::InsertProject(project) => {
                current.projects.insert(project.rid.clone(), project);
            }

            ProjectsStateAction::InsertProjects(projects) => {
                for project in projects {
                    current.projects.insert(project.rid.clone(), project);
                }
            }

            ProjectsStateAction::AddOpenProject(project) => {
                current.open_projects.insert(project);
            }

            ProjectsStateAction::RemoveOpenProject(closing, next) => {
                if current.active_project.as_ref() == Some(&closing) {
                    // closed the active project
                    current.active_project = next;
                }

                current.open_projects.remove(&closing);
            }

            ProjectsStateAction::SetActiveProject(rid) => {
                current.active_project = Some(rid);
            }

            ProjectsStateAction::InsertProjectScripts(project, scripts) => {
                current.project_scripts.insert(project.clone(), scripts);
            }

            ProjectsStateAction::UpdateProjectScripts(project, scripts) => {
                current.project_scripts.insert(project.clone(), scripts);
            }

            ProjectsStateAction::UpdateProject(project) => {
                current.projects.insert(project.rid.clone(), project);
            }
        }

        current.into()
    }
}

pub type ProjectsStateReducer = UseReducerHandle<ProjectsState>;

#[cfg(test)]
#[path = "./projects_state_test.rs"]
mod projects_state_test;
