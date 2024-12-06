use super::*;

#[test]
fn association_ordering_should_work() {
    let p0 = AnalysisAssociation::new(ResourceId::new());
    let mut p1 = AnalysisAssociation::new(ResourceId::new());

    // equal `priority`, equal `autorun`
    assert_eq!(&p0, &p1, "parameters should be equal");

    // different `priority`, equal `autorun`
    p1.priority = 1;
    assert!(p0 < p1, "parameters with `autorun` equal should be ordered");
    assert!(p1 > p0, "parameters with `autorun` equal should be ordered");

    // different `priority`, different `autorun`
    p1.autorun = !p0.autorun;
    assert!(
        p0 < p1,
        "parameters with `autorun` not equal should be ordered"
    );
    assert!(
        p1 > p0,
        "parameters with `autorun` not equal should be ordered"
    );

    // equal `priority`, different `autorun`
    p1.priority = p0.priority;
    assert!(
        !(p0 < p1),
        "parameters with `autorun` different, and `priority` equal, should not be comparable."
    );
    assert!(
        !(p0 > p1),
        "parameters with `autorun` different, and `priority` equal, should not be comparable."
    );
    assert!(
        p0 != p1,
        "parameters with `autorun` different, and `priority` equal, should not be comparable."
    );
}
