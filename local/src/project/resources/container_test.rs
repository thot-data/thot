use super::*;

// *****************
// *** Container ***
// *****************

// --------------------------
// --- script assocations ---
// --------------------------

#[test]
fn container_contains_script_association_should_work() {
    // setup
    let dir = tempfile::tempdir().unwrap();
    let mut container = Container::new(dir.path());
    let sid = ResourceId::new();
    let assoc = AnalysisAssociation::new(sid.clone());
    container.analyses.push(assoc);

    // test
    assert!(
        container.contains_analysis_association(&sid),
        "container should have script association"
    );
    assert_eq!(
        false,
        container.contains_analysis_association(&ResourceId::new()),
        "container should not have association with script"
    );
}

#[test]
fn container_add_script_association_should_work() {
    // setup
    let dir = tempfile::tempdir().unwrap();
    let mut container = Container::new(dir.path());
    let sid = ResourceId::new();
    let assoc = AnalysisAssociation::new(sid.clone());

    // test
    container
        .add_analysis_association(assoc)
        .expect("add association should work");
    assert!(
        container.contains_analysis_association(&sid),
        "container should contain association"
    );
}

#[test]
#[should_panic(expected = "AlreadyExists")]
fn container_add_script_association_if_already_exists_should_error() {
    // setup
    let dir = tempfile::tempdir().unwrap();
    let mut container = Container::new(dir.path());
    let sid = ResourceId::new();
    let assoc = AnalysisAssociation::new(sid.clone());
    container
        .add_analysis_association(assoc.clone())
        .expect("add association should work");

    // test
    container.add_analysis_association(assoc).unwrap();
}

#[test]
fn container_set_script_association_should_work() {
    // setup
    let dir = tempfile::tempdir().unwrap();
    let mut container = Container::new(dir.path());
    let sid = ResourceId::new();
    let mut assoc = AnalysisAssociation::new(sid.clone());

    // test
    // initial
    container.set_analysis_association(assoc.clone());
    let found = container
        .analyses
        .iter()
        .find(|association| association.analysis() == &sid)
        .unwrap();

    assert_eq!(
        &assoc.priority, &found.priority,
        "association should be set"
    );

    // second
    assoc.priority = 1;
    container.set_analysis_association(assoc.clone());
    let found = container
        .analyses
        .iter()
        .find(|association| association.analysis() == &sid)
        .unwrap();

    assert_eq!(
        &assoc.priority, &found.priority,
        "association should be updated"
    );
}

#[test]
fn container_remove_script_association_should_work() {
    // setup
    let dir = tempfile::tempdir().unwrap();
    let mut container = Container::new(dir.path());
    let sid = ResourceId::new();
    container
        .analyses
        .push(AnalysisAssociation::new(sid.clone()));

    container.remove_analysis_association(&sid);
    assert_eq!(
        false,
        container.contains_analysis_association(&sid),
        "association should no longer exist"
    );
}
