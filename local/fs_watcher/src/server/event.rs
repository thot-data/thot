//! File system events.
use notify_debouncer_full::DebouncedEvent;
use std::{path::PathBuf, time::Instant};
use uuid::Uuid;

/// Internal server event.
/// These are derived from [`notify_debouncer_full::DebouncedEvent`]s,
/// and are further processed into [`crate::Event`]s.
#[derive(Debug)]
pub struct Event<'a> {
    /// Tracker id.
    id: Uuid,

    /// Tracker ids that led to this event.
    parents: Vec<&'a DebouncedEvent>,

    /// The instant the event was created.
    pub time: Instant,

    pub kind: EventKind,
}

impl Event<'_> {
    pub fn new(kind: impl Into<EventKind>, time: Instant) -> Self {
        Self {
            id: Uuid::now_v7(),
            parents: Vec::new(),
            time,
            kind: kind.into(),
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }
}

impl<'a> Event<'a> {
    pub fn add_parent<'b: 'a>(mut self, parent: &'b DebouncedEvent) -> Self {
        self.parents.push(parent);
        self
    }

    pub fn parents(&self) -> Vec<&'a DebouncedEvent> {
        self.parents.clone()
    }
}

#[derive(Debug, derive_more::From)]
pub enum EventKind {
    File(File),
    Folder(Folder),

    /// Could not determine if the event affect a file, folder, or other resource.
    Any(Any),
}

#[derive(Debug)]
pub enum File {
    Created(PathBuf),
    Removed(PathBuf),

    /// A file's name was changed.
    /// Its base directory is unchanged.
    Renamed {
        from: PathBuf,
        to: PathBuf,
    },

    /// A file was moved to a different folder.
    Moved {
        from: PathBuf,
        to: PathBuf,
    },

    /// The content of the file changed.
    DataModified(PathBuf),

    /// The file was modified, but the type of change could not be determined.
    Other(PathBuf),
}

#[derive(Debug)]
pub enum Folder {
    /// A new folder was created.
    /// This folder may already have contents in it, e.g. if it was pasted in from another location.
    Created(PathBuf),

    Removed(PathBuf),

    /// A folder's name was changed.
    Renamed {
        from: PathBuf,
        to: PathBuf,
    },

    /// A folder was moved to a different parent.
    Moved {
        from: PathBuf,
        to: PathBuf,
    },

    /// The folder was modified, but the type of change could not be determined.
    Other(PathBuf),
}

#[derive(Debug)]
pub enum Any {
    Removed(PathBuf),
}
