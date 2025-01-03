use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use syre_core::{db::search_filter::deserialize_possible_empty_string, types::ResourceId};

#[derive(Serialize, Deserialize, Debug, derive_more::From)]
pub enum Query {
    Config(Config),
    State(State),
    User(User),
    Project(Project),
    Graph(Graph),
    Container(Container),
    Asset(Asset),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Config {
    Id,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum State {
    /// Retrieve the state of the user manifest.
    UserManifest,

    /// Retrieve the state of the project manifest.
    ProjectManifest,

    /// Retrieve the state of the local config.
    LocalConfig,

    /// Retrieve the state of all projects.
    Projects,

    /// Retrieve the entire graph of a project.
    Graph(
        /// Base path of the project.
        PathBuf,
    ),

    /// Retrieve the state of a container.
    Asset {
        /// Base path of the project.
        project: PathBuf,

        /// Relative path to the container from the data root.
        container: PathBuf,

        /// Relative path to the asset from the container.
        asset: PathBuf,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum User {
    /// Return info on the user.
    Info(ResourceId),

    /// Get all the user's projects.
    Projects(ResourceId),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Project {
    /// Retrieve the state of the project at the given path.
    Get(PathBuf),

    /// Retrieve the state of the project with the given id.
    GetById(ResourceId),

    /// Retrieve the project's path.
    Path(ResourceId),

    /// Retrieve the state of the projects at the given paths.
    ///
    /// # Notes
    /// If a path is not associated with a state, it is excluded from the
    /// result. It is up to the client application to diff the request and response.
    GetMany(Vec<PathBuf>),

    /// Retrieve the project's data and graph.
    Resources(ResourceId),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Graph {
    /// Retrieve a container's parent up to the given root.
    /// If the parent is above the root, or the given container is the
    /// graph's root `None` is returned.
    ///
    /// # Returns
    /// Result<Option<Container>, error::Parent>
    Parent {
        project: ResourceId,
        root: PathBuf,
        container: ResourceId,
    },

    /// Retrieve a container's children.
    ///
    /// # Returns
    /// Result<Vec<Container>, >
    Children {
        project: ResourceId,
        parent: ResourceId,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Container {
    /// Retrieve the state of a container.
    ///
    /// # Returns
    /// `Option<[Container](crate::state::Container)>`
    Get {
        project: ResourceId,

        /// Relative path to the container from the data root.
        container: PathBuf,
    },

    /// Retrieve the state of the container.
    ///
    /// # Returns
    /// `Option<[Container](crate::state::Container)>`
    GetById {
        project: ResourceId,
        container: ResourceId,
    },

    /// Retrieve a container with inherited metadata shaped for use in an analysis script.
    ///
    /// # Returns
    /// Result<
    ///   Option<Result<
    ///     ContainerForAnalysis,
    ///     Vec<Option<IoSerde>>
    ///   >>,
    ///   error::InvalidPath,
    /// >
    GetForAnalysis {
        project: ResourceId,
        container: PathBuf,
    },

    /// Retrieve a container with inherited metadata shaped for use in an analysis script.
    ///
    /// # Returns
    /// Option<Result<
    ///   ContainerForAnalysis,
    ///   Vec<Option<IoSerde>>
    /// >>
    GetByIdForAnalysis {
        project: ResourceId,
        container: ResourceId,
    },

    /// Retrieve the system path of the container.
    ///
    /// # Returns
    /// The abosulte system path to the container.
    /// `Option<Pathbuf>`
    SystemPathById {
        project: ResourceId,
        container: ResourceId,
    },

    /// Find containers from `root` matching `query` with inherited metadata shaped for use in an analysis script.
    ///
    /// # Returns
    /// Result<
    ///     Vec<ContainerForAnalysis>,
    ///     crate::query::error::Query
    /// >
    Search {
        project: ResourceId,
        root: PathBuf,
        query: ContainerQuery,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Asset {
    /// Retrieve the assets parent container.
    ///
    /// # Returns
    /// Result<Container, error::Parent>
    Parent {
        project: ResourceId,
        asset: ResourceId,
    },

    /// Find assets from `root` matching `query` with inherited metadata shaped for use in an analysis script.
    ///
    /// # Returns
    /// Result<
    ///   Vec<AssetForAnalysis>,
    ///   error::InvalidPath,
    /// >
    Search {
        project: ResourceId,
        root: PathBuf,
        query: AssetQuery,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContainerQuery {
    pub name: Option<String>,

    #[serde(default, deserialize_with = "deserialize_possible_empty_string")]
    pub kind: Option<Option<String>>,
    pub tags: Vec<String>,
    pub metadata: Metadata,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AssetQuery {
    #[serde(default, deserialize_with = "deserialize_possible_empty_string")]
    pub name: Option<Option<String>>,

    #[serde(default, deserialize_with = "deserialize_possible_empty_string")]
    pub kind: Option<Option<String>>,
    pub tags: Vec<String>,
    pub metadata: Metadata,
    pub path: Option<PathBuf>,
}

pub type Metadata = Vec<Metadatum>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metadatum {
    pub key: String,
    pub value: syre_core::types::data::Value,
}

pub mod error {
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    use syre_local as local;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum Parent {
        ProjectDoesNotExist,

        /// Project's graph does not exist.
        GraphDoesNotExist,

        /// Child resource does not exist.
        ResourceDoesNotExist,

        /// Graph root path is not valid.
        ///
        /// # Notes
        /// Only occurs for `Container::Parent`.
        InvalidRootPath,

        /// Graph root was not found.
        ///
        /// # Notes
        /// Only occurs for `Container::Parent`.
        GraphRootDoesNotExist,

        /// States within the inheritance graph are corrupt.
        /// Values are ancestor errors starting with the asset's direct parent,
        /// and ending with the graph's root node.
        Inheritance(Vec<CorruptState>),
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum Children {
        ProjectDoesNotExist,

        /// Project's graph does not exist.
        GraphDoesNotExist,

        /// Parent resource does not exist.
        ResourceDoesNotExist,

        /// States within the inheritance graph are corrupt.
        /// Values are ancestor errors starting with the asset's direct parent,
        /// and ending with the graph's root node.
        Inheritance(Vec<CorruptState>),
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum Search {
        ProjectDoesNotExist,

        /// The root of a query does not exist.
        RootDoesNotExist,

        InvalidPath,

        /// Project properties are corrupt.
        ///
        /// # Notes
        /// + Only occurs in asset searches
        ProjectProperties(local::error::IoSerde),

        /// States within the inheritance graph are corrupt.
        /// This may include the container itself.
        Inheritance(Vec<CorruptState>),
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct CorruptState {
        pub path: PathBuf,
        pub err: local::error::IoSerde,
    }

    impl From<(PathBuf, local::error::IoSerde)> for CorruptState {
        fn from((path, err): (PathBuf, local::error::IoSerde)) -> Self {
            Self { path, err }
        }
    }
}
