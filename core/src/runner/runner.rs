//! Syre project runner.
use super::{Runnable, CONTAINER_ID_KEY, PROJECT_ID_KEY};
use crate::{graph::ResourceTree, project::Container, types::ResourceId};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    result::Result as StdResult,
    sync::Arc,
    {process, str},
};

type Result<T = ()> = std::result::Result<T, Error>;
type ContainerTree = ResourceTree<Container>;

/// Identifies the context in which an analysis was run.
#[derive(Clone, Debug)]
pub struct AnalysisExecutionContext {
    /// Project id.
    pub project: ResourceId,

    /// Analysis being executed.
    pub analysis: ResourceId,

    /// [`Container`] the analysis was executed on.
    pub container: PathBuf,
}

pub trait RunnerHooks {
    /// Retrieve an analysis from its [`ResourceId`].
    fn get_analysis(
        &self,
        project: ResourceId,
        analysis: ResourceId,
    ) -> StdResult<Box<dyn Runnable + Send + Sync>, String>;

    /// Run when an analysis errors.
    /// Should return `Ok` if evaluation should continue, or
    /// `Err` to defer to the `ignore_errors` state of the execution.
    ///
    /// # Notes
    /// + Default implmentation ignores errors.
    ///
    /// # See also
    /// [`Runner::run_analyses`].
    #[allow(unused_variables)]
    fn analysis_error(&self, ctx: AnalysisExecutionContext, err: Error) -> Result {
        Ok(())
    }

    /// Runs before every analysis.
    #[allow(unused_variables)]
    fn pre_analysis(&self, ctx: AnalysisExecutionContext) {}

    /// Run after an analysis exits successfully and evaluation will continue.
    /// i.e. This handle does not run if the analysis errors and the error is
    /// not successfully handled by `analysis_error` or ignored.
    #[allow(unused_variables)]
    fn post_analysis(&self, ctx: AnalysisExecutionContext) {}

    /// Run after an analysis finishes.
    /// This runs before `post_analysis` and regardless of the success of the analysis.
    #[allow(unused_variables)]
    fn assets_added(&self, ctx: AnalysisExecutionContext, assets: Vec<ResourceId>) {}
}

pub struct Builder {
    hooks: Arc<dyn RunnerHooks + Send + Sync>,
    num_threads: Option<usize>,
}

impl Builder {
    pub fn new(hooks: impl RunnerHooks + Send + Sync + 'static) -> Self {
        Self {
            hooks: Arc::new(hooks),
            num_threads: None,
        }
    }

    /// Set the number of threads the runner should use.
    pub fn num_threads(&mut self, num_threads: usize) -> &mut Self {
        let _ = self.num_threads.insert(num_threads);
        self
    }

    pub fn build(self) -> Runner {
        let mut thread_pool =
            rayon::ThreadPoolBuilder::new().thread_name(|_id| format!("syre runner thread"));
        if let Some(max_tasks) = self.num_threads {
            thread_pool = thread_pool.num_threads(max_tasks);
        }

        Runner {
            hooks: self.hooks,
            thread_pool: thread_pool.build().unwrap(),
        }
    }
}

/// # Notes
/// + All analyses launched from a single runner share a thread pool for evaluation.
pub struct Runner {
    hooks: Arc<dyn RunnerHooks + Send + Sync>,
    thread_pool: rayon::ThreadPool,
}

impl Runner {
    /// Analyze a tree.
    ///
    /// # Arguments
    /// 1. Container tree to evaluate.
    pub fn run(&self, project: &ResourceId, tree: &mut ContainerTree) -> Result {
        let root = tree.root().clone();
        let mut analyzer =
            TreeRunner::new(project, tree, &root, self.hooks.clone(), &self.thread_pool);
        analyzer.run()
    }

    /// Analyze a subtree.
    ///
    /// # Arguments
    /// 1. Container tree to evaluate.
    /// 2. Root of subtree to evaluate.
    pub fn from(
        &self,
        project: &ResourceId,
        tree: &mut ContainerTree,
        root: &ResourceId,
    ) -> Result {
        let mut analyzer =
            TreeRunner::new(project, tree, root, self.hooks.clone(), &self.thread_pool);
        analyzer.run()
    }
}

struct TreeRunner<'a> {
    project: &'a ResourceId,
    tree: &'a mut ContainerTree,
    root: &'a ResourceId,
    thread_pool: &'a rayon::ThreadPool,
    error_response: ErrorResponse,
    hooks: Arc<dyn RunnerHooks + Send + Sync>,
}

