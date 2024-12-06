use super::super::dev_utils;
use super::*;
use crate::types::data::Value;
use rand::Rng;
use std::collections::HashMap;

#[test]
fn standard_search_filter_new_should_work() {
    let f = StandardSearchFilter::default();

    assert_eq!(f.rid, None, "rid should be None");
    assert_eq!(f.name, None, "name should be None");
    assert_eq!(f.kind, None, "kind should be None");
    assert_eq!(f.tags, None, "tags should be None");
    assert_eq!(f.metadata, None, "metadata should be None");
}

// TODO Known errors in test caused by randomized values can cause failure.
#[test]
fn standard_search_filter_container_matches_should_work() {
    // setup
    // properties

    let obj0 = dev_utils::mock_container(None);

    let p1_kind = match &obj0.properties.kind {
        None => Some(true),
        Some(_) => None,
    };

    let mut obj1 = dev_utils::mock_container(p1_kind);

    // prepend to tags to prevent clash
    obj1.properties.tags = obj1
        .properties
        .tags
        .into_iter()
        .map(|t| format!("pre_{t}"))
        .collect();

    // filters
    let mut rid_filter = StandardSearchFilter::default();
    rid_filter.rid = Some(obj0.rid().clone());

    let mut name_filter = StandardSearchFilter::default();
    name_filter.name = Some(Some(obj0.properties.name.clone()));

    let mut kind_filter = StandardSearchFilter::default();
    kind_filter.kind = Some(obj0.properties.kind.clone());

    let mut tags_filter = StandardSearchFilter::default();
    let mut tf = HashSet::new();
    for t in &obj0.properties.tags {
        if rand::random() {
            tf.insert(t.clone());
        }
    }
    tags_filter.tags = Some(tf);

    let mut md_filter = StandardSearchFilter::default();
    let mut md = HashMap::new();

    // create random metadata filter from data
    // ensure at least one value is included
    let f_len = obj0.properties.metadata.len();
    let mut include: Vec<bool> = Vec::with_capacity(f_len);
    for _ in 0..f_len {
        include.push(rand::random());
    }
    if include.iter().all(|&x| !x) {
        // no true values for include
        // force one to be true
        let mut rng = rand::thread_rng();
        include[rng.gen_range(0..f_len)] = true;
    }

    let mut i = 0;
    for (k, v) in &obj0.properties.metadata {
        if include[i] {
            md.insert(k.clone(), v.clone());
        }
        i += 1;
    }
    if md.len() == 1 {
        for (k, v) in &md {
            if let Value::Bool(_) = v {
                // ensure if matching on boolean value only
                // obj1 has opposite value of obj0
                let p0_val = obj0
                    .properties
                    .metadata
                    .get(k)
                    .expect("metadata should exist");
                let p0_val = p0_val.as_bool().expect("metadata value should be a bool");
                obj1.properties
                    .metadata
                    .insert(k.clone(), Value::Bool(!p0_val));
            }
        }
    }
    md_filter.metadata = Some(md);

    // test
    // rid
    assert!(rid_filter.matches(&obj0), "rid filter should match");
    assert!(!rid_filter.matches(&obj1), "rid filter should not match");

    // name
    assert!(name_filter.matches(&obj0), "name filter should match");
    assert!(!name_filter.matches(&obj1), "name filter should not match");

    // kind
    assert!(kind_filter.matches(&obj0), "kind filter should match");
    assert!(!kind_filter.matches(&obj1), "kind filter should not match");

    // tags
    assert!(tags_filter.matches(&obj0), "tags filter should match");
    assert!(!tags_filter.matches(&obj1), "tags filter should not match");

    // metadata
    assert!(md_filter.matches(&obj0), "metadata filter should match");
    assert!(
        !md_filter.matches(&obj1),
        "metadata filter should not match"
    );

    // TODO Check empty filters. Specifically for `tags` and `metadata`.
}

// TODO Known errors in test caused by randomized values can cause failure.
#[test]
fn standard_search_filter_asset_matches_should_work() {
    // setup
    // properties

    let obj0 = dev_utils::mock_asset(None, None);

    // force p1 to have different name and kind if either is None
    let p1_name = match &obj0.properties.name {
        None => Some(true),
        Some(_) => None,
    };

    let p1_kind = match &obj0.properties.kind {
        None => Some(true),
        Some(_) => None,
    };

    let mut obj1 = dev_utils::mock_asset(p1_name, p1_kind);

    // prepend to tags to prevent clash
    obj1.properties.tags = obj1
        .properties
        .tags
        .into_iter()
        .map(|t| format!("pre_{t}"))
        .collect();

    // filters
    let mut rid_filter = StandardSearchFilter::default();
    rid_filter.rid = Some(obj0.rid().clone());

    let mut name_filter = StandardSearchFilter::default();
    name_filter.name = Some(obj0.properties.name.clone());

    let mut kind_filter = StandardSearchFilter::default();
    kind_filter.kind = Some(obj0.properties.kind.clone());

    let mut tags_filter = StandardSearchFilter::default();
    let mut tf = HashSet::new();
    for t in &obj0.properties.tags {
        if rand::random() {
            tf.insert(t.clone());
        }
    }
    tags_filter.tags = Some(tf);

    let mut md_filter = StandardSearchFilter::default();
    let mut md = HashMap::new();

    // create random metadata filter from data
    // ensure at least one value is included
    let f_len = obj0.properties.metadata.len();
    let mut include: Vec<bool> = Vec::with_capacity(f_len);
    for _ in 0..f_len {
        include.push(rand::random());
    }
    if include.iter().all(|&x| !x) {
        // no true values for include
        // force one to be true
        let mut rng = rand::thread_rng();
        include[rng.gen_range(0..f_len)] = true;
    }

    let mut i = 0;
    for (k, v) in &obj0.properties.metadata {
        if include[i] {
            md.insert(k.clone(), v.clone());
        }
        i += 1;
    }
    if md.len() == 1 {
        for (k, v) in &md {
            if let Value::Bool(_) = v {
                // ensure if matching on boolean value only
                // obj1 has opposite value of obj0
                let p0_val = obj0
                    .properties
                    .metadata
                    .get(k)
                    .expect("metadata should exist");
                let p0_val = p0_val.as_bool().expect("metadata value should be a bool");
                obj1.properties
                    .metadata
                    .insert(k.clone(), Value::Bool(!p0_val));
            }
        }
    }
    md_filter.metadata = Some(md);

    // test
    // rid
    assert!(rid_filter.matches(&obj0), "rid filter should match");
    assert!(!rid_filter.matches(&obj1), "rid filter should not match");

    // name
    assert!(name_filter.matches(&obj0), "name filter should match");
    assert!(!name_filter.matches(&obj1), "name filter should not match");

    // kind
    assert!(kind_filter.matches(&obj0), "kind filter should match");
    assert!(!kind_filter.matches(&obj1), "kind filter should not match");

    // tags
    assert!(tags_filter.matches(&obj0), "tags filter should match");
    assert!(!tags_filter.matches(&obj1), "tags filter should not match");

    // metadata
    assert!(md_filter.matches(&obj0), "metadata filter should match");
    assert!(
        !md_filter.matches(&obj1),
        "metadata filter should not match"
    );

    // TODO Check empty filters. Specifically for `tags` and `metadata`.
}
