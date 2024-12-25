use super::Command;
use std::str::FromStr;
use surrealdb::{
    engine::local::{Db, Mem},
    error, Error, Surreal,
};
use syre_core::types::ResourceId;
use tokio::sync::{mpsc, oneshot};

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
pub enum ResourceKind {
    Container,
    Asset,
}

pub type Result<T = ()> = surrealdb::Result<T>;
type Tx<T> = oneshot::Sender<Result<T>>;
type Rx = mpsc::UnboundedReceiver<Command>;

pub const NAMESPACE: &str = "syre";
pub const DATABASE: &str = "datastore";

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
struct IdRecord {
    id: surrealdb::RecordIdKey,
}

const DEFINE_TABLE_USER: &str = "
DEFINE TABLE user SCHEMAFULL;

DEFINE FIELD email ON TABLE user TYPE string;
";

const DEFINE_TABLE_PROJECT: &str = "
DEFINE TABLE project SCHEMAFULL;

DEFINE FIELD name ON TABLE project TYPE string;
DEFINE FIELD description ON TABLE project TYPE option<string>;
DEFINE FIELD data_root ON TABLE project TYPE string;
DEFINE FIELD analysis_root ON TABLE project TYPE option<string>;

DEFINE FIELD creator ON TABLE project TYPE option<string>;
DEFINE FIELD created ON TABLE project TYPE datetime;

DEFINE FIELD base_path ON TABLE project TYPE string;
";

// NB: `env` field may need to be marked as `FLEXIBLE`.
// https://surrealdb.com/docs/surrealdb/surrealql/statements/define/field#flexible-data-types
const DEFINE_TABLE_ANALYSIS: &str = "
DEFINE TABLE analysis SCHEMAFULL;

DEFINE FIELD path ON TABLE analysis TYPE string;
DEFINE FIELD name ON TABLE analysis TYPE option<string>;
DEFINE FIELD description ON TABLE analysis TYPE option<string>;

DEFINE FIELD language ON TABLE analysis TYPE string;
DEFINE FIELD cmd ON TABLE analysis TYPE string;
DEFINE FIELD args ON TABLE analysis TYPE array<string>;
DEFINE FIELD env ON TABLE analysis TYPE object;

DEFINE FIELD creator ON TABLE container TYPE option<string>;
DEFINE FIELD created ON TABLE container TYPE datetime;
";

const DEFINE_TABLE_CONTAINER: &str = "
DEFINE TABLE container SCHEMAFULL;

DEFINE FIELD name ON TABLE container TYPE string;
DEFINE FIELD kind ON TABLE container TYPE option<string>;
DEFINE FIELD description ON TABLE container TYPE option<string>;
DEFINE FIELD tags ON TABLE container TYPE set<string>;
DEFINE FIELD metadata ON TABLE container TYPE object;

DEFINE FIELD creator ON TABLE container TYPE option<string>;
DEFINE FIELD created ON TABLE container TYPE datetime;

DEFINE FIELD base_path ON TABLE container TYPE string;
";

const DEFINE_TABLE_ASSET: &str = "
DEFINE TABLE asset SCHEMAFULL;

DEFINE FIELD name ON TABLE asset TYPE option<string>;
DEFINE FIELD kind ON TABLE asset TYPE option<string>;
DEFINE FIELD description ON TABLE asset TYPE option<string>;
DEFINE FIELD tags ON TABLE asset TYPE set<string>;
DEFINE FIELD metadata ON TABLE asset TYPE object;

DEFINE FIELD path ON TABLE asset TYPE string;

DEFINE FIELD creator_kind ON TABLE container TYPE string;
DEFINE FIELD creator ON TABLE container TYPE option<string>;
DEFINE FIELD created ON TABLE container TYPE datetime;
";

const DEFINE_TABLE_PERMISSIONS: &str = "
DEFINE TABLE permissions SCHEMAFULL;