impl<'a> TreeRunner<'a> {
    pub fn new(
        project: &'a ResourceId,
        tree: &'a mut ContainerTree,
        root: &'a ResourceId,
        hooks: Arc<dyn RunnerHooks + Send + Sync>,
        thread_pool: &'a rayon::ThreadPool,
    ) -> Self {
        Self {
            project,
            tree,
            root,
            hooks,
            thread_pool,
            error_response: ErrorResponse::default(),
        }
    }

    pub fn run(&mut self) -> Result {
        let mut analysis_ids = self
            .tree
            .descendants(&self.root)
            .unwrap()
            .into_iter()
            .flat_map(|id| {
                let container = self.tree.get(&id).unwrap();
                container
                    .analyses
                    .iter()
                    .map(|association| association.analysis())
            })
            .collect::<Vec<_>>();

        analysis_ids.sort();
        analysis_ids.dedup();
        let (analyses, analysis_errors): (Vec<_>, Vec<_>) = analysis_ids
            .into_iter()
            .map(|id| {
                (
                    id,
                    self.hooks.get_analysis(self.project.clone(), id.clone()),
                )
            })
            .partition(|(_, result)| result.is_ok());

        if !analysis_errors.is_empty() {
            let errors = analysis_errors
                .into_iter()
                .map(|(id, err)| (id.clone(), err.err().unwrap()))
                .collect();
            return Err(Error::LoadAnalyses(errors));
        }

        let analyses = analyses
            .into_iter()
            .map(|(id, analysis)| (id.clone(), analysis.ok().unwrap()))
            .collect();
        self.evaluate_tree(self.root, Arc::new(analyses))
    }

    /// Evaluates a `Container` tree.
    ///
    /// # Arguments
    /// 1. Container tree to evaluate.
    /// 2. Root of subtree.
    /// 3. Maximum number of analysis tasks to run at once.
    #[tracing::instrument(skip(self, analyses))]
    fn evaluate_tree(
        &self,
        root: &ResourceId,
        analyses: Arc<HashMap<ResourceId, Box<dyn Runnable + Send + Sync>>>,
    ) -> Result {
        // recurse on children
        let Some(children) = self.tree.children(root).cloned() else {
            return Err(Error::ContainerNotFound(root.clone()));
        };

        if !children.is_empty() {
            self.thread_pool.install({
                let analyses = analyses.clone();
                move || {
                    children
                        .par_iter()
                        .map(|child| self.evaluate_tree(child, analyses.clone()))
                        .collect::<Vec<_>>()
                }
            });
        }

        self.evaluate_container(root, analyses)
    }

    /// Evaluates a single container.
    ///
    /// # Arguments
    /// 1. The [`ContainerTree`].
    /// 1. The [`Container`] to evaluate.
    /// 2. `None` to run all analyses set to `autorun`.
    ///     Otherwise a [`HashSet`] of the analyses to run.
    /// + `ignore_errors`: Whether to continue running on a analysis error.
    #[tracing::instrument(skip(self, analyses))]
    fn evaluate_container(
        &self,
        container: &ResourceId,
        analyses: Arc<HashMap<ResourceId, Box<dyn Runnable + Send + Sync>>>,
    ) -> Result {
        let Some(container) = self.tree.get(container) else {
            return Err(Error::ContainerNotFound(container.clone()));
        };

        // batch and sort analyses by priority
        let mut analysis_groups = HashMap::new();
        for association in container.analyses.iter() {
            let group = analysis_groups
                .entry(association.priority)
                .or_insert(vec![]);

            group.push(association);
        }

        let mut analysis_groups = analysis_groups.into_iter().collect::<Vec<_>>();
        analysis_groups.sort_by(|(p0, _), (p1, _)| p0.cmp(p1));

        for (_priority, analysis_group) in analysis_groups {
            let analyses = analysis_group
                .into_iter()
                .filter(|s| s.autorun)
                .map(|assoc| analyses.get(assoc.analysis()).unwrap())
                .collect();

            let container_path = self.tree.path(container.rid()).unwrap();
            self.run_analysis_group(analyses, container_path, self.project.clone())?;
        }

        Ok(())
    }

    #[cfg_attr(doc, aquamarine::aquamarine)]
    /// Runs a group of analyses.
    ///
    /// ```mermaid
    ///flowchart TD
    ///    %% happy path
    ///    run_analyses("run_analyses(analyses: Vec&ltAnalysis&gt;, container: Container, ...)") -- "for analysis in analyses" --> pre_analysis("pre_analysis(ctx: AnalysisExecutionContext)")
    ///    pre_analysis --> run_analyses("run_analyses(analysis: Analysis, container: Container, ...)")
    ///    run_analysis -- "Result&lt;Ok, Err&gt;" --> assets_added("assets_added(AnalysisExecutionContext, assets: HashSet<RerourceId>, verboes: bool)")
    ///    assets_added -- "Ok(())" --> post_analysis("post_analysis(ctx: AnalysisExecutionContext)")
    ///    post_analysis --> pre_analysis
    ///    post_analysis -- "complete" --> exit("Ok(())")

