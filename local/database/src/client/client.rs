//! Client to interact with a [`Database`].
use crate::command::Command;
use crate::command::{Command as DbCommand, DatabaseCommand};
use crate::constants::{DATABASE_ID, REQ_REP_PORT};
use crate::types::PortNumber;
use serde_json::Value as JsValue;
use std::net::TcpListener;
use thot_core::db::resources::StandardSearchFilter;
use thot_core::project::{Asset as CoreAsset, Container as CoreContainer};
use thot_core::types::ResourceMap;

pub struct Client {
    zmq_context: zmq::Context,
}

impl Client {
    pub fn new() -> Self {
        Client {
            zmq_context: zmq::Context::new(),
        }
    }

    pub fn send(&self, cmd: Command) -> JsValue {
        let req_socket = self
            .zmq_context
            .socket(zmq::SocketType::REQ)
            .expect("could not create `REQ` socket");

        req_socket
            .connect(&format!("tcp://0.0.0.0:{REQ_REP_PORT}"))
            .expect("socket could not connect");

        req_socket
            .send(
                &serde_json::to_string(&cmd).expect("could not convert `Command` to JSON"),
                0,
            )
            .expect("socket could not send message");

        let mut msg = zmq::Message::new();
        req_socket
            .recv(&mut msg, 0)
            .expect("socket could not recieve `Message`");

        serde_json::from_str(
            msg.as_str()
                .expect("could not interpret `Message` as string"),
        )
        .expect("could not convert `Message` to JsValue")
    }

    pub fn containers_where(filter: StandardSearchFilter) -> ResourceMap<CoreContainer> {
        todo!();
    }

    pub fn assets_where(filter: StandardSearchFilter) -> ResourceMap<CoreAsset> {
        todo!();
    }

    /// Checks if a database is running.
    pub fn server_available() -> bool {
        // check if port is occupied
        if port_is_free(REQ_REP_PORT) {
            // port is open, no chance of a listener
            return false;
        }

        let ctx = zmq::Context::new();
        let req_socket = ctx
            .socket(zmq::SocketType::REQ)
            .expect("could not create socket");

        req_socket
            .connect(&format!("tcp://0.0.0.0:{REQ_REP_PORT}"))
            .expect("socket could not connect");

        req_socket
            .send(
                &serde_json::to_string(&DbCommand::DatabaseCommand(DatabaseCommand::Id))
                    .expect("could not serialize `Command`"),
                0,
            )
            .expect("could not send `Id` command");

        let mut msg = zmq::Message::new();
        req_socket
            .recv(&mut msg, 0)
            .expect("could not recieve `Message`");

        let Some(id_str) = msg.as_str() else {
        panic!("invalid response");
    };

        return id_str == DATABASE_ID;
    }
}

/// Checks if a given port on `0.0.0.0` is free.
fn port_is_free(port: PortNumber) -> bool {
    TcpListener::bind(format!("0.0.0.0:{port}")).is_ok()
}

#[cfg(test)]
#[path = "./client_test.rs"]
mod client_test;