DEFINE FIELD resource ON TABLE permissions TYPE record<string>;
DEFINE FIELD user ON TABLE permissions TYPE record<user>;
DEFINE FIELD owner ON TABLE permissions TYPE bool;
DEFINE FIELD read ON TABLE permissions TYPE bool;
DEFINE FIELD write ON TABLE permissions TYPE bool;
DEFINE FIELD execute ON TABLE permissions TYPE bool;

DEFINE INDEX id ON TABLE permissions FIELDS resource, user UNIQUE;
";

const DEFINE_SEARCH_INDICES: &str = "
DEFINE ANALYZER properties_analyzer 
    TOKENIZERS blank, class, punct 
    FILTERS lowercase, ascii, snowball(english), ngram(1, 15);

DEFINE INDEX container_name ON container COLUMNS name SEARCH ANALYZER properties_analyzer BM25(1.2, 0.75);
DEFINE INDEX container_kind ON container COLUMNS kind SEARCH ANALYZER properties_analyzer BM25(1.2, 0.75);
DEFINE INDEX container_description ON container COLUMNS description SEARCH ANALYZER properties_analyzer BM25(1.2, 0.75);
DEFINE INDEX container_tags ON container COLUMNS tags SEARCH ANALYZER properties_analyzer BM25(1.2, 0.75);
DEFINE INDEX container_metadata ON container COLUMNS metadata SEARCH ANALYZER properties_analyzer BM25(1.2, 0.75);

DEFINE INDEX asset_name ON asset COLUMNS name SEARCH ANALYZER properties_analyzer BM25(1.2, 0.75);
DEFINE INDEX asset_kind ON asset COLUMNS kind SEARCH ANALYZER properties_analyzer BM25(1.2, 0.75);
DEFINE INDEX asset_description ON asset COLUMNS description SEARCH ANALYZER properties_analyzer BM25(1.2, 0.75);
DEFINE INDEX asset_tags ON asset COLUMNS tags SEARCH ANALYZER properties_analyzer BM25(1.2, 0.75);
DEFINE INDEX asset_metadata ON asset COLUMNS metadata SEARCH ANALYZER properties_analyzer BM25(1.2, 0.75);
DEFINE INDEX asset_path ON asset COLUMNS path SEARCH ANALYZER properties_analyzer BM25(1.2, 0.75);
";

pub struct Datastore {
    command_rx: Rx,
}

impl Datastore {
    pub fn new(command_rx: Rx) -> Self {
        Self { command_rx }
    }

    #[tokio::main]
    pub async fn run(&mut self) -> Result {
        let db = Surreal::new::<Mem>(()).await?;
        db.use_ns(NAMESPACE).use_db(DATABASE).await?;
        db.query(DEFINE_TABLE_PROJECT).await?;
        db.query(DEFINE_TABLE_CONTAINER).await?;
        db.query(DEFINE_TABLE_ASSET).await?;
        db.query(DEFINE_SEARCH_INDICES).await?;
        let store = Store::new(db);

        while let Some(cmd) = self.command_rx.recv().await {
            tracing::debug!(?cmd);
            match cmd {
                Command::Clear { tx } => store.clear(tx).await,
                Command::Query { query, tx } => store.handle_query(tx, query).await,
                Command::Search { tx, query } => store.handle_search(tx, query).await,
                Command::Project(cmd) => store.handle_command_project(cmd).await,
                Command::Graph(cmd) => store.handle_command_graph(cmd).await,
                Command::Container(cmd) => store.handle_command_container(cmd).await,
                Command::Asset(cmd) => store.handle_command_asset(cmd).await,
            }
        }

        tracing::debug!("shutting down datastore");
        Ok(())
    }
}

struct Store {
    db: Surreal<Db>,
}

impl Store {
    pub fn new(db: Surreal<Db>) -> Self {
        Self { db }
    }

    /// Clear all records in all tables.
    pub async fn clear(&self, tx: Tx<()>) {
        if let Err(err) = self.db.delete::<Vec<project::Record>>("project").await {
            Self::send_response(tx, Err(err));
            return;
        }

        Self::send_response(tx, Ok(()))
    }

    pub async fn handle_query(&self, tx: Tx<surrealdb::Response>, query: String) {
        Self::send_response(tx, self.db.query(query).await);
    }

