//! Datastore client.
use super::Command;
pub use error::Error;
use syre_core::types::ResourceId;
use tokio::sync::{mpsc, oneshot};

pub type Result<T = ()> = std::result::Result<T, Error>;
type Tx = mpsc::UnboundedSender<Command>;

pub struct Client {
    tx: Tx,
    project: project::Client,
    graph: graph::Client,
    container: container::Client,
    asset: asset::Client,
}

impl Client {
    pub fn new(tx: Tx) -> Self {
        let project = project::Client::new(tx.clone());
        let graph = graph::Client::new(tx.clone());
        let container = container::Client::new(tx.clone());
        let asset = asset::Client::new(tx.clone());

        Self {
            tx,
            project,
            graph,
            container,
            asset,
        }
    }

    /// Remove all records from all tables.
    pub fn clear(&self) -> Result {
        let (tx, rx) = oneshot::channel();
        Self::send(&self.tx, Command::Clear { tx })?;
        Ok(rx.blocking_recv()??)
    }

    pub fn query(&self, query: String) -> Result<surrealdb::Response> {
        let (tx, rx) = oneshot::channel();
        Self::send(&self.tx, Command::Query { tx, query })?;
        Ok(rx.blocking_recv()??)
    }

    pub fn project(&self) -> &project::Client {
        &self.project
    }

    pub fn graph(&self) -> &graph::Client {
        &self.graph
    }

    pub fn container(&self) -> &container::Client {
        &self.container
    }

    pub fn asset(&self) -> &asset::Client {
        &self.asset
    }

    pub fn search(&self, query: String) -> Result<Vec<ResourceId>> {
        let (tx, rx) = oneshot::channel();
        Self::send(&self.tx, Command::Search { tx, query })?;
        Ok(rx.blocking_recv()??)
    }

    pub fn send(tx: &Tx, cmd: Command) -> Result {
        match tx.send(cmd) {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::Send),
        }
    }
}

mod project {
    use super::super::command::project::Command;
    use super::super::data_store::project::Record;
    use super::{Result, Tx};
    use syre_core::types::ResourceId;
    use tokio::sync::oneshot;

    pub struct Client {
        tx: Tx,
    }

    impl Client {
        pub fn new(tx: Tx) -> Self {
            Self { tx }
        }

        pub fn create(&self, id: ResourceId, project: Record) -> Result {
            let (tx, rx) = oneshot::channel();
            super::Client::send(&self.tx, Command::Create { id, project, tx }.into())?;
            Ok(rx.blocking_recv()??)
        }

        pub fn update(&self, id: ResourceId, project: Record) -> Result {
            let (tx, rx) = oneshot::channel();
            super::Client::send(&self.tx, Command::Update { id, project, tx }.into())?;
            Ok(rx.blocking_recv()??)
        }
    }
}

mod graph {
    use super::super::command::graph::{Command, ContainerTree};
    use super::{Result, Tx};
    use syre_core::types::ResourceId;
    use tokio::sync::oneshot;

    pub struct Client {
        tx: Tx,
    }

    impl Client {
        pub fn new(tx: Tx) -> Self {
            Self { tx }
        }

        pub fn create(&self, graph: ContainerTree, project: ResourceId) -> Result {
            let (tx, rx) = oneshot::channel();
            super::Client::send(&self.tx, Command::Create { tx, graph, project }.into())?;
            Ok(rx.blocking_recv()??)
        }

        pub fn create_subgraph(&self, graph: ContainerTree, parent: ResourceId) -> Result {
            let (tx, rx) = oneshot::channel();
            super::Client::send(
                &self.tx,
                Command::CreateSubgraph { tx, graph, parent }.into(),
            )?;
            Ok(rx.blocking_recv()??)
        }

        pub fn remove(&self, root: ResourceId) -> Result {
            let (tx, rx) = oneshot::channel();
            super::Client::send(&self.tx, Command::Remove { tx, root }.into())?;
            Ok(rx.blocking_recv()??)
        }
    }
}

mod container {
    use super::super::command::container::Command;
    use super::super::data_store::container::Record;
    use super::{Result, Tx};
    use syre_core::types::ResourceId;
    use tokio::sync::oneshot;

    pub struct Client {
        tx: Tx,
    }

    impl Client {
        pub fn new(tx: Tx) -> Self {
            Self { tx }
        }

        pub fn create(&self, id: ResourceId, container: Record, parent: ResourceId) -> Result {
            let (tx, rx) = oneshot::channel();
            super::Client::send(
                &self.tx,
                Command::Create {
                    tx,
                    id,
                    container,
                    parent,
                }
                .into(),
            )?;

            Ok(rx.blocking_recv()??)
        }

        pub fn update(&self, id: ResourceId, container: Record) -> Result {
            let (tx, rx) = oneshot::channel();
            super::Client::send(&self.tx, Command::Update { tx, id, container }.into())?;
            Ok(rx.blocking_recv()??)
        }
    }
}

mod asset {
    use super::super::command::asset::Command;
    use super::super::data_store::asset::Record;
    use super::{Result, Tx};
    use syre_core::types::ResourceId;
    use tokio::sync::oneshot;

    pub struct Client {
        tx: Tx,
    }

    impl Client {
        pub fn new(tx: Tx) -> Self {
            Self { tx }
        }

        pub fn create(&self, id: ResourceId, asset: Record, container: ResourceId) -> Result {
            let (tx, rx) = oneshot::channel();
            super::Client::send(
                &self.tx,
                Command::Create {
                    tx,
                    id,
                    asset,
                    container,
                }
                .into(),
            )?;

            Ok(rx.blocking_recv()??)
        }

        pub fn update(&self, id: ResourceId, asset: Record) -> Result {
            let (tx, rx) = oneshot::channel();
            super::Client::send(&self.tx, Command::Update { tx, id, asset }.into())?;
            Ok(rx.blocking_recv()??)
        }

        pub fn remove(&self, id: ResourceId) -> Result {
            let (tx, rx) = oneshot::channel();
            super::Client::send(&self.tx, Command::Remove { tx, id }.into())?;
            Ok(rx.blocking_recv()??)
        }
    }
}

pub mod error {
    use tokio::sync::oneshot::error::RecvError;

    #[derive(Debug)]
    pub enum Error {
        /// Could not send command.
        Send,

        /// Could not recieve response.
        Recieve(RecvError),

        /// Database error.
        Db(surrealdb::Error),
    }

    impl From<RecvError> for Error {
        fn from(value: RecvError) -> Self {
            Self::Recieve(value)
        }
    }

    impl From<surrealdb::Error> for Error {
        fn from(value: surrealdb::Error) -> Self {
            Self::Db(value)
        }
    }
}
