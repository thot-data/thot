use super::super::command::{graph::Command as GraphCommand, project::Command as ProjectCommand};
use super::project::Record as ProjectRecord;
use super::*;
use chrono::Utc;
use std::thread;
use syre_core::graph::ResourceTree;
use syre_core::project::{asset::Builder as Asset, container::Builder as Container};

type ContainerTree = ResourceTree<syre_core::project::Container>;

#[test]
fn datastore_graph_create_subgraph_should_work() {
    // *************
    // *** setup ***
    // *************

    let pid = ResourceId::new();
    let mut project = ProjectRecord::new(
        "/Users/user/Documents/test",
        "fireworks",
        "data",
        Utc::now(),
    );
    project.set_description("Testing which fireworks recipe has the quietest explosion.");

    let (ids, mut graph) = create_fireworks_graph();
    let recipe_b = graph.remove(&ids.recipe_b).unwrap();

    let (tx, rx) = mpsc::unbounded_channel();
    let mut db = Datastore::new(rx);
    thread::spawn(move || db.run());

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Project(ProjectCommand::Create {
        tx: ctx,
        id: pid.clone(),
        project,
    }))
    .unwrap();
    crx.blocking_recv().unwrap().unwrap();

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Graph(GraphCommand::Create {
        tx: ctx,
        graph,
        project: pid.clone(),
    }))
    .unwrap();
    crx.blocking_recv().unwrap().unwrap();

    // ************
    // *** test ***
    // ************

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Graph(GraphCommand::CreateSubgraph {
        tx: ctx,
        graph: recipe_b,
        parent: ids.root.clone(),
    }))
    .unwrap();
    crx.blocking_recv().unwrap().unwrap();

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Query {
        tx: ctx,
        query: format!(
            "SELECT id FROM container WHERE id == container:`{}`",
            ids.recipe_b
        ),
    })
    .unwrap();

    let mut result = crx.blocking_recv().unwrap().unwrap();
    let result = result.take::<Vec<IdRecord>>(0).unwrap();
    assert_eq!(result.len(), 1);
}

#[test]
fn datastore_graph_remove_should_work() {
    // *************
    // *** setup ***
    // *************

    let pid = ResourceId::new();
    let mut project = ProjectRecord::new(
        "/Users/user/Documents/test",
        "fireworks",
        "data",
        Utc::now(),
    );
    project.set_description("Testing which fireworks recipe has the quietest explosion.");

    let (ids, graph) = create_fireworks_graph();
    let (tx, rx) = mpsc::unbounded_channel();
    let mut db = Datastore::new(rx);
    thread::spawn(move || db.run());

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Project(ProjectCommand::Create {
        tx: ctx,
        id: pid.clone(),
        project,
    }))
    .unwrap();
    crx.blocking_recv().unwrap().unwrap();

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Graph(GraphCommand::Create {
        tx: ctx,
        graph,
        project: pid.clone(),
    }))
    .unwrap();
    crx.blocking_recv().unwrap().unwrap();

    // ************
    // *** test ***
    // ************

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Graph(GraphCommand::Remove {
        tx: ctx,
        root: ids.recipe_b.clone(),
    }))
    .unwrap();
    crx.blocking_recv().unwrap().unwrap();

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Query {
        tx: ctx,
        query: format!(
            "SELECT * FROM container:`{}`, container:`{}`, container:`{}`",
            ids.recipe_b, ids.batch_b1, ids.batch_b2
        ),
    })
    .unwrap();

    let mut results = crx.blocking_recv().unwrap().unwrap();
    let results = results
        .take::<Vec<Option<super::container::Record>>>(0)
        .unwrap();

    assert!(results.iter().all(|record| record.is_none()));

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Query {
        tx: ctx,
        query: format!(
            "SELECT * FROM asset:`{}`, asset:`{}`, asset:`{}`, asset:`{}`",
            ids.noise_data_b1, ids.noise_stats_b1, ids.noise_data_b2, ids.noise_stats_b2,
        ),
    })
    .unwrap();

    let mut results = crx.blocking_recv().unwrap().unwrap();
    let results = results
        .take::<Vec<Option<super::asset::Record>>>(0)
        .unwrap();

    assert!(results.iter().all(|record| record.is_none()));
}