    pub async fn handle_search(&self, tx: Tx<Vec<ResourceId>>, query: String) {
        #[allow(dead_code)]
        #[derive(serde::Deserialize, Debug)]
        struct Record {
            id: surrealdb::RecordIdKey,
            score: f64,
        }

        let query = escape_string(query);
        let container_query = format!(
            "SELECT
                id,
                math::product([
                    search::score(0) * 3 
                    + search::score(1) * 3 
                    + search::score(2) * 1 
                    + search::score(3) * 2 
                    + search::score(4) * 2,
                    0.090909 // normalization
                ]) AS score
            FROM container
            WHERE name @0@ '{query}'
                OR kind @1@ '{query}'
                OR description @2@ '{query}'
                OR tags @3@ '{query}'
                OR metadata @4@ '{query}'
            ORDER BY score DESC"
        );

        let asset_query = format!(
            "SELECT
                id,
                math::product([
                    search::score(0) * 3 
                    + search::score(1) * 3 
                    + search::score(2) * 1 
                    + search::score(3) * 2 
                    + search::score(4) * 2
                    + search::score(5) * 3,
                    0.071429 // normalization
                ]) AS score
            FROM asset
            WHERE name @0@ '{query}'
                OR kind @1@ '{query}'
                OR description @2@ '{query}'
                OR tags @3@ '{query}'
                OR metadata @4@ '{query}'
                OR path @5@ '{query}'
            ORDER BY score DESC"
        );

        let mut container_results = match self.db.query(container_query).await {
            Ok(results) => results,
            Err(err) => {
                Self::send_response(tx, Err(err));
                return;
            }
        };

        let mut asset_results = match self.db.query(asset_query).await {
            Ok(results) => results,
            Err(err) => {
                Self::send_response(tx, Err(err));
                return;
            }
        };

        let container_results = match container_results.take::<Vec<Record>>(0) {
            Ok(results) => results,
            Err(err) => {
                Self::send_response(tx, Err(err));
                return;
            }
        };

        let mut asset_results = match asset_results.take::<Vec<Record>>(0) {
            Ok(results) => results,
            Err(err) => {
                Self::send_response(tx, Err(err));
                return;
            }
        };

        let mut results = container_results;
        results.append(&mut asset_results);
        results.sort_by(|ra, rb| ra.score.partial_cmp(&rb.score).unwrap());

        let results = results
            .into_iter()
            .map(|record| ResourceId::from_str(&record.id.to_string()).unwrap())
            .collect();

        Self::send_response(tx, Ok(results));
    }

    fn send_response<T>(tx: Tx<T>, value: Result<T>) {
        match tx.send(value) {
            Ok(_) => {}
            Err(_) => tracing::error!("could not send response"),
        }
    }

    /// Selects the projects id that contains a resource.
    ///
    /// # Returns
    /// Id of the project that the resource belongs to.
    async fn project_from_resource_id(
        &self,
        kind: ResourceKind,
        id: ResourceId,
    ) -> Result<Option<ResourceId>> {
        let table = match kind {
            ResourceKind::Container => "container",
            ResourceKind::Asset => "asset",
        };

        let mut result = self
            .db
            // .query("SELECT in FROM has_resource WHERE out = type::thing($table, $id)")
            // .bind(("table", table))
            // .bind(("id", id))
            .query("SELECT in AS id FROM has_resource WHERE out = $id")
            .bind(("id", (table, id.into_surreal_id())))
            .await?;

        let result = result.take::<Vec<IdRecord>>(0)?;
        if result.is_empty() {
            return Ok(None);
        };

        assert_eq!(result.len(), 1);
        let rid = result[0].id.to_string();
        let Ok(rid) = ResourceId::from_str(&rid) else {
            return Err(Error::Db(error::Db::IdInvalid {
                value: rid.to_string(),
            }));
        };

        Ok(Some(rid))
    }
}

pub mod project {
    use super::super::command::project::Command;
    use super::{Result, Store};
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    use syre_core::{project::Project as CoreProject, types::ResourceId};
    use syre_local::project::{config::Settings, resources::Project as LocalProject};

