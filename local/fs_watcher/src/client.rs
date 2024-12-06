//! Client to communicate with the watcher server.
use crate::Command;
use crossbeam::channel::{self, Sender};
use std::path::PathBuf;

/// Communicate with the file system watcher.
pub struct Client {
    tx: Sender<Command>,
}

impl Client {
    pub fn new(tx: Sender<Command>) -> Self {
        Self { tx }
    }

    /// Watch a path.
    ///
    /// # Arguments
    /// 1. `path`: Path to watch. Must be absolute.
    ///
    /// # Errors
    /// If the given path is not absolute.
    pub fn watch(&self, path: impl Into<PathBuf>) -> Result<(), ()> {
        let path: PathBuf = path.into();
        if !path.is_absolute() {
            return Err(());
        }
        self.tx.send(Command::Watch(path)).unwrap();
        Ok(())
    }

    /// Unwatch a path.
    ///
    /// # Arguments
    /// 1. `path`: Path to watch. Must be absolute.
    ///
    /// # Errors
    /// If the given path is not absolute.
    pub fn unwatch(&self, path: impl Into<PathBuf>) -> Result<(), ()> {
        let path: PathBuf = path.into();
        if !path.is_absolute() {
            return Err(());
        }
        self.tx.send(Command::Unwatch(path)).unwrap();
        Ok(())
    }

    /// Clear all projects from watch list.
    pub fn clear_projects(&self) {
        self.tx.send(Command::ClearProjects).unwrap();
    }

    /// Get the final (canonicalized) path of the given path.
    ///
    /// # Arguments
    /// 1. `path`: Path to canonicalize. Must be absolute.
    pub fn final_path(
        &self,
        path: impl Into<PathBuf>,
    ) -> Result<Option<PathBuf>, error::FinalPath> {
        let path: PathBuf = path.into();
        if !path.is_absolute() {
            return Err(error::FinalPath::InvalidPath);
        }
        let (tx, rx) = channel::bounded(1);
        self.tx.send(Command::FinalPath { path, tx }).unwrap();
        let res = rx.recv().unwrap();
        res.map_err(|err| error::FinalPath::Retrieval(err))
    }

    /// Shutdown the file system watcher.
    ///
    /// # Notes
    /// + Consumes self to close receiver.
    pub fn shutdown(self) {
        self.tx.send(Command::Shutdown).unwrap();
    }
}

pub mod error {
    pub enum FinalPath {
        /// Indicates an invalid path was passed for the operation,
        InvalidPath,

        /// Error retrieving the final path.
        Retrieval(file_path_from_id::Error),
    }
}
