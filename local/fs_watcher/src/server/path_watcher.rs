//! File system watcher used to check if paths exist.
//! Used because `notify` watchers require the path to
//! exist before wathcing it.
use crossbeam::channel::{Receiver, Sender};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const POLL_INTERVAL: std::time::Duration = Duration::from_millis(2_000);

pub enum Command {
    Watch(PathBuf),
    Unwatch(PathBuf),
}

/// Simple poll watcher to check if paths exist.
/// Emits which watched paths currently exist.
pub struct Watcher {
    command_rx: Receiver<Command>,
    event_tx: Sender<Vec<PathBuf>>,
    paths: Vec<PathBuf>,
    poll_interval: Duration,
    last_poll: Instant,
}

impl Watcher {
    /// Create a new actor to watch the file system.
    /// Uses polling.
    /// Begins watching upon creation.
    pub fn new(event_tx: Sender<Vec<PathBuf>>, command_rx: Receiver<Command>) -> Self {
        Self {
            command_rx,
            event_tx,
            paths: vec![],
            poll_interval: POLL_INTERVAL,
            last_poll: Instant::now(),
        }
    }

    pub fn run(&mut self) {
        loop {
            let cmd: Result<Command, RecvError> = if self.paths.len() == 0 {
                self.command_rx.recv().map_err(|err| err.into())
            } else {
                self.command_rx
                    .recv_timeout(self.poll_interval)
                    .map_err(|err| err.into())
            };

            match cmd {
                Ok(cmd) => {
                    match cmd {
                        Command::Watch(path) => self.watch(path),
                        Command::Unwatch(path) => self.unwatch(path),
                    }

                    if self.last_poll.elapsed() > self.poll_interval {
                        self.poll();
                    }
                }
                Err(RecvError::Timeout) => {
                    self.poll();
                }

                Err(RecvError::Disconnected) => break,
            };
        }

        tracing::debug!("command channel closed, shutting down");
    }

    fn watch(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        tracing::debug!("watching {path:?}");

        self.paths.push(path);
    }

    fn unwatch(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        tracing::debug!("unwatching {path:?}");

        self.paths.retain(|p| p != path);
    }

    fn poll(&mut self) {
        let mut paths = Vec::with_capacity(self.paths.len());
        for path in self.paths.iter() {
            if path.exists() {
                paths.push(path.clone());
            }
        }

        if paths.len() > 0 {
            self.event_tx.send(paths).unwrap();
        }

        self.last_poll = Instant::now();
    }
}

enum RecvError {
    Disconnected,
    Timeout,
}

impl From<crossbeam::channel::RecvTimeoutError> for RecvError {
    fn from(value: crossbeam::channel::RecvTimeoutError) -> Self {
        use crossbeam::channel::RecvTimeoutError;

        match value {
            RecvTimeoutError::Timeout => Self::Timeout,
            RecvTimeoutError::Disconnected => Self::Disconnected,
        }
    }
}

impl From<crossbeam::channel::RecvError> for RecvError {
    fn from(_value: crossbeam::channel::RecvError) -> Self {
        Self::Disconnected
    }
}
