use crate::{common, Error, Result};
use crossbeam::channel::Sender;

pub struct Query {
    pub query: crate::Query,
    pub tx: Sender<serde_json::Value>,
}

/// Actor to handle queries.
/// Listens for queries on ZMQ channel and reemits them
/// over the transmission channel.
pub struct Actor {
    tx: Sender<Query>,

    /// Reply socket for command requests.
    zmq_socket: zmq::Socket,
}

impl Actor {
    pub fn new(tx: Sender<Query>) -> Self {
        let zmq_context = zmq::Context::new();
        let zmq_socket = zmq_context.socket(zmq::REP).unwrap();
        zmq_socket
            .bind(&common::zmq_url(zmq::REP).unwrap())
            .unwrap();

        Self { tx, zmq_socket }
    }

    pub fn run(&self) -> Result {
        self.listen()?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    fn listen(&self) -> Result {
        loop {
            let query = self.receive_query()?;
            let (tx, rx) = crossbeam::channel::bounded(1);
            self.tx.send(Query { query, tx }).unwrap();

            let res = rx.recv().unwrap();
            self.zmq_socket.send(&res.to_string(), 0)?;
        }
    }

    fn receive_query(&self) -> Result<crate::Query> {
        let mut msg = zmq::Message::new();
        self.zmq_socket
            .recv(&mut msg, 0)
            .expect("could not recieve request");

        let Some(msg_str) = msg.as_str() else {
            let err_msg = "invalid message: could not convert to string";
            tracing::debug!(?err_msg);
            return Err(Error::ZMQ(err_msg.into()));
        };

        let cmd = match serde_json::from_str(msg_str) {
            Ok(cmd) => cmd,
            Err(err) => {
                tracing::debug!(?err, msg = msg_str);
                return Err(Error::ZMQ(format!("{err:?}")));
            }
        };

        Ok(cmd)
    }
}
