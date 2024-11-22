//! Syre project runner.
use super::{tree, Runnable, Tree, CONTAINER_ID_KEY, PROJECT_ID_KEY};
use crate::types::ResourceId;
use core::time;
use rayon::prelude::*;
use std::{
    collections::HashMap,
    path::PathBuf,
    process, str,
    sync::{Arc, Mutex},
    thread,
};

/// Time between checking an analyzer's state.
const ANALYZER_STATE_POLL_DELAY_MS: u64 = 100;

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
    ) -> Result<Box<dyn Runnable + Send + Sync>, String>;

    /// Run when an analysis errors.
    /// Return value indicates whether the error should be ignored or not.
    ///
    /// # Notes
    /// + Default implementation terminates analyses.
    ///
    /// # See also
    /// [`Runner::run_analyses`].
    #[allow(unused_variables)]
    fn analysis_error(
        &self,
        ctx: &AnalysisExecutionContext,
        status: process::ExitStatus,
        err: String,
    ) -> ErrorResponse {
        ErrorResponse::Terminate
    }

    /// Runs before every analysis.
    #[allow(unused_variables)]
    fn pre_analysis(&self, ctx: &AnalysisExecutionContext) {}

    /// Run after an analysis exits successfully and evaluation will continue.
    /// i.e. This handle does not run if the analysis errors and the error is
    /// not successfully handled by `analysis_error` or ignored.
    #[allow(unused_variables)]
    fn post_analysis(&self, ctx: &AnalysisExecutionContext) {}

    /// Run after an analysis finishes.
    /// This runs before `post_analysis` and regardless of the success of the analysis.
    #[allow(unused_variables)]
    fn assets_added(&self, ctx: &AnalysisExecutionContext, assets: Vec<ResourceId>) {}
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
            thread_pool: Arc::new(thread_pool.build().unwrap()),
        }
    }
}

/// # Notes
/// + All analyses launched from a single runner share a thread pool for evaluation.
pub struct Runner {
    hooks: Arc<dyn RunnerHooks + Send + Sync>,
    thread_pool: Arc<rayon::ThreadPool>,
}

impl Runner {
    /// Analyze a tree.
    ///
    /// # Notes
    /// + Spawns a new process.
    pub fn run(&self, project: ResourceId, tree: Tree) -> Handle {
        let root = tree.root().rid().clone();
        Handle::run(
            project,
            tree,
            &root,
            self.hooks.clone(),
            self.thread_pool.clone(),
        )
        .unwrap()
    }

    /// Analyze a subtree.
    ///
    /// # Notes
    /// + Spawns a new process.
    pub fn from(
        &self,
        project: ResourceId,
        tree: Tree,
        root: &ResourceId,
    ) -> Result<Handle, error::ContainerNotFound> {
        Handle::run(
            project,
            tree,
            root,
            self.hooks.clone(),
            self.thread_pool.clone(),
        )
    }
}

#[derive(Debug)]
enum AnalyzerState {
    Running,
    Done,
    Cancel,
    Kill,
}

#[derive(Clone, Copy, Debug)]
pub enum AnalysisStatus {
    Pending,
    Running,
    Complete,
    Killed,
}

#[derive(Clone, Debug)]
pub struct AnalysisState {
    container: ResourceId,
    analysis: ResourceId,
    status: AnalysisStatus,
}

pub struct Handle {
    handle: thread::JoinHandle<Result<(), error::AnalyzerRun>>,
    state: Arc<Mutex<AnalyzerState>>,
    analysis_status: Arc<Mutex<Vec<AnalysisState>>>,
}

impl Handle {
    fn run(
        project: ResourceId,
        tree: Tree,
        root: &ResourceId,
        hooks: Arc<dyn RunnerHooks + Send + Sync>,
        thread_pool: Arc<rayon::ThreadPool>,
    ) -> Result<Self, error::ContainerNotFound> {
        let state = Arc::new(Mutex::new(AnalyzerState::Running));
        let mut analyzer = Analyzer::new(project, tree, root, hooks, thread_pool, state.clone())?;
        let analysis_status = analyzer.status().clone();

        // TODO: Spawn in thread pool?
        let analyzer = thread::Builder::new()
            .name("syre analyzer".to_string())
            .spawn(move || analyzer.run())
            .unwrap();

        Ok(Self {
            handle: analyzer,
            state,
            analysis_status,
        })
    }