    impl Store {
        pub async fn handle_command_project(&self, cmd: Command) {
            match cmd {
                Command::Create { id, project, tx } => {
                    let resp = self.project_create(id, project).await;
                    Self::send_response(tx, resp);
                }

                Command::Update { id, project, tx } => {
                    let resp = self.project_update(id, project).await;
                    Self::send_response(tx, resp);
                }
            }
        }

        async fn project_create(&self, id: ResourceId, project: Record) -> Result {
            self.db
                .create::<Option<Record>>(("project", id.to_string()))
                .content(project)
                .await?;

            Ok(())
        }

        async fn project_update(&self, id: ResourceId, project: Record) -> Result {
            self.db
                .update::<Option<Record>>(("project", id.to_string()))
                .content(project)
                .await?;

            Ok(())
        }
    }

    #[allow(dead_code)]
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Record {
        name: String,
        description: Option<String>,
        data_root: PathBuf,
        analysis_root: Option<PathBuf>,

        creator: Option<syre_core::types::UserId>,
        created: DateTime<Utc>,

        base_path: PathBuf,
    }

    impl Record {
        pub fn new(
            base_path: impl Into<PathBuf>,
            name: impl Into<String>,
            data_root: impl Into<PathBuf>,
            created: DateTime<Utc>,
        ) -> Self {
            Self {
                name: name.into(),
                description: None,
                data_root: data_root.into(),
                analysis_root: None,
                creator: None,
                created,
                base_path: base_path.into(),
            }
        }

        pub fn set_description(&mut self, description: impl Into<String>) {
            let _ = self.description.insert(description.into());
        }

        pub fn set_analysis_root(&mut self, analysis_root: impl Into<PathBuf>) {
            let _ = self.analysis_root.insert(analysis_root.into());
        }
    }

    impl From<LocalProject> for Record {
        fn from(value: LocalProject) -> Self {
            let (properties, settings, base_path) = value.into_parts();
            let CoreProject {
                name,
                description,
                data_root,
                analysis_root,
                meta_level: _,
                ..
            } = properties;

            let Settings {
                local_format_version: _,
                created,
                creator,
                permissions: _,
            } = settings;

            Self {
                name,
                description,
                data_root,
                analysis_root,
                creator,
                created,
                base_path,
            }
        }
    }
}

pub mod graph {
    use super::{
        super::command::graph::{Command, ContainerTree},
        asset::Record as AssetRecord,
        container::Record as ContainerRecord,
        error, Error, Result, Store,
    };
    use futures::future::{BoxFuture, FutureExt};
    use std::{collections::HashMap, str::FromStr};
    use syre_core::{
        graph::ResourceNode,
        project::{Asset, Container},
        types::ResourceId,
    };

    type Nodes = HashMap<ResourceId, ResourceNode<Container>>;

    impl Store {
        pub async fn handle_command_graph(&self, cmd: Command) {
            match cmd {
                Command::Create { tx, graph, project } => {
                    let resp = self.graph_create(graph, project).await;
                    Self::send_response(tx, resp);
                }

                Command::CreateSubgraph { tx, graph, parent } => {
                    let resp = self.graph_create_subgraph(graph, parent).await;
                    Self::send_response(tx, resp);
                }

                Command::Remove { tx, root } => {
                    let resp = self.graph_remove(root).await;
                    Self::send_response(tx, resp);
                }
            }
        }

