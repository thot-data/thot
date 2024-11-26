//! Local runner hooks.
use std::{
    path::{Path, PathBuf},
    process,
};
use syre_core::{
    self as core,
    project::{ExcelTemplate, Script, ScriptLang},
    runner::{Runnable, RunnerHooks},
    types::ResourceId,
};
use syre_local::{system::config, types::analysis::AnalysisKind};
use syre_local_database as db;

pub struct Builder<'a> {
    path: &'a dyn AsRef<Path>,
    project: &'a db::state::ProjectData,
    settings: Option<&'a config::runner_settings::Settings>,
    ignore_errors: bool,
}

impl<'a> Builder<'a> {
    /// # Arguments
    /// `path`: Path to the projects base directory.
    pub fn new(
        path: &'a impl AsRef<Path>,
        project: &'a db::state::ProjectData,
        settings: Option<&'a config::runner_settings::Settings>,
    ) -> Self {
        Self {
            path,
            project,
            settings,
            ignore_errors: false,
        }
    }

    pub fn ignore_errors(&mut self) -> &mut Self {
        self.ignore_errors = true;
        self
    }

    pub fn build(self) -> Result<Runner, error::From> {
        let analyses = Self::create_analyses(self.path, self.project, self.settings)?;
        Ok(Runner {
            analyses,
            ignore_errors: self.ignore_errors,
        })
    }

    /// # Returns
    /// List of `(id, analysis)`.
    fn create_analyses(
        path: impl AsRef<Path>,
        project_data: &db::state::ProjectData,
        settings: Option<&config::runner_settings::Settings>,
    ) -> Result<Vec<(ResourceId, AnalysisKind)>, error::From> {
        let db::state::DataResource::Ok(analyses) = project_data.analyses() else {
            return Err(error::From::InvalidAnalysesState);
        };

        let db::state::DataResource::Ok(properties) = project_data.properties() else {
            return Err(error::From::InvalidPropertiesState);
        };

        let Some(analysis_root) = properties.analysis_root.as_ref() else {
            return Err(error::From::NoAnalysisRoot);
        };
        let analysis_root = path.as_ref().join(analysis_root);

        let map = analyses
            .clone()
            .into_iter()
            .map(|analysis| match analysis.properties() {
                AnalysisKind::Script(script) => {
                    let script =
                        Self::create_analysis_script(script.clone(), &analysis_root, settings);
                    (script.rid().clone(), AnalysisKind::Script(script))
                }
                AnalysisKind::ExcelTemplate(template) => {
                    let template = Self::create_analysis_excel_template(
                        template.clone(),
                        &analysis_root,
                        settings,
                    );
                    (
                        template.rid().clone(),
                        AnalysisKind::ExcelTemplate(template),
                    )
                }
            })
            .collect();

        Ok(map)
    }

    /// Modifies the given analysis script for the runner.
    fn create_analysis_script(
        mut script: Script,
        analysis_root: &PathBuf,
        runner_settings: Option<&config::runner_settings::Settings>,
    ) -> Script {
        if script.path.is_relative() {
            script.path = analysis_root.join(script.path);
        } else if script.path.is_absolute() {
            todo!();
        } else {
            todo!();
        }

        if let Some(runner_settings) = runner_settings {
            match script.env.language {
                ScriptLang::Python => {
                    if let Some(python_path) = runner_settings.python_path.clone() {
                        script.env.cmd = python_path.to_string_lossy().to_string();
                    }
                }

                ScriptLang::R => {
                    if let Some(r_path) = runner_settings.r_path.clone() {
                        script.env.cmd = r_path.to_string_lossy().to_string();
                    }
                }
            }
        };

        script
    }

    /// Modifies the given analysis script for the runner.
    fn create_analysis_excel_template(
        mut template: ExcelTemplate,
        analysis_root: &PathBuf,
        runner_settings: Option<&config::runner_settings::Settings>,
    ) -> ExcelTemplate {
        if template.template.path.is_relative() {
            template.template.path = analysis_root.join(template.template.path);
        } else if template.template.path.is_absolute() {
            todo!();
        } else {
            todo!();
        }

        if let Some(runner_settings) = runner_settings {
            if let Some(python_path) = runner_settings.python_path.clone() {
                template.python_exe = python_path;
            }
        }

        template
    }
}

#[derive(Debug)]
pub struct Runner {
    analyses: Vec<(ResourceId, AnalysisKind)>,
    ignore_errors: bool,
}

impl Runner {
    pub fn new(analyses: Vec<(ResourceId, AnalysisKind)>, ignore_errors: bool) -> Self {
        Self {
            analyses,
            ignore_errors,
        }
    }
}

impl RunnerHooks for Runner {
    /// Retrieves a local [`Script`](CoreScript) given its [`ResourceId`].
    fn get_analysis(
        &self,
        project: ResourceId,
        analysis: ResourceId,
    ) -> Result<Box<dyn Runnable + Send + Sync>, String> {
        self.analyses
            .iter()
            .find_map(|(id, runner_analysis)| {
                if *id == analysis {
                    let analysis = match runner_analysis {
                        AnalysisKind::Script(script) => {
                            Box::new(script.clone()) as Box<dyn Runnable + Send + Sync>
                        }
                        AnalysisKind::ExcelTemplate(template) => {
                            Box::new(template.clone()) as Box<dyn Runnable + Send + Sync>
                        }
                    };
                    Some(analysis)
                } else {
                    None
                }
            })
            .ok_or(format!("could not find analysis {analysis}"))
    }

    fn analysis_error(
        &self,
        ctx: &core::runner::AnalysisExecutionContext,
        exit_code: i32,
        err: &str,
    ) -> core::runner::ErrorResponse {
        if self.ignore_errors {
            core::runner::ErrorResponse::Continue
        } else {
            core::runner::ErrorResponse::Terminate
        }
    }
}

pub mod error {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug)]
    pub enum From {
        /// Project analyses are not in a valid state.
        InvalidAnalysesState,

        /// Project properties are not in a valid state.
        InvalidPropertiesState,

        // The Project's analysis root is not set.
        NoAnalysisRoot,
    }
}
