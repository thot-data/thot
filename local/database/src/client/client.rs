//! Client to interact with a [`Database`].
use crate::{common, constants::LOCALHOST, types::PortNumber, Query};
use serde_json::Value as JsValue;
use std::net::TcpListener;

static CONNECT_TIMEOUT: i32 = 5000;
static RECV_TIMEOUT: i32 = 5000;

pub type CmdResult<T, E> = zmq::Result<Result<T, E>>;

/// Checks if a given port on the loopback address is free.
fn port_is_free(port: PortNumber) -> bool {
    TcpListener::bind(format!("{LOCALHOST}:{port}")).is_ok()
}

pub struct Client {
    config: Config,
    zmq_context: zmq::Context,
    state: state::Client,
    user: user::Client,
    project: project::Client,
    container: container::Client,
}

impl Client {
    pub fn new() -> Self {
        let config = Config::new(CONNECT_TIMEOUT, RECV_TIMEOUT);
        let ctx = zmq::Context::new();
        Self {
            config: config.clone(),
            zmq_context: ctx.clone(),
            state: state::Client::new(config.clone(), ctx.clone()),
            user: user::Client::new(config.clone(), ctx.clone()),
            project: project::Client::new(config.clone(), ctx.clone()),
            container: container::Client::new(config.clone(), ctx.clone()),
        }
    }

    pub fn send(&self, query: Query) -> zmq::Result<JsValue> {
        // TODO: May be able to move creation of `socket` to `#new`, but may run into `Sync` issues.
        let socket = self.socket();
        socket.send(&serde_json::to_string(&query).unwrap(), 0)?;

        let mut msg = zmq::Message::new();
        socket.recv(&mut msg, 0)?;

        Ok(serde_json::from_str(msg.as_str().unwrap()).unwrap())
    }

    pub fn state(&self) -> &state::Client {
        &self.state
    }

    pub fn user(&self) -> &user::Client {
        &self.user
    }

    pub fn project(&self) -> &project::Client {
        &self.project
    }

    pub fn container(&self) -> &container::Client {
        &self.container
    }

    /// Checks if a database is running.
    pub fn server_available() -> bool {
        use crate::{constants, query};

        if port_is_free(constants::REQ_REP_PORT) {
            // port is open, no chance of a listener
            return false;
        }

        let ctx = zmq::Context::new();
        const SOCKET_KIND: zmq::SocketType = zmq::REQ;
        let socket = ctx.socket(SOCKET_KIND).unwrap();
        socket.set_connect_timeout(CONNECT_TIMEOUT).unwrap();
        socket.set_rcvtimeo(RECV_TIMEOUT).unwrap();
        socket
            .connect(&common::zmq_url(SOCKET_KIND).unwrap())
            .unwrap();

        socket
            .send(
                &serde_json::to_string(&Query::Config(query::Config::Id)).unwrap(),
                0,
            )
            .unwrap();

        let mut msg = zmq::Message::new();
        let res = socket.recv(&mut msg, 0);
        if res.is_err() {
            return false;
        }

        let Some(id_str) = msg.as_str() else {
            panic!("invalid response");
        };

        let id_str: &str = serde_json::from_str(id_str).unwrap();

        return id_str == constants::DATABASE_ID;
    }

    fn socket(&self) -> zmq::Socket {
        const SOCKET_KIND: zmq::SocketType = zmq::REQ;
        let socket = self.zmq_context.socket(SOCKET_KIND).unwrap();
        socket
            .set_connect_timeout(self.config.connect_timeout())
            .unwrap();
        socket.set_rcvtimeo(self.config.recv_timeout()).unwrap();
        socket
            .connect(&common::zmq_url(SOCKET_KIND).unwrap())
            .unwrap();

        socket
    }
}

mod state {
    use super::Config;
    use crate::{
        common, query,
        state::{self, ConfigState, ManifestState},
        Query,
    };
    use serde_json::Value as JsValue;
    use std::path::PathBuf;
    use syre_core::system::User;
    use syre_local::system::resources::Config as LocalConfig;

    pub struct Client {
        config: Config,
        zmq_context: zmq::Context,
    }

