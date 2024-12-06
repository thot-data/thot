//! Runner test.
//!
//! # Notes
//! + local/database must be running.
use std::fs;
use std::path::PathBuf;
use syre_core::project::{Container, Project, Script};
use syre_core::types::ResourceMap;
use syre_local_database as db;
use syre_local_runner::Runner;

#[test]
fn local_runner_with_python() {
    const PATH: &str = "tests/resources/py";

    let mut tree = load_project_resources(PATH);
    let runner = Runner::new();
    runner.run(&mut tree).unwrap();
}

#[test]
fn local_runner_with_r() {
    const PATH: &str = "tests/resources/r";

    let mut tree = load_project_resources(PATH);
    let runner = Runner::new();
    runner.run(&mut tree).unwrap();
}

fn load_project_resources(path: &str) -> syre_core::graph::ResourceTree<Container> {
    let path = fs::canonicalize(path).unwrap();
    let db = db::Client::new();
    let project = db
        .send(ldb::ProjectCommand::Load(PathBuf::from(path)).into())
        .unwrap();

    let project: db::Result<Project> = serde_json::from_value(project).unwrap();
    let project = project.unwrap();

    let scripts = db
        .send(ldb::AnalysisCommand::LoadProject(project.rid.clone()).into())
        .unwrap();

    let scripts: db::Result<ResourceMap<Script>> = serde_json::from_value(scripts).unwrap();

    scripts.unwrap();

    let tree = db
        .send(ldb::GraphCommand::Load(project.rid.clone()).into())
        .unwrap();

    let tree: db::Result<syre_core::graph::ResourceTree<Container>> =
        serde_json::from_value(tree).unwrap();

    tree.unwrap()
}