#[test]
fn datastore_search_should_work() {
    // *************
    // *** setup ***
    // *************

    let pid = ResourceId::new();
    let mut project = ProjectRecord::new(
        "/Users/user/Documents/test",
        "fireworks",
        "data",
        Utc::now(),
    );
    project.set_description("Testing which fireworks recipe has the quietest explosion.");

    let (ids, graph) = create_fireworks_graph();

    let (tx, rx) = mpsc::unbounded_channel();
    let mut db = Datastore::new(rx);
    thread::spawn(move || db.run());

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Project(ProjectCommand::Create {
        tx: ctx,
        id: pid.clone(),
        project,
    }))
    .unwrap();
    crx.blocking_recv().unwrap().unwrap();

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Graph(GraphCommand::Create {
        tx: ctx,
        graph,
        project: pid.clone(),
    }))
    .unwrap();
    crx.blocking_recv().unwrap().unwrap();

    // ************
    // *** test ***
    // ************

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Query {
        tx: ctx,
        query: "SELECT * FROM container".to_string(),
    })
    .unwrap();
    let mut results = crx.blocking_recv().unwrap().unwrap();
    let results = results.take::<Vec<super::container::Record>>(0).unwrap();
    assert_eq!(results.len(), 7);

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Query {
        tx: ctx,
        query: "SELECT * FROM asset".to_string(),
    })
    .unwrap();
    let mut results = crx.blocking_recv().unwrap().unwrap();
    let results = results.take::<Vec<super::asset::Record>>(0).unwrap();
    assert_eq!(results.len(), 12);

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Search {
        tx: ctx,
        query: "recipe".to_string(),
    })
    .unwrap();
    let results = crx.blocking_recv().unwrap().unwrap();
    assert!(results.contains(&ids.root));
    assert!(results.contains(&ids.recipe_comparison_img));
    assert!(results.contains(&ids.recipe_comparison_table));
    assert!(results.contains(&ids.recipe_a));
    assert!(results.contains(&ids.recipe_b));
    assert!(results.contains(&ids.recipe_stats_a));
    assert!(results.contains(&ids.recipe_stats_b));
    assert_eq!(results.len(), 7);

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Search {
        tx: ctx,
        query: "recipe-stats".to_string(),
    })
    .unwrap();
    let results = crx.blocking_recv().unwrap().unwrap();
    assert!(results.contains(&ids.recipe_stats_a));
    assert!(results.contains(&ids.recipe_stats_b));
    assert_eq!(results.len(), 2);

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Search {
        tx: ctx,
        query: "recipe_stats.csv".to_string(),
    })
    .unwrap();
    let results = crx.blocking_recv().unwrap().unwrap();
    assert!(results.contains(&ids.recipe_stats_a));
    assert!(results.contains(&ids.recipe_stats_b));
    assert_eq!(results.len(), 2);

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Search {
        tx: ctx,
        query: "image".to_string(),
    })
    .unwrap();
    let results = crx.blocking_recv().unwrap().unwrap();
    assert!(results.contains(&ids.recipe_comparison_img));
    assert_eq!(results.len(), 1);

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Search {
        tx: ctx,
        query: "is_second_batch".to_string(),
    })
    .unwrap();
    let results = crx.blocking_recv().unwrap().unwrap();
    assert!(results.contains(&ids.batch_a2));
    assert!(results.contains(&ids.batch_b2));
    assert_eq!(results.len(), 2);

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Search {
        tx: ctx,
        query: "humidity".to_string(),
    })
    .unwrap();
    let results = crx.blocking_recv().unwrap().unwrap();
    assert!(results.contains(&ids.noise_data_a1));
    assert!(results.contains(&ids.noise_data_a2));
    assert!(results.contains(&ids.noise_data_b1));
    assert!(results.contains(&ids.noise_data_b2));
    assert_eq!(results.len(), 4);

    let (ctx, crx) = oneshot::channel();
    tx.send(Command::Search {
        tx: ctx,
        query: "h 0.8".to_string(),
    })
    .unwrap();
    let results = crx.blocking_recv().unwrap().unwrap();
    assert!(results.contains(&ids.recipe_a));
    assert_eq!(results.len(), 1);
}

