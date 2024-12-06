use notify_debouncer_full::DebouncedEvent;

pub type Result<T = ()> = std::result::Result<T, Error>;

#[derive(Debug, derive_more::From)]
pub enum Error {
    /// An error occurred with the underlying watcher.
    Watch(notify::Error),

    /// A file system event could not be processed into an app event.
    ///
    /// # Fields
    /// + `events`: The events that led to the error.
    /// Events may be grouped during processing.
    /// e.g. A `Create` and `Remove` event may be grouped into a `Move` event.
    /// + `kind`: The error that ocurred.
    Processing {
        events: Vec<DebouncedEvent>,
        kind: Process,
    },
}

#[derive(Debug)]
pub enum Process {
    /// Could not distinguish if resource was a file or folder.
    UnknownFileType,

    /// No resource is associated to the event's path.
    NotFound,

    /// Could not canonicalize the path.
    Canonicalize,

    /// The event required a project to be loaded to determine the type of resource it is,
    /// but the associated project could not be loaded.
    LoadProject,

    /// The event created a state that could not be handled.
    InvalidState,
}

impl From<processing::Error> for Process {
    fn from(value: processing::Error) -> Self {
        match value {
            processing::Error::InvalidState(_) => Self::InvalidState,
            processing::Error::LoadProject => Self::LoadProject,
        }
    }
}

pub(crate) mod processing {
    #[derive(Debug)]
    pub enum Error {
        /// The event ocurred in a project that could not be loaded.
        LoadProject,

        /// A state ocurred that could not be handled.
        InvalidState(String),
    }
}
