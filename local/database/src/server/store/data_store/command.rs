//! Datastore commands.
use super::data_store::Result;
use syre_core::types::ResourceId;
use tokio::sync::oneshot::Sender as Tx;

#[derive(Debug, derive_more::From)]
pub enum Command {
    /// Remove all records from all tables.
    Clear {
        /// Response channel.
        tx: Tx<Result>,
    },

    Query {
        /// Response channel.
        tx: Tx<Result<surrealdb::Response>>,
        query: String,
    },

    Search {
        tx: Tx<Result<Vec<ResourceId>>>,
        query: String,
    },

    #[from]
    Project(project::Command),

    #[from]
    Graph(graph::Command),

    #[from]
    Container(container::Command),

    #[from]
    Asset(asset::Command),
}

pub mod project {
    use super::super::data_store::project::Record;
    use super::{Result, Tx};
    use syre_core::types::ResourceId;

    #[derive(Debug)]
    pub enum Command {
        Create {
            /// Response channel.
            tx: Tx<Result>,
            id: ResourceId,
            project: Record,
        },

        Update {
            /// Response channel.
            tx: Tx<Result>,
            id: ResourceId,
            project: Record,
        },
    }
}

pub mod graph {
    use super::{Result, Tx};
    use syre_core::graph::ResourceTree;
    use syre_core::project::Container;
    use syre_core::types::ResourceId;

    pub type ContainerTree = ResourceTree<Container>;

    #[derive(Debug)]
    pub enum Command {
        Create {
            tx: Tx<Result>,
            graph: ContainerTree,
            project: ResourceId,
        },

        CreateSubgraph {
            tx: Tx<Result>,
            graph: ContainerTree,
            parent: ResourceId,
        },

        Remove {
            tx: Tx<Result>,
            root: ResourceId,
        },
    }
}

pub mod container {
    use super::super::data_store::container::Record;
    use super::{Result, Tx};
    use syre_core::types::ResourceId;

    #[derive(Debug)]
    pub enum Command {
        Create {
            tx: Tx<Result>,
            id: ResourceId,
            container: Record,
            parent: ResourceId,
        },

        Update {
            tx: Tx<Result>,
            id: ResourceId,
            container: Record,
        },
    }
}

pub mod asset {
    use super::super::data_store::asset::Record;
    use super::{Result, Tx};
    use syre_core::types::ResourceId;

    #[derive(Debug)]
    pub enum Command {
        Create {
            tx: Tx<Result>,
            id: ResourceId,
            asset: Record,
            container: ResourceId,
        },

        Update {
            tx: Tx<Result>,
            id: ResourceId,
            asset: Record,
        },

        Remove {
            tx: Tx<Result>,
            id: ResourceId,
        },
    }
}
