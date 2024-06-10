use crate::{common, Error, Result};
use crossbeam::channel::Sender;

pub struct Command {
    pub cmd: crate::Command,
    pub tx: Sender<serde_json::Value>,
}

/// Actor to handle command events.
pub struct CommandActor {
    event_tx: Sender<Command>,

    /// Reply socket for command requests.
    zmq_socket: zmq::Socket,
}

impl CommandActor {
    pub fn new(event_tx: Sender<Command>) -> Self {
        let zmq_context = zmq::Context::new();
        let zmq_socket = zmq_context.socket(zmq::REP).unwrap();
        zmq_socket
            .bind(&common::zmq_url(zmq::REP).unwrap())
            .unwrap();

        Self {
            event_tx,
            zmq_socket,
        }
    }

    pub fn run(&self) -> Result {
        self.listen_for_commands()?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    fn listen_for_commands(&self) -> Result {
        loop {
            let cmd = self.receive_command()?;
            let (value_tx, value_rx) = crossbeam::channel::bounded(1);
            self.event_tx.send(Command { cmd, tx: value_tx }).unwrap();

            let res = value_rx.recv().unwrap();
            self.zmq_socket.send(&res.to_string(), 0)?;
        }
    }

    fn receive_command(&self) -> Result<crate::Command> {
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