    ///    %% error path
    ///    assets_added -- "Err(Error)" --> analysis_error("analysis_error(ctx: AnalysisExecutionContext, err: Error)")
    ///    analysis_error -- "Ok(())" --> post_analysis
    ///    analysis_error -- "Err(_)" --> ignore_errors("ignore_errors")
    ///    ignore_errors -- "true" --> post_analysis
    ///    ignore_errors -- "false" ---> break("return Err(_)")
    /// ```
    #[tracing::instrument(skip(self, analyses))]
    fn run_analysis_group(
        &self,
        analyses: Vec<&Box<dyn Runnable + Send + Sync>>,
        container: PathBuf,
        project: ResourceId,
    ) -> Result {
        self.thread_pool.install(move || {
            analyses
                .into_par_iter()
                .map(|analysis| {
                    let exec_ctx = AnalysisExecutionContext {
                        project: project.clone(),
                        analysis: analysis.id().clone(),
                        container: container.clone(),
                    };

                    self.hooks.pre_analysis(exec_ctx.clone());
                    let assets = Vec::new(); // TODO: Collect `ResourceId`s of `Assets`.
                    let mut ignored_errors = Vec::new();
                    let result = self
                        .run_analysis(analysis, container.clone(), project.clone())
                        .map_err(|err| {
                            self.hooks
                                .analysis_error(exec_ctx.clone(), err)
                                .or_else(|err| match self.error_response {
                                    ErrorResponse::Terminate => {
                                        tracing::trace!(
                                            "terminating analysis due to error: {err:?}"
                                        );
                                        Err(err)
                                    }
                                    ErrorResponse::Ignore => {
                                        ignored_errors.push(err);
                                        Ok(())
                                    }
                                })
                        });
                    self.hooks.assets_added(exec_ctx.clone(), assets);

                    if result.is_ok() {
                        self.hooks.post_analysis(exec_ctx);
                    }
                })
                .collect::<Vec<_>>()
        });

        Ok(())
    }

    /// Runs an individual analysis.
    ///
    /// # Returns
    /// [`Output`](process:Output) from the analysis.
    ///
    /// # Errors
    /// + [`Error`]: The analysis returned a `status` other than `0`.
    #[tracing::instrument(skip(self, analysis))]
    fn run_analysis(
        &self,
        analysis: &Box<dyn Runnable + Send + Sync>,
        container: PathBuf,
        project: ResourceId,
    ) -> Result<process::Output> {
        tracing::trace!("running {} on {:?}", analysis.id(), container);
        let mut out = analysis.command();
        let out = match out
            .env(PROJECT_ID_KEY, project.to_string())
            .env(CONTAINER_ID_KEY, &container)
            .output()
        {
            Ok(out) => out,
            Err(err) => {
                tracing::error!(?err);
                return Err(Error::CommandError {
                    project: project.clone(),
                    analysis: analysis.id().clone(),
                    container,
                    cmd: format!("{out:?}"),
                }
                .into());
            }
        };

        tracing::trace!(?out);
        if out.status.success() {
            Ok(out)
        } else {
            let stderr = str::from_utf8(out.stderr.as_slice()).unwrap().to_string();
            return Err(Error::AnalysisError {
                project: project.clone(),
                analysis: analysis.id().clone(),
                container,
                description: stderr,
            }
            .into());
        }
    }
}

pub enum ErrorResponse {
    /// Terminate all analyses on first error.
    Terminate,

    /// Collect errors, but continue running.
    Ignore,
}

impl Default for ErrorResponse {
    fn default() -> Self {
        Self::Terminate
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0:?}")]
    LoadAnalyses(HashMap<ResourceId, String>),

    /// The `Container` could not be found in the graph.
    #[error("Container {0} not found")]
    ContainerNotFound(ResourceId),

    /// An error occured when running the analysis
    /// on the specified `Container`.
    #[error("Analysis `{analysis}` running over Container `{container}` in project `{project}` errored: {description}")]
    AnalysisError {
        project: ResourceId,
        analysis: ResourceId,
        container: PathBuf,
        description: String,
    },

    #[error("error running `{cmd}` from analysis `{analysis}` on container `{container}` in project `{project}`")]
    CommandError {
        project: ResourceId,
        analysis: ResourceId,
        container: PathBuf,
        cmd: String,
    },
}

#[cfg(test)]
#[path = "runner_test.rs"]
mod runner_test;