        async fn graph_create(&self, graph: ContainerTree, project: ResourceId) -> Result {
            let (nodes, edges) = graph.into_components();
            let Resources { containers, assets } = nodes_to_records(nodes);
            for container in containers {
                let ContainerInfo { id, record } = container;
                self.db
                    .create::<Option<ContainerRecord>>(("container", id.to_string()))
                    .content(record)
                    .await?;

                self.db
                    .query("RELATE $project -> has_resource -> $id")
                    .bind(("project", ("project", project.clone().into_surreal_id())))
                    .bind(("id", ("container", id.into_surreal_id())))
                    .await?;
            }

            for (parent, children) in edges {
                for child in children {
                    self.db
                        .query("RELATE $parent -> has_child -> $child")
                        .bind(("parent", ("container", parent.clone().into_surreal_id())))
                        .bind(("child", ("container", child.clone().into_surreal_id())))
                        .await?;
                }
            }

            for asset in assets {
                let AssetInfo {
                    id,
                    record,
                    container,
                } = asset;
                self.db
                    .create::<Option<AssetRecord>>(("asset", id.to_string()))
                    .content(record)
                    .await?;

                self.db
                    .query("RELATE $container -> has_asset -> $id")
                    .bind((
                        "container",
                        ("container", container.clone().into_surreal_id()),
                    ))
                    .bind(("id", ("asset", id.clone().into_surreal_id())))
                    .await?;

                self.db
                    .query("RELATE $project -> has_resource -> $id")
                    .bind(("project", ("project", project.clone().into_surreal_id())))
                    .bind(("id", ("asset", id.into_surreal_id())))
                    .await?;
            }

            Ok(())
        }

        async fn graph_create_subgraph(&self, graph: ContainerTree, parent: ResourceId) -> Result {
            let Some(project) = self
                .project_from_resource_id(super::ResourceKind::Container, parent.clone())
                .await?
            else {
                return Err(Error::Db(error::Db::NoRecordFound));
            };

            let root = graph.root().clone();
            self.graph_create(graph, project).await?;

            self.db
                .query("RELATE $parent -> has_child -> $child")
                .bind(("parent", ("container", parent.clone().into_surreal_id())))
                .bind(("child", ("container", root.into_surreal_id())))
                .await?;

            Ok(())
        }

        async fn graph_remove(&self, root: ResourceId) -> Result {
            let containers = self.descendants(root).await?;
            for container in containers {
                self.db
                    .query("DELETE asset WHERE <-(has_asset WHERE in == $container)")
                    .bind((
                        "container",
                        ("container", container.clone().into_surreal_id()),
                    ))
                    .await?;

                self.db
                    .delete::<Option<super::container::Record>>(("container", container))
                    .await?;
            }

            Ok(())
        }

        async fn children(&self, parent: ResourceId) -> Result<Vec<ResourceId>> {
            #[derive(serde::Deserialize, Debug)]
            struct Record {
                out: surrealdb::RecordIdKey,
            }

            let mut results = self
                .db
                .query("SELECT out FROM has_child WHERE in == $parent")
                .bind(("parent", ("container", parent.into_surreal_id())))
                .await?;

            let results = results.take::<Vec<Record>>(0)?;
            let ids = results
                .into_iter()
                .map(|record| ResourceId::from_str(&record.out.to_string()).unwrap())
                .collect();

            Ok(ids)
        }

        // See https://rust-lang.github.io/async-book/07_workarounds/04_recursion.html
        /// Get all descendant Containers.
        /// Include root.
        fn descendants(&self, root: ResourceId) -> BoxFuture<'_, Result<Vec<ResourceId>>> {
            let mut descendants = vec![root.clone()];
            async move {
                for child in self.children(root).await? {
                    descendants.extend(self.descendants(child).await?);
                }

                Ok(descendants)
            }
            .boxed()
        }
    }

    fn nodes_to_records(nodes: Nodes) -> Resources {
        let mut container_info = Vec::with_capacity(nodes.len());
        let mut asset_info = Vec::new();
        for container in nodes.into_values() {
            todo!();
            // let Container {
            //     rid: cid,
            //     properties,
            //     assets,
            //     analyses: _,
            // } = container.into_data();

            // let record = ContainerRecord::;

            // container_info.push(ContainerInfo {
            //     id: cid.clone(),
            //     record,
            // });

            // for asset in assets.into_values() {
            //     let Asset {
            //         rid: aid,
            //         properties,
            //         path,
            //     } = asset;

            //     asset_info.push(AssetInfo {
            //         id: aid,
            //         record: AssetRecord::from_properties(properties, path),
            //         container: cid.clone(),
            //     });
            // }
        }

        Resources {
            containers: container_info,
            assets: asset_info,
        }
    }

    struct Resources {
        pub containers: Vec<ContainerInfo>,
        pub assets: Vec<AssetInfo>,
    }

    struct ContainerInfo {
        id: ResourceId,
        record: ContainerRecord,
    }

    struct AssetInfo {
        id: ResourceId,
        record: AssetRecord,
        container: ResourceId,
    }
}