    impl Client {
        pub(super) fn new(config: Config, zmq_context: zmq::Context) -> Self {
            Self {
                config,
                zmq_context,
            }
        }

        fn send(&self, query: Query) -> zmq::Result<JsValue> {
            // TODO: May be able to move creation of `socket` to `#new`, but may run into `Sync` issues.
            let socket = self.socket();
            socket.send(&serde_json::to_string(&query).unwrap(), 0)?;

            let mut msg = zmq::Message::new();
            socket.recv(&mut msg, 0)?;

            Ok(serde_json::from_str(msg.as_str().unwrap()).unwrap())
        }

        fn socket(&self) -> zmq::Socket {
            const SOCKET_KIND: zmq::SocketType = zmq::REQ;
            let socket = self.zmq_context.socket(SOCKET_KIND).unwrap();
            socket
                .set_connect_timeout(self.config.connect_timeout())
                .unwrap();
            socket.set_rcvtimeo(self.config.recv_timeout()).unwrap();
            socket
                .connect(&common::zmq_url(SOCKET_KIND).unwrap())
                .unwrap();

            socket
        }
    }

    impl Client {
        /// # Returns
        /// State of the user manifest.
        pub fn user_manifest(&self) -> zmq::Result<ManifestState<User>> {
            let state = self.send(query::State::UserManifest.into())?;
            Ok(serde_json::from_value(state).unwrap())
        }

        /// # Returns
        /// State of the project manifest.
        pub fn project_manifest(&self) -> zmq::Result<ManifestState<PathBuf>> {
            let state = self.send(query::State::ProjectManifest.into())?;
            Ok(serde_json::from_value(state).unwrap())
        }

        /// # Returns
        /// State of the local config settings.
        pub fn local_config(&self) -> zmq::Result<ConfigState<LocalConfig>> {
            let state = self.send(query::State::LocalConfig.into())?;
            Ok(serde_json::from_value(state).unwrap())
        }

        /// # Returns
        /// State of all projects.
        pub fn projects(&self) -> zmq::Result<Vec<crate::state::Project>> {
            let state = self.send(query::State::Projects.into())?;
            Ok(serde_json::from_value(state).unwrap())
        }

        /// # Returns
        /// State of the project at the given path.
        pub fn project(
            &self,
            path: impl Into<PathBuf>,
        ) -> zmq::Result<Option<crate::state::Project>> {
            let state = self.send(query::Project::Get(path.into()).into())?;
            Ok(serde_json::from_value(state).unwrap())
        }

        /// # Returns
        /// State of the project at the given path.

        /// # Arguments
        /// 1. `project`: Base path of the project.
        ///
        /// # Returns
        /// Entire graph of a project.
        /// `None` if the project or graph does not exist.
        pub fn graph(&self, project: impl Into<PathBuf>) -> zmq::Result<Option<state::Graph>> {
            let state = self.send(query::State::Graph(project.into()).into())?;
            Ok(serde_json::from_value(state).unwrap())
        }

        /// Retrieve the state of a container.
        ///
        /// # Arguments
        /// 1. `project`: Base path of the project.
        /// 2. `container`: Relative path to the container from the data root.
        /// 3. `asset`: Relative path to the asset from the container.
        ///
        /// # Returns
        /// `None` if the project, container, or asset does not exist.
        pub fn asset(
            &self,
            project: impl Into<PathBuf>,
            container: impl Into<PathBuf>,
            asset: impl Into<PathBuf>,
        ) -> zmq::Result<Option<state::Asset>> {
            let state = self.send(
                query::State::Asset {
                    project: project.into(),
                    container: container.into(),
                    asset: asset.into(),
                }
                .into(),
            )?;

            Ok(serde_json::from_value(state).unwrap())
        }
    }
}

mod user {
    use super::Config;
    use crate::{common, query, state, Query};
    use serde_json::Value as JsValue;
    use std::path::PathBuf;
    use syre_core::{system::User, types::ResourceId};

    pub struct Client {
        config: Config,
        zmq_context: zmq::Context,
    }

    impl Client {
        pub(super) fn new(config: Config, zmq_context: zmq::Context) -> Self {
            Self {
                config,
                zmq_context,
            }
        }

