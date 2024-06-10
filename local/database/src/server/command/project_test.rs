use crate::server::Database;
use fake::faker::lorem::raw::Word;
use fake::locales::EN;
use fake::Fake;
use syre_core::project::Project as CoreProject;
use syre_local::project::project;
use syre_local::project::resources::Project as LocalProject;

#[test]
fn load_user_projects_should_work() {
    todo!();
}

#[test]
fn update_project_should_work() {
    // setup
    let _dir = tempfile::tempdir().unwrap();
    project::init(_dir.path()).expect("could not init `Project`");
    let project = LocalProject::load_from(_dir.path()).expect("could not load `Project`");
    let pid = project.rid.clone();

    let name = Word(EN).fake::<String>();
    let mut update = (*project).clone();
    update.name = name.clone();

    let mut db = Database::new().unwrap();
    db.object_store.insert_project(project).unwrap();

    // test
    db.handle_project_update(update).unwrap();
    let project = db.object_store.get_project(&pid).unwrap();
    assert_eq!(name, project.name, "update not applied");
}

#[test]
#[should_panic(expected = "DoesNotExist")]
fn update_project_when_project_does_not_exist_should_error() {
    let project = CoreProject::new("test");
    let mut db = Database::new().unwrap();
    db.handle_project_update(project).unwrap();
}