pub mod container {
    use super::{super::command::container::Command, ResourceKind, Result, Store};
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    use surrealdb::{error, Error};
    use syre_core::{
        project::{ContainerProperties, Metadata},
        types::{ResourceId, UserId},
    };
    use syre_local::project::{config::ContainerSettings, resources::Container};

    impl Store {
        pub async fn handle_command_container(&self, cmd: Command) {
            match cmd {
                Command::Create {
                    tx,
                    id,
                    container,
                    parent,
                } => {
                    let resp = self.container_create(id, container, parent).await;
                    Self::send_response(tx, resp);
                }

                Command::Update { tx, id, container } => {
                    let resp = self.container_update(id, container).await;
                    Self::send_response(tx, resp);
                }
            }
        }

        async fn container_create(
            &self,
            id: ResourceId,
            container: Record,
            parent: ResourceId,
        ) -> Result {
            let Some(project) = self
                .project_from_resource_id(ResourceKind::Container, parent.clone())
                .await?
            else {
                return Err(Error::Db(error::Db::NoRecordFound));
            };

            self.db
                .create::<Option<Record>>(("container", id.to_string()))
                .content(container)
                .await?;

            self.db
                .query("RELATE $parent -> has_child -> $id")
                .bind(("parent", ("container", parent.into_surreal_id())))
                .bind(("id", ("container", id.clone().into_surreal_id())))
                .await?;

            self.db
                .query("RELATE $project -> has_resource -> $id")
                .bind(("project", ("project", project.into_surreal_id())))
                .bind(("id", ("container", id.into_surreal_id())))
                .await?;

            Ok(())
        }

        async fn container_update(&self, id: ResourceId, container: Record) -> Result {
            self.db
                .update::<Option<Record>>(("container", id.to_string()))
                .content(container)
                .await?;

            Ok(())
        }
    }

    #[allow(dead_code)]
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Record {
        name: String,
        kind: Option<String>,
        description: Option<String>,
        tags: Vec<String>,
        metadata: Metadata,
        creator: Option<UserId>,
        created: DateTime<Utc>,
        base_path: PathBuf,
    }

    impl From<Container> for Record {
        fn from(value: Container) -> Self {
            let (container, settings, base_path) = value.into_parts();
            let ContainerProperties {
                name,
                kind,
                description,
                tags,
                metadata,
                ..
            } = container.properties;

            let ContainerSettings {
                creator, created, ..
            } = settings;

            Self {
                name,
                kind,
                description,
                tags,
                metadata,
                creator,
                created,
                base_path,
            }
        }
    }
}

pub mod asset {
    use super::{super::command::asset::Command, ResourceKind, Result, Store};
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    use surrealdb::{error, Error};
    use syre_core::{
        project::{Asset, AssetProperties, Metadata},
        types::ResourceId,
    };

    impl Store {
        pub async fn handle_command_asset(&self, cmd: Command) {
            match cmd {
                Command::Create {
                    tx,
                    id,
                    asset,
                    container,
                } => {
                    let resp = self.asset_create(id, asset, container).await;
                    Self::send_response(tx, resp);
                }

                Command::Update { tx, id, asset } => {
                    let resp = self.asset_update(id, asset).await;
                    Self::send_response(tx, resp);
                }

                Command::Remove { tx, id } => {
                    let resp = self.asset_remove(id).await;
                    Self::send_response(tx, resp);
                }
            }
        }