        fn send(&self, query: Query) -> zmq::Result<JsValue> {
            // TODO: May be able to move creation of `socket` to `#new`, but may run into `Sync` issues.
            let socket = self.socket();
            socket.send(&serde_json::to_string(&query).unwrap(), 0)?;

            let mut msg = zmq::Message::new();
            socket.recv(&mut msg, 0)?;

            Ok(serde_json::from_str(msg.as_str().unwrap()).unwrap())
        }

        fn socket(&self) -> zmq::Socket {
            const SOCKET_KIND: zmq::SocketType = zmq::REQ;
            let socket = self.zmq_context.socket(SOCKET_KIND).unwrap();
            socket
                .set_connect_timeout(self.config.connect_timeout())
                .unwrap();
            socket.set_rcvtimeo(self.config.recv_timeout()).unwrap();
            socket
                .connect(&common::zmq_url(SOCKET_KIND).unwrap())
                .unwrap();

            socket
        }
    }

    impl Client {
        /// # Returns
        /// User with the given id.
        /// `None` if the user is not found, or the user manifest can not be read.
        pub fn get(&self, id: ResourceId) -> zmq::Result<Option<User>> {
            let user = self.send(query::User::Info(id).into())?;
            Ok(serde_json::from_value(user).unwrap())
        }

        /// # Returns
        /// Projects associated to the user.
        /// Only includes projects whose settings can be read.
        pub fn projects(
            &self,
            user: ResourceId,
        ) -> zmq::Result<Vec<(PathBuf, state::ProjectData)>> {
            let projects = self.send(query::User::Projects(user).into())?;
            Ok(serde_json::from_value(projects).unwrap())
        }
    }
}

mod project {
    use super::Config;
    use crate::{
        common, query,
        state::{self, Analysis},
        Query,
    };
    use serde_json::Value as JsValue;
    use std::path::PathBuf;
    use syre_core::types::ResourceId;

    pub struct Client {
        config: Config,
        zmq_context: zmq::Context,
    }

    impl Client {
        pub(super) fn new(config: Config, zmq_context: zmq::Context) -> Self {
            Self {
                config,
                zmq_context,
            }
        }

        fn send(&self, query: Query) -> zmq::Result<JsValue> {
            // TODO: May be able to move creation of `socket` to `#new`, but may run into `Sync` issues.
            let socket = self.socket();
            socket.send(&serde_json::to_string(&query).unwrap(), 0)?;

            let mut msg = zmq::Message::new();
            socket.recv(&mut msg, 0)?;

            Ok(serde_json::from_str(msg.as_str().unwrap()).unwrap())
        }

        fn socket(&self) -> zmq::Socket {
            const SOCKET_KIND: zmq::SocketType = zmq::REQ;
            let socket = self.zmq_context.socket(SOCKET_KIND).unwrap();
            socket
                .set_connect_timeout(self.config.connect_timeout())
                .unwrap();
            socket.set_rcvtimeo(self.config.recv_timeout()).unwrap();
            socket
                .connect(&common::zmq_url(SOCKET_KIND).unwrap())
                .unwrap();

            socket
        }
    }

    impl Client {
        /// # Returns
        /// State of the project at the given path.
        /// `None` if no state is associated with the path.
        pub fn get(&self, path: impl Into<PathBuf>) -> zmq::Result<Option<state::Project>> {
            let project = self.send(query::Project::Get(path.into()).into())?;
            Ok(serde_json::from_value(project).unwrap())
        }

        /// # Returns
        /// Base path and state of the project at the given path.
        /// `None` if no state is associated with the path.
        pub fn get_by_id(
            &self,
            project: ResourceId,
        ) -> zmq::Result<Option<(PathBuf, state::ProjectData)>> {
            let project_data = self.send(query::Project::GetById(project.clone()).into())?;
            let project_data =
                serde_json::from_value::<Option<(PathBuf, state::ProjectData)>>(project_data)
                    .unwrap();

            if let Some((_, state)) = project_data.as_ref() {
                assert_eq!(state.properties().unwrap().rid(), &project);
            }

            Ok(project_data)
        }