    /// Wait for the analysis to finish.
    ///
    /// # Panics
    /// If the analysis thread panics.
    pub fn join(self) -> Result<Vec<AnalysisState>, error::AnalyzerRun> {
        self.handle.join().unwrap()?;

        let status = Arc::try_unwrap(self.analysis_status).unwrap();
        let status = status.into_inner().unwrap();
        Ok(status)
    }

    /// If no more analyses will run.
    /// This can be caused by all analyses finishing, or the process being `cancel`ed or `kill`ed.
    pub fn done(&self) -> bool {
        let state = self.state.lock().unwrap();
        matches!(*state, AnalyzerState::Done)
    }

    /// Waits for all current tasks to finish and aborts any pending tasks.
    /// Only performs an action if analyzer is running.
    pub fn cancel(&self) {
        let mut state = self.state.lock().unwrap();
        if matches!(*state, AnalyzerState::Running) {
            *state = AnalyzerState::Cancel;
        }
    }

    /// Immediately kills all current tasks, and aborts any pending tasks.
    /// Only performs an action if analyzer is running.
    pub fn kill(&self) {
        let mut state = self.state.lock().unwrap();
        if matches!(*state, AnalyzerState::Running) {
            *state = AnalyzerState::Kill;
        }
    }

    pub fn status(&self) -> Vec<AnalysisState> {
        let status = self.analysis_status.lock().unwrap();
        (*status).clone()
    }
}

struct Analyzer {
    project: ResourceId,
    tree: Tree,
    root: tree::Node,
    thread_pool: Arc<rayon::ThreadPool>,
    hooks: Arc<dyn RunnerHooks + Send + Sync>,
    state: Arc<Mutex<AnalyzerState>>,
    status: Arc<Mutex<Vec<AnalysisState>>>,
}

impl Analyzer {
    /// # Returns
    /// `Err` if `root` is not found in `tree`.
    fn new(
        project: ResourceId,
        tree: Tree,
        root: &ResourceId,
        hooks: Arc<dyn RunnerHooks + Send + Sync>,
        thread_pool: Arc<rayon::ThreadPool>,
        state: Arc<Mutex<AnalyzerState>>,
    ) -> Result<Self, error::ContainerNotFound> {
        let Some(root) = tree.nodes().iter().find(|node| node.rid() == root).cloned() else {
            return Err(error::ContainerNotFound(root.clone()));
        };
        let status = Arc::new(Mutex::new(Self::collect_analyses_to_perform(&tree)));
        Ok(Self {
            project,
            tree,
            root,
            hooks,
            thread_pool,
            state,
            status,
        })
    }

    pub fn status(&self) -> &Arc<Mutex<Vec<AnalysisState>>> {
        &self.status
    }

    fn collect_analyses_to_perform(tree: &Tree) -> Vec<AnalysisState> {
        tree.nodes()
            .iter()
            .flat_map(|node| {
                node.analyses.iter().map(|analysis| AnalysisState {
                    container: node.rid().clone(),
                    analysis: analysis.analysis().clone(),
                    status: AnalysisStatus::Pending,
                })
            })
            .collect()
    }

