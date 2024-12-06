//! Syre application events.
use std::{path::PathBuf, time::Instant};
use uuid::Uuid;

pub type EventResult = Result<Vec<Event>, Vec<crate::Error>>;

#[derive(Debug)]
pub struct Event {
    id: Uuid,
    parent: ParentId,

    /// Instant the underlying event was created.
    time: Instant,

    kind: EventKind,

    paths: Vec<PathBuf>,
}

impl Event {
    pub fn new(kind: EventKind, parent: Uuid) -> Self {
        Self {
            id: Uuid::now_v7(),
            parent: ParentId::Fs(parent),
            time: Instant::now(),
            kind,
            paths: Vec::new(),
        }
    }

    pub fn with_time(kind: EventKind, time: Instant, parent: Uuid) -> Self {
        Self {
            id: Uuid::now_v7(),
            parent: ParentId::Fs(parent),
            time,
            kind,
            paths: Vec::new(),
        }
    }

    pub fn new_from_notify(kind: EventKind, parent: Option<usize>) -> Self {
        Self {
            id: Uuid::now_v7(),
            parent: ParentId::Notify(parent),
            time: Instant::now(),
            kind,
            paths: Vec::new(),
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn time(&self) -> &Instant {
        &self.time
    }

    pub fn kind(&self) -> &EventKind {
        &self.kind
    }

    pub fn paths(&self) -> &Vec<PathBuf> {
        &self.paths
    }

    /// Sets `paths` to a single path.
    pub fn add_path(mut self, path: PathBuf) -> Self {
        self.paths.push(path);
        self
    }
}

#[derive(Debug)]
pub enum ParentId {
    Fs(Uuid),
    Notify(Option<usize>),
}

#[derive(Debug, derive_more::From)]
pub enum EventKind {
    /// An application config resource was modified.
    #[from]
    Config(Config),

    /// A project resource was modified.
    #[from]
    Project(Project),

    #[from]
    Graph(Graph),

    /// A container or asset, but could not be determined.
    #[from]
    GraphResource(GraphResource),

    #[from]
    Container(Container),

    /// An asset file was modified.
    /// The file may not be assocated with an Asset yet.
    AssetFile(ResourceEvent),

    /// An analysis file was modified.
    /// The file may not be assocated with an Analysis yet.
    AnalysisFile(ResourceEvent),

    /// A file not associated to a resource type was modified.
    File(ResourceEvent),

    /// A folder not associated with a resource type was modified.
    ///
    /// This is the default event in case the resource type could not be determined.
    /// This occurs for instance, when a folder is removed.
    /// It is left to the client application to determine if the folder actually represented a
    /// resource.
    Folder(ResourceEvent),

    /// An unknown resource was modified.
    #[from]
    Any(Any),

    /// Indicates the provided events may not be in sync with the file system any longer.
    /// Programs relying on an in-memory representation of the file system should sync directly
    /// with the file system.
    /// See [`notify::event::Flag::Rescan] for more info.
    OutOfSync,
}

/// A resource that is distinguished by path.
/// i.e. The resource's file is identified by its path.
///
/// This means renaming the file at that path to another name
/// effectively removes that file from begin identified as the resource,
/// and renaming a file to that path effectively creates or modifies the resource.
#[derive(Debug)]
pub enum StaticResourceEvent {
    Created,
    Removed,

    /// The resource was modified.
    /// This could occur for multiple reasons including:
    /// + A file was moved to the resource's path.
    /// + The content of the resource's file was modified.
    Modified(ModifiedKind),
}

/// A resource that is distinguished by its inode (macOS, *nix) or file id (Windows).
/// i.e. The resource's file is the resource its self.
#[derive(Debug)]
pub enum ResourceEvent {
    /// A resource was created.
    ///
    /// # Notes
    /// + If one path is present the app resource was created via a newly created file system resource.
    /// If two paths are present (`[from, to]`) the app resource was created by an existing file system resource moving
    /// into the app resource's path.
    Created,
    Removed,
    Renamed,

    /// The resource file was moved within the same project.
    Moved,

    /// The resource file was moved into another project.
    MovedProject,

    /// The resource was modified.
    /// This could occur for multiple reasons including:
    /// + A file was moved to the resource's path.
    /// + The content of the resource's file was modified.
    Modified(ModifiedKind),
}

#[derive(Debug)]
pub enum ModifiedKind {
    /// The content of the resource was changed.
    Data,

    /// The type of modification could not be determined.
    /// The resource likely needs to be rescanned.
    Other,
}

/// App config events.
#[derive(Debug)]
pub enum Config {
    /// The app config directory was created.
    Created,

    /// The app config directory was removed.
    Removed,

    /// The app config directory was modified.
    Modified(ModifiedKind),

    /// The project manifest file was modified.
    ProjectManifest(StaticResourceEvent),

    /// The user manifest file was modified.
    UserManifest(StaticResourceEvent),

    /// The local config file was modified.
    LocalConfig(StaticResourceEvent),
}

#[derive(Debug)]
pub enum Project {
    /// The project root directory was created.
    Created,

    /// The project's base folder was deleted.
    FolderRemoved,

    /// The project folder was moved or renamed.
    Moved,

    /// The project's config dir was modified.
    ConfigDir(StaticResourceEvent),

    /// The project's analysis dir was modified.
    AnalysisDir(ResourceEvent),

    /// The project's data dir was modified.
    DataDir(ResourceEvent),

    /// The project's properties file was modified.
    Properties(StaticResourceEvent),

    /// The project's settings file was modified.
    Settings(StaticResourceEvent),

    /// The project's analyses file was modified.
    Analyses(StaticResourceEvent),

    /// The project directory was modified.
    /// It should likely be rescanned.
    Modified,
}

/// Graph events.
///
/// # Notes
/// + The `Removed` event indicates that a folder that could be determined to be a container
/// was removed. This can not always be determined, in which case a [Folder] event is emitted.
#[derive(Debug)]
pub enum Graph {
    /// A directory in a project's data folder was created as a container.
    Created,

    /// A container directory in a project's data folder was removed.
    Removed,

    /// A container directory was moved within the same project.
    /// i.e. The parent directory changed.
    Moved,

    /// A container directory was modified.
    Modified(ModifiedKind),
}

/// A graph resource.
/// i.e. A container or asset, but could not be deteremined.
#[derive(Debug)]
pub enum GraphResource {
    Removed,
}

#[derive(Debug)]
pub enum Container {
    /// The name of the container's folder was changed.
    Renamed,

    /// The container's config dir (i.e. `.syre` folder) was modified.
    ConfigDir(StaticResourceEvent),

    /// The container's properties file was modified.
    Properties(StaticResourceEvent),

    /// The container's settings file was modified.
    Settings(StaticResourceEvent),

    /// The container's assets file was modified.
    Assets(StaticResourceEvent),
}

#[derive(Debug)]
pub enum Any {
    Removed,
}