        /// # Returns
        /// Path of the project with the given id.
        pub fn path(&self, project: ResourceId) -> zmq::Result<Option<PathBuf>> {
            let path = self.send(query::Project::Path(project).into())?;
            let path = serde_json::from_value::<Option<PathBuf>>(path).unwrap();
            Ok(path)
        }

        /// # Returns
        /// State of the projects at the given paths.
        /// Paths without an associated state are ommitted from the result.
        pub fn get_many(&self, paths: Vec<PathBuf>) -> zmq::Result<Vec<state::Project>> {
            let user = self.send(query::Project::GetMany(paths).into())?;
            Ok(serde_json::from_value(user).unwrap())
        }

        /// # Returns
        /// Project's path, data, and graph.
        /// `None` if a state is not associated with the project.
        pub fn resources(
            &self,
            project: ResourceId,
        ) -> zmq::Result<
            Option<(
                PathBuf,
                state::ProjectData,
                state::FolderResource<state::Graph>,
            )>,
        > {
            let user = self.send(query::Project::Resources(project).into())?;
            Ok(serde_json::from_value(user).unwrap())
        }
    }
}

mod container {
    use super::Config;
    use crate::{common, error, query, state, Query};
    use serde_json::Value as JsValue;
    use std::path::PathBuf;
    use syre_core::types::ResourceId;

    pub struct Client {
        config: Config,
        zmq_context: zmq::Context,
    }

    impl Client {
        pub(super) fn new(config: Config, zmq_context: zmq::Context) -> Self {
            Self {
                config,
                zmq_context,
            }
        }

        fn send(&self, query: Query) -> zmq::Result<JsValue> {
            // TODO: May be able to move creation of `socket` to `#new`, but may run into `Sync` issues.
            let socket = self.socket();
            socket.send(&serde_json::to_string(&query).unwrap(), 0)?;

            let mut msg = zmq::Message::new();
            socket.recv(&mut msg, 0)?;

            Ok(serde_json::from_str(msg.as_str().unwrap()).unwrap())
        }

        fn socket(&self) -> zmq::Socket {
            const SOCKET_KIND: zmq::SocketType = zmq::REQ;
            let socket = self.zmq_context.socket(SOCKET_KIND).unwrap();
            socket
                .set_connect_timeout(self.config.connect_timeout())
                .unwrap();
            socket.set_rcvtimeo(self.config.recv_timeout()).unwrap();
            socket
                .connect(&common::zmq_url(SOCKET_KIND).unwrap())
                .unwrap();

            socket
        }
    }

    impl Client {
        /// Retrieve the state of a container.
        ///
        /// # Arguments
        /// 1. `project`: Project id.
        /// 2. `container`: Absolute path to the container from the data root.
        ///
        /// # Returns
        /// `None` if the project or container does not exist.
        pub fn get(
            &self,
            project: ResourceId,
            container: impl Into<PathBuf>,
        ) -> zmq::Result<Result<Option<state::Container>, error::InvalidPath>> {
            let state = self.send(
                query::Container::Get {
                    project,
                    container: container.into(),
                }
                .into(),
            )?;

            Ok(serde_json::from_value(state).unwrap())
        }

        /// Retrieve the state of a container.
        ///
        /// # Arguments
        /// 1. `project`: Project id.
        /// 2. `container`: Container id.
        ///
        /// # Returns
        /// `None` if the container project or does not exist.
        pub fn get_by_id(
            &self,
            project: ResourceId,
            container: ResourceId,
        ) -> zmq::Result<Option<state::Container>> {
            let state = self.send(query::Container::GetById { project, container }.into())?;
            Ok(serde_json::from_value(state).unwrap())
        }
    }
}

#[derive(Clone)]
struct Config {
    connect_timeout: i32,
    recv_timeout: i32,
}

impl Config {
    pub fn new(connect_timeout: i32, recv_timeout: i32) -> Self {
        Self {
            connect_timeout,
            recv_timeout,
        }
    }

    pub fn connect_timeout(&self) -> i32 {
        self.connect_timeout
    }

    pub fn recv_timeout(&self) -> i32 {
        self.recv_timeout
    }
}
