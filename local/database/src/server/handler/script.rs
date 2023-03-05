//! Handle `Script` related functionality.
use super::super::Database;
use crate::command::ScriptCommand;
use crate::Result;
use serde_json::Value as JsValue;
use settings_manager::{LocalSettings, SystemSettings};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use thot_core::error::{Error as CoreError, ProjectError, ResourceError};
use thot_core::project::Script as CoreScript;
use thot_core::types::{ResourceId, ResourcePath};
use thot_local::project::resources::{
    Container as LocalContainer, Script as LocalScript, Scripts as ProjectScripts,
};
use thot_local::system::collections::Projects;
use thot_local::types::ResourceValue;

impl Database {
    pub fn handle_command_script(&mut self, cmd: ScriptCommand) -> JsValue {
        match cmd {
            ScriptCommand::Get(script) => {
                let script = self.store.get_script(&script);
                serde_json::to_value(script.clone()).expect("could not convert `Script` to JsValue")
            }

            ScriptCommand::Add(project, script) => {
                let script = self.add_script(project, script);
                serde_json::to_value(script).expect("could not convert `Script` to JsValue")
            }

            ScriptCommand::Remove(project, script) => {
                let res = self.remove_script(&project, &script);
                serde_json::to_value(res).expect("could not convert to JsValue")
            }

            ScriptCommand::Update(script) => {
                let res = self.update_script(script);
                serde_json::to_value(res).expect("could not convert result to JsValue")
            }

            ScriptCommand::LoadProject(project) => {
                let scripts = self.load_project_scripts(project);
                serde_json::to_value(scripts).expect("could not convert result to JsValue")
            }
        }
    }

    /// Loads a `Project`'s `Scripts`.
    ///
    /// # Arguments
    /// 1. `Project`'s id.
    fn load_project_scripts(&mut self, rid: ResourceId) -> Result<Vec<CoreScript>> {
        if let Some(scripts) = self.store.get_project_scripts(&rid) {
            // project scripts already loaded
            let scripts = (*scripts).clone().into_values().collect();
            return Ok(scripts);
        }

        let projects = Projects::load()?;
        let Some(project) = projects.get(&rid).clone() else {
            return Err(CoreError::ResourceError(ResourceError::DoesNotExist("`Project` does not exist".to_string())).into());
        };

        let scripts = ProjectScripts::load(&project.path)?;
        let script_vals = (*scripts).clone().into_values().collect();
        self.store.insert_project_scripts(rid, scripts);

        Ok(script_vals)
    }

    /// Adds a `Script` to a `Project`.
    fn add_script(&mut self, project: ResourceId, script: PathBuf) -> Result<CoreScript> {
        let script = LocalScript::new(ResourcePath::new(script)?)?;
        self.store.insert_script(project, script.clone())?;

        Ok(script)
    }

    /// Remove `Script` from `Project`.
    fn remove_script(&mut self, pid: &ResourceId, script: &ResourceId) -> Result {
        fn remove_recursively_script_association(
            container: Arc<Mutex<LocalContainer>>,
            script: &ResourceId,
        ) -> Result {
            let mut container = container.lock().expect("could not lock `Container`");
            container.scripts.remove(script);
            container.save()?;

            for child in container.children.values() {
                let ResourceValue::Resource(child) = child else { 
                    return Err(CoreError::ResourceError(ResourceError::DoesNotExist("child `Container` does not exist".to_string())).into())};
                remove_recursively_script_association(child.clone(), script)?;
            }
            Ok(())
        }

        // Get root container
        let Some(project) = self.store.get_project(pid) else { 
            return Err(CoreError::ResourceError(ResourceError::DoesNotExist("`Project` does not exist".to_string())).into()) };
        let Some(data_root) = project.data_root.as_ref() else { 
            return Err(CoreError::ProjectError(ProjectError::Misconfigured("`data_root` not set".to_string())).into())};
        let Some(root_container) = self.store.get_path_container(&data_root) else { 
            return Err(CoreError::ResourceError(ResourceError::DoesNotExist("`Container` does not exist".to_string())).into())};
        let Some(root_container) = self.store.get_container(root_container) else { 
            return Err(CoreError::ResourceError(ResourceError::DoesNotExist("`Container` does not exist".to_string())).into())};

        // Remove script associations
        remove_recursively_script_association(root_container.clone(), script)?;

        // Remove project script
        self.store.remove_script(pid, script)?;

        Ok(())
    }

    /// Update a `Script`.
    fn update_script(&mut self, script: CoreScript) -> Result {
        let Some(project) = self.store.get_script_project(&script.rid) else {
            return Err(CoreError::ResourceError(ResourceError::DoesNotExist("`Script` does not exist".to_string())).into());
        };

        self.store.insert_script(project.clone(), script)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "./script_test.rs"]
mod script_test;