        async fn asset_create(
            &self,
            id: ResourceId,
            asset: Record,
            container: ResourceId,
        ) -> Result {
            let Some(project) = self
                .project_from_resource_id(ResourceKind::Container, container.clone())
                .await?
            else {
                return Err(Error::Db(error::Db::NoRecordFound));
            };

            self.db
                .create::<Option<Record>>(("asset", id.to_string()))
                .content(asset)
                .await?;

            self.db
                .query("RELATE $container -> has_asset -> $id")
                .bind(("container", ("container", container.into_surreal_id())))
                .bind(("id", ("asset", id.clone().into_surreal_id())))
                .await?;

            self.db
                .query("RELATE $project -> has_resource -> $id")
                .bind(("project", ("project", project.into_surreal_id())))
                .bind(("id", ("asset", id.into_surreal_id())))
                .await?;

            Ok(())
        }

        async fn asset_update(&self, id: ResourceId, asset: Record) -> Result {
            self.db
                .update::<Option<Record>>(("asset", id.to_string()))
                .content(asset)
                .await?;

            Ok(())
        }

        async fn asset_remove(&self, id: ResourceId) -> Result {
            self.db
                .delete::<Option<Record>>(("asset", id.into_surreal_id()))
                .await?;

            Ok(())
        }
    }

    #[allow(dead_code)]
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Record {
        name: Option<String>,
        kind: Option<String>,
        description: Option<String>,
        tags: Vec<String>,
        metadata: Metadata,
        path: PathBuf,
        creator_kind: types::CreatorKind,
        creator: Option<types::CreatorId>,
        created: DateTime<Utc>,
    }

    impl Record {
        pub fn new(
            path: PathBuf,
            creator: syre_core::types::Creator,
            created: DateTime<Utc>,
        ) -> Self {
            let (creator_kind, creator) = types::creator_to_parts(creator);
            Self {
                path,
                created,
                name: None,
                kind: None,
                description: None,
                tags: vec![],
                metadata: Metadata::new(),
                creator,
                creator_kind,
            }
        }

        pub fn from_properties(properties: AssetProperties, path: PathBuf) -> Self {
            let created = properties.created().clone();
            let AssetProperties {
                creator,
                name,
                kind,
                description,
                tags,
                metadata,
                ..
            } = properties;
            let (creator_kind, creator) = types::creator_to_parts(creator);
            Self {
                path,
                created,
                name,
                kind,
                description,
                tags,
                metadata,
                creator,
                creator_kind,
            }
        }
    }

    impl From<Asset> for Record {
        fn from(value: Asset) -> Self {
            let Asset {
                properties, path, ..
            } = value;

            let created = properties.created().clone();
            let AssetProperties {
                name,
                kind,
                description,
                tags,
                metadata,
                creator,
                ..
            } = properties;
            let (creator_kind, creator) = types::creator_to_parts(creator);

            Self {
                name,
                kind,
                description,
                tags,
                metadata,
                path,
                created,
                creator,
                creator_kind,
            }
        }
    }

    mod types {
        use serde::{Deserialize, Serialize};
        use syre_core::types::{Creator, ResourceId, UserId};

        #[derive(Debug, Serialize, Deserialize)]
        pub enum CreatorId {
            Id(ResourceId),
            Email(String),
        }

        #[derive(Debug, Serialize, Deserialize)]
        pub enum CreatorKind {
            User,
            Script,
        }

        /// Converts a [syre_core::Creator](syre_core::types::Creator) into its
        /// corresponding components.
        pub fn creator_to_parts(
            creator: syre_core::types::Creator,
        ) -> (CreatorKind, Option<CreatorId>) {
            match creator {
                Creator::User(None) => (CreatorKind::User, None),
                Creator::User(Some(UserId::Id(id))) => (CreatorKind::User, Some(CreatorId::Id(id))),
                Creator::User(Some(UserId::Email(email))) => {
                    (CreatorKind::User, Some(CreatorId::Email(email)))
                }
                Creator::Script(id) => (CreatorKind::Script, Some(CreatorId::Id(id))),
            }
        }
    }
}

/// Escapes a string.
///
/// # Characters
/// + `'`
fn escape_string(input: impl AsRef<str>) -> String {
    let input = input.as_ref();
    let input = input.replace("'", "\\'");
    input
}

#[cfg(test)]
#[path = "./data_store_test.rs"]
mod data_store_test;