    pub fn run(&mut self) -> Result<(), error::AnalyzerRun> {
        let mut analysis_ids = self
            .tree
            .nodes()
            .into_iter()
            .flat_map(|container| {
                container
                    .analyses
                    .iter()
                    .map(|association| association.analysis().clone())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        analysis_ids.sort();
        analysis_ids.dedup();
        let (analyses, analysis_errors): (Vec<_>, Vec<_>) = analysis_ids
            .into_iter()
            .map(|id| {
                (
                    id.clone(),
                    self.hooks.get_analysis(self.project.clone(), id),
                )
            })
            .partition(|(_, result)| result.is_ok());

        if !analysis_errors.is_empty() {
            let errors = analysis_errors
                .into_iter()
                .map(|(id, err)| (id.clone(), err.err().unwrap()))
                .collect();

            return Err(error::AnalyzerRun::LoadAnalyses(errors));
        }

        let analyses = analyses
            .into_iter()
            .map(|(id, analysis)| (id.clone(), analysis.ok().unwrap()))
            .collect();

        let result = self
            .evaluate_tree(self.root.clone(), Arc::new(analyses))
            .map_err(|err| error::AnalyzerRun::from(err));

        let mut state = self.state.lock().unwrap();
        *state = AnalyzerState::Done;

        result
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
        root: tree::Node,
        analyses: Arc<HashMap<ResourceId, Box<dyn Runnable + Send + Sync>>>,
    ) -> Result<(), Vec<error::Evaluation>> {
        let children = self.tree.children(&root).unwrap().clone();
        if !children.is_empty() {
            let errors = self.thread_pool.install({
                let analyses = analyses.clone();
                move || {
                    children
                        .into_par_iter()
                        .filter_map(|child| self.evaluate_tree(child, analyses.clone()).err())
                        .flatten()
                        .collect::<Vec<_>>()
                }
            });

            if !errors.is_empty() {
                return Err(errors);
            }
        }

        self.evaluate_container(&root, analyses)
    }

    /// Evaluates a single container.
    #[tracing::instrument(skip(self, analyses))]
    fn evaluate_container(
        &self,
        container: &tree::Node,
        analyses: Arc<HashMap<ResourceId, Box<dyn Runnable + Send + Sync>>>,
    ) -> Result<(), Vec<error::Evaluation>> {
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

            let container_path = self.tree.path(container).unwrap();
            self.run_analysis_group(
                analyses,
                container_path,
                container.rid(),
                self.project.clone(),
            )?;
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
    /// ```
    #[tracing::instrument(skip(self, analyses))]
    fn run_analysis_group(
        &self,
        analyses: Vec<&Box<dyn Runnable + Send + Sync>>,
        container_path: PathBuf,
        container_id: &ResourceId,
        project: ResourceId,
    ) -> Result<(), Vec<error::Evaluation>> {
        let errors = self.thread_pool.install(move || {
            analyses
                .into_par_iter()
                .filter_map(|analysis| {
                    let exec_ctx = AnalysisExecutionContext {
                        project: project.clone(),
                        analysis: analysis.id().clone(),
                        container: container_path.clone(),
                    };

                    self.hooks.pre_analysis(&exec_ctx);
                    self.run_analysis(
                        analysis,
                        container_path.clone(),
                        container_id,
                        project.clone(),
                    )
                    .map(|output| {
                        output.map(|output| {
                            let assets = Vec::new(); // TODO: Collect `ResourceId`s of `Assets`.
                            self.hooks.assets_added(&exec_ctx, assets);
                            if output.status.success() {
                                self.hooks.post_analysis(&exec_ctx);
                                return None;
                            }

                            let stderr = str::from_utf8(output.stderr.as_slice())
                                .unwrap()
                                .to_string();
                            let exec_ctx = AnalysisExecutionContext {
                                project: project.clone(),
                                analysis: analysis.id().clone(),
                                container: container_path.clone(),
                            };

                            match self.hooks.analysis_error(
                                &exec_ctx,
                                output.status,
                                stderr.clone(),
                            ) {
                                ErrorResponse::Continue => None,
                                ErrorResponse::Terminate => Some(error::Evaluation::Analysis {
                                    project: project.clone(),
                                    analysis: analysis.id().clone(),
                                    container: container_path.clone(),
                                    exit_code: output.status.code(),
                                    err: stderr,
                                }),
                            }
                        })
                    })
                    .err()
                })
                .collect::<Vec<_>>()
        });

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Runs an individual analysis.
    ///
    /// # Returns
    /// `Some([Output](process:Output)) from the analysis if the analysis runs to completion.
    /// `None` if the analysis is killed.
    ///
    /// # Errors
    /// + [`Error`]: The analysis returned a `status` other than `0`.
    #[tracing::instrument(skip(self, analysis))]
    fn run_analysis(
        &self,
        analysis: &Box<dyn Runnable + Send + Sync>,
        container_path: PathBuf,
        container_id: &ResourceId,
        project: ResourceId,
    ) -> Result<Option<process::Output>, error::Evaluation> {
        tracing::trace!("running {} on {:?}", analysis.id(), container_path);

        let mut status = self.status.lock().unwrap();
        let status_idx = status
            .iter()
            .position(|status| {
                status.analysis == *analysis.id() && status.container == *container_id
            })
            .unwrap();
        status[status_idx].status = AnalysisStatus::Running;
        drop(status);

        let mut cmd = analysis.command();
        let mut child = match cmd
            .env(PROJECT_ID_KEY, project.to_string())
            .env(CONTAINER_ID_KEY, &container_path)
            .spawn()
        {
            Ok(child) => child,
            Err(err) => {
                tracing::error!(?err);
                return Err(error::Evaluation::Command {
                    project: project.clone(),
                    analysis: analysis.id().clone(),
                    container: container_path,
                    cmd: format!("{cmd:?}"),
                    err: err.kind(),
                });
            }
        };

        while let Ok(None) = child.try_wait() {
            let state = self.state.lock().unwrap();
            match *state {
                AnalyzerState::Running => {}
                AnalyzerState::Cancel => {
                    break;
                }
                AnalyzerState::Kill => {
                    if let Err(err) = child.kill() {
                        tracing::error!(?err);
                    }

                    let mut status = self.status.lock().unwrap();
                    status[status_idx].status = AnalysisStatus::Killed;
                    return Ok(None);
                }
                AnalyzerState::Done => panic!("invalid state"),
            }
            thread::sleep(time::Duration::from_millis(ANALYZER_STATE_POLL_DELAY_MS));
        }
        let out = child.wait_with_output();

        let mut status = self.status.lock().unwrap();
        status[status_idx].status = AnalysisStatus::Complete;

        let out = match out {
            Ok(out) => out,
            Err(err) => {
                tracing::error!(?err);
                return Err(error::Evaluation::Command {
                    project: project.clone(),
                    analysis: analysis.id().clone(),
                    container: container_path,
                    cmd: format!("{cmd:?}"),
                    err: err.kind(),
                });
            }
        };

        tracing::trace!(?out);
        Ok(Some(out))
    }
}

pub enum ErrorResponse {
    /// Terminate remaining dependent analyses.
    Terminate,

    /// Continue running analyses.
    Continue,
}

impl Default for ErrorResponse {
    fn default() -> Self {
        Self::Terminate
    }
}

pub mod error {
    use crate::types::ResourceId;
    use serde::{Deserialize, Serialize};
    use std::{io, path::PathBuf};

    /// The `Container` could not be found in the graph.
    #[derive(thiserror::Error, Debug)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[error("container {0} not found")]
    pub struct ContainerNotFound(pub ResourceId);

    #[derive(thiserror::Error, Debug)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[error("{0:?}")]
    pub struct LoadAnalyses(Vec<(ResourceId, String)>);

    #[derive(thiserror::Error, Debug)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    pub enum Evaluation {
        #[error("error running `{cmd}` from analysis `{analysis}` on container `{container}` in project `{project}`: {err:?}")]
        Command {
            project: ResourceId,
            analysis: ResourceId,
            container: PathBuf,
            cmd: String,

            #[cfg_attr(feature = "serde", serde(with = "io_error_serde::ErrorKind"))]
            err: io::ErrorKind,
        },

        /// An error occured when running the analysis on the specified `Container`.
        #[error("analysis `{analysis}` running over Container `{container}` in project `{project}` errored: {err}")]
        Analysis {
            project: ResourceId,
            analysis: ResourceId,
            container: PathBuf,
            exit_code: Option<i32>,
            err: String,
        },
    }

    #[derive(thiserror::Error, Debug, derive_more::From)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    pub enum AnalyzerRun {
        #[error("{0:?}")]
        LoadAnalyses(Vec<(ResourceId, String)>),

        #[error("{0:?}")]
        Evaluation(Vec<Evaluation>),
    }

    impl From<LoadAnalyses> for AnalyzerRun {
        fn from(value: LoadAnalyses) -> Self {
            Self::LoadAnalyses(value.0)
        }
    }
}

#[cfg(test)]
#[path = "runner_test.rs"]
mod runner_test;
