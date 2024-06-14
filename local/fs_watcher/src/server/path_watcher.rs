//! File system watcher used to check if paths exist.
//! Used because `notify` watchers require the path to
//! exist before wathcing it.
use crossbeam::channel::{Receiver, RecvTimeoutError, Sender};
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
            match self.command_rx.recv_timeout(self.poll_interval) {
                Ok(cmd) => {
                    match cmd {
                        Command::Watch(path) => self.watch(path),
                        Command::Unwatch(path) => self.unwatch(path),
                    }

                    if self.last_poll.elapsed() > self.poll_interval {
                        self.poll();
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    self.poll();
                }

                Err(RecvTimeoutError::Disconnected) => break,
            };
        }

        tracing::debug!("command channel closed, shutting down");
    }

    fn watch(&mut self, path: impl Into<PathBuf>) {
        self.paths.push(path.into());
    }

    fn unwatch(&mut self, path: impl AsRef<Path>) {
        self.paths.retain(|p| p != path.as_ref());
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
