use super::*;
use crate::project;
use has_id::HasId;

#[test_log::test]
pub fn runner_should_work() {
    let project = ResourceId::new();
    let analyses = vec![
        MockAnalysis::new(),
        MockAnalysis::new(),
        MockAnalysis::new(),
    ];

    let mut root = project::Container::new("root");
    root.analyses
        .push(project::AnalysisAssociation::new(analyses[0].id().clone()));
    root.analyses
        .push(project::AnalysisAssociation::new(analyses[1].id().clone()));

    let mut c1 = project::Container::new("c1");
    c1.analyses.push(project::AnalysisAssociation::with_params(
        analyses[0].id().clone(),
        true,
        0,
    ));
    c1.analyses.push(project::AnalysisAssociation::with_params(
        analyses[1].id().clone(),
        true,
        1,
    ));

    let mut c2 = project::Container::new("c2");
    c2.analyses.push(project::AnalysisAssociation::with_params(
        analyses[0].id().clone(),
        true,
        0,
    ));
    c2.analyses.push(project::AnalysisAssociation::with_params(
        analyses[1].id().clone(),
        true,
        1,
    ));

    let mut tree = ResourceTree::new(root);
    tree.insert(tree.root().clone(), c1).unwrap();
    tree.insert(tree.root().clone(), c2).unwrap();

    let hooks = MockHooks::new(analyses);
    let builder = Builder::new(hooks);
    let runner = builder.build();
    runner.run(&project, &mut tree).unwrap()
}

#[derive(HasId, Clone)]
struct MockAnalysis {
    #[id]
    id: ResourceId,
}

impl MockAnalysis {
    pub fn new() -> Self {
        Self {
            id: ResourceId::new(),
        }
    }
}

impl Runnable for MockAnalysis {
    #[cfg(target_os = "windows")]
    fn command(&self) -> std::process::Command {
        // noop command with delay
        let mut cmd = std::process::Command::new("ping");
        cmd.args(["192.0.2.0", "-w", "100"]);
        cmd
    }

    #[cfg(not(target_os = "windows"))]
    fn command(&self) -> std::process::Command {
        return std::process::Command::new(":"); // noop
    }
}

struct MockHooks<A>
where
    A: HasId + Runnable + Clone,
{
    analyses: Vec<A>,
}

impl<A> MockHooks<A>
where
    A: HasId + Runnable + Send + Sync + Clone + 'static,
{
    pub fn new(analyses: Vec<A>) -> Self {
        Self { analyses }
    }
}

impl<A> RunnerHooks for MockHooks<A>
where
    A: HasId + Runnable + Send + Sync + Clone + 'static,
{
    fn get_analysis(
        &self,
        _project: ResourceId,
        analysis: ResourceId,
    ) -> StdResult<Box<dyn Runnable + Send + Sync>, String> {
        self.analyses
            .iter()
            .find(|a| *a.id() == analysis)
            .map(|analysis| Box::new(analysis.clone()) as Box<dyn Runnable + Send + Sync>)
            .ok_or("could not find analysis".to_string())
    }
}