#[allow(dead_code)]
struct FireworkIds {
    pub root: ResourceId,
    pub recipe_a: ResourceId,
    pub recipe_b: ResourceId,
    pub batch_a1: ResourceId,
    pub batch_a2: ResourceId,
    pub batch_b1: ResourceId,
    pub batch_b2: ResourceId,
    pub recipe_comparison_img: ResourceId,
    pub recipe_comparison_table: ResourceId,
    pub recipe_stats_a: ResourceId,
    pub recipe_stats_b: ResourceId,
    pub noise_data_a1: ResourceId,
    pub noise_data_a2: ResourceId,
    pub noise_data_b1: ResourceId,
    pub noise_data_b2: ResourceId,
    pub noise_stats_a1: ResourceId,
    pub noise_stats_a2: ResourceId,
    pub noise_stats_b1: ResourceId,
    pub noise_stats_b2: ResourceId,
}

fn create_fireworks_graph() -> (FireworkIds, ContainerTree) {
    let mut recipe_comp_png = Asset::with_path("recipe_comparison.png");
    recipe_comp_png
        .set_kind("recipe-comparison")
        .add_tag("image");
    let recipe_comp_png = recipe_comp_png.build();
    let recipe_comp_png_id = recipe_comp_png.rid().clone();

    let mut recipe_comp_csv = Asset::with_path("recipe_comparison.csv");
    recipe_comp_csv
        .set_kind("recipe-comparison")
        .add_tag("table");
    let recipe_comp_csv = recipe_comp_csv.build();
    let recipe_comp_csv_id = recipe_comp_csv.rid().clone();

    let mut root = Container::new("Root");
    root.set_kind("project")
        .set_description("Testing different firework recipes.")
        .insert_asset(recipe_comp_csv)
        .insert_asset(recipe_comp_png);
    let root = root.build();
    let root_id = root.rid().clone();

    let mut ra_stats = Asset::with_path("recipe_stats.csv");
    ra_stats.set_kind("recipe-stats");
    let ra_stats = ra_stats.build();
    let ra_stats_id = ra_stats.rid().clone();

    let mut ra = Container::new("Recipe A");
    ra.set_kind("recipe")
        .set_description("Recipe A has 80% H and 20% C")
        .set_metadatum("name", "A")
        .set_metadatum("h", 0.8)
        .set_metadatum("c", 0.2)
        .insert_asset(ra_stats);
    let ra = ra.build();
    let ra_id = ra.rid().clone();

    let mut rb_stats = Asset::with_path("recipe_stats.csv");
    rb_stats.set_kind("recipe-stats");
    let rb_stats = rb_stats.build();
    let rb_stats_id = rb_stats.rid().clone();

    let mut rb = Container::new("Recipe B");
    rb.set_kind("recipe")
        .set_description("Recipe B has 50% H and 50% C")
        .set_metadatum("name", "B")
        .set_metadatum("h", 0.5)
        .set_metadatum("c", 0.5)
        .insert_asset(rb_stats);
    let rb = rb.build();
    let rb_id = rb.rid().clone();

    let mut rab1_data = Asset::with_path("a1-data.csv");
    rab1_data
        .set_kind("noise-data")
        .set_metadatum("humidity", "low");
    let rab1_data = rab1_data.build();
    let rab1_data_id = rab1_data.rid().clone();

    let mut rab1_stats = Asset::with_path("noise_stats.csv");
    rab1_stats
        .set_kind("noise-stats")
        .set_metadatum("humidity", "high");
    let rab1_stats = rab1_stats.build();
    let rab1_stats_id = rab1_stats.rid().clone();

    let mut rab1 = Container::new("Batch 1");
    rab1.set_kind("batch")
        .set_metadatum("batch", 1)
        .insert_asset(rab1_data)
        .insert_asset(rab1_stats);
    let rab1 = rab1.build();
    let rab1_id = rab1.rid().clone();

    let mut rab2_data = Asset::with_path("a2-data.csv");
    rab2_data.set_kind("noise-data");
    let rab2_data = rab2_data.build();
    let rab2_data_id = rab2_data.rid().clone();

    let mut rab2_stats = Asset::with_path("noise_stats.csv");
    rab2_stats.set_kind("noise-stats");
    let rab2_stats = rab2_stats.build();
    let rab2_stats_id = rab2_stats.rid().clone();

    let mut rab2 = Container::new("Batch 2");
    rab2.set_kind("batch")
        .set_metadatum("batch", 2)
        .set_metadatum("is_second_batch", true)
        .insert_asset(rab2_data)
        .insert_asset(rab2_stats);
    let rab2 = rab2.build();
    let rab2_id = rab2.rid().clone();

    let mut rbb1_data = Asset::with_path("b1-data.csv");
    rbb1_data
        .set_kind("noise-data")
        .set_metadatum("humidity", "low");
    let rbb1_data = rbb1_data.build();
    let rbb1_data_id = rbb1_data.rid().clone();

    let mut rbb1_stats = Asset::with_path("noise_stats.csv");
    rbb1_stats.set_kind("noise-stats");
    let rbb1_stats = rbb1_stats.build();
    let rbb1_stats_id = rbb1_stats.rid().clone();

    let mut rbb1 = Container::new("Batch 1");
    rbb1.set_kind("batch")
        .set_metadatum("batch", 1)
        .insert_asset(rbb1_data)
        .insert_asset(rbb1_stats);
    let rbb1 = rbb1.build();
    let rbb1_id = rbb1.rid().clone();

    let mut rbb2_data = Asset::with_path("b2-data.csv");
    rbb2_data
        .set_kind("noise-data")
        .set_metadatum("humidity", "high");
    let rbb2_data = rbb2_data.build();
    let rbb2_data_id = rbb2_data.rid().clone();

    let mut rbb2_stats = Asset::with_path("noise_stats.csv");
    rbb2_stats.set_kind("noise-stats");
    let rbb2_stats = rbb2_stats.build();
    let rbb2_stats_id = rbb2_stats.rid().clone();

    let mut rbb2 = Container::new("Batch 2");
    rbb2.set_kind("batch")
        .set_metadatum("batch", 2)
        .set_metadatum("is_second_batch", true)
        .insert_asset(rbb2_data)
        .insert_asset(rbb2_stats);
    let rbb2 = rbb2.build();
    let rbb2_id = rbb2.rid().clone();

    let mut graph = ContainerTree::new(root);
    graph.insert(root_id.clone(), ra).unwrap();
    graph.insert(root_id.clone(), rb).unwrap();
    graph.insert(ra_id.clone(), rab1).unwrap();
    graph.insert(ra_id.clone(), rab2).unwrap();
    graph.insert(rb_id.clone(), rbb1).unwrap();
    graph.insert(rb_id.clone(), rbb2).unwrap();

    let ids = FireworkIds {
        root: root_id,
        recipe_a: ra_id,
        recipe_b: rb_id,
        batch_a1: rab1_id,
        batch_a2: rab2_id,
        batch_b1: rbb1_id,
        batch_b2: rbb2_id,
        recipe_comparison_img: recipe_comp_png_id,
        recipe_comparison_table: recipe_comp_csv_id,
        recipe_stats_a: ra_stats_id,
        recipe_stats_b: rb_stats_id,
        noise_data_a1: rab1_data_id,
        noise_data_a2: rab2_data_id,
        noise_data_b1: rbb1_data_id,
        noise_data_b2: rbb2_data_id,
        noise_stats_a1: rab1_stats_id,
        noise_stats_a2: rab2_stats_id,
        noise_stats_b1: rbb1_stats_id,
        noise_stats_b2: rbb2_stats_id,
    };

    (ids, graph)
}
