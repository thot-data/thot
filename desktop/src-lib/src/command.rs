//! Shared types between the desktop ui and tauri commands.
pub mod project {
    pub mod error {
        use serde::{Deserialize, Serialize};
        use std::io;
        use syre_core as core;
        use syre_local::error::IoSerde;
        use syre_local_runner as runner;

        #[derive(Serialize, Deserialize, Debug)]
        pub enum Initialize {
            /// The path is not a valid project root path.
            /// This is likely because it contains other or is contained within another project root path(s).
            InvalidRootPath,

            /// Could not register the project in the project manifest.
            ProjectManifest(IoSerde),

            /// Could not intialize the folder as a project.
            Init(String),
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub enum Import {
            /// Could not update the project's settings.
            Settings(IoSerde),

            /// Could not register the project in the project manifest.
            ProjectManifest(IoSerde),
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub enum AnalysesUpdate {
            AnalysesFile(IoSerde),
            RemoveFile(#[serde(with = "io_error_serde::ErrorKind")] io::ErrorKind),
        }

        #[derive(Serialize, Deserialize, derive_more::From, Debug)]
        pub enum Analyze {
            GraphAbsent,
            InvalidGraph,
            RunnerCreation(runner::error::From),
            Analysis(core::runner::error::AnalyzerRun),
        }
    }
}

pub mod analyses {
    pub mod error {
        use super::super::error::IoErrorKind;
        use serde::{Deserialize, Serialize};
        use std::path::PathBuf;
        use syre_local::error::IoSerde;

        #[derive(Serialize, Deserialize, Debug)]
        pub enum AddAnalyses {
            /// Moving the file system resource failed.
            FsResource { path: PathBuf, error: IoErrorKind },

            /// Updating the project's analyses failed.
            UpdateAnalyses(IoSerde),
        }
    }
}

pub mod graph {
    pub mod error {
        use super::super::error::IoErrorKind;
        use serde::{Deserialize, Serialize};
        use std::path::PathBuf;

        #[derive(Serialize, Deserialize, Debug)]
        pub enum LoadTree {
            /// The tree's root resource could not be accessed.
            Root(IoErrorKind),

            /// The tree could not be loaded normally.
            State,

            /// An ignore file could not be read correctly.
            Ignore(PathBuf),
        }

        pub mod duplicate {
            use super::super::super::error::IoErrorKind;
            use serde::{Deserialize, Serialize};
            use std::path::PathBuf;
            use syre_local::error::IoSerde;

            #[derive(Serialize, Deserialize, Debug)]
            pub enum Error {
                /// Creating a unique file name for the duplicate root failed.
                Filename(IoErrorKind),

                /// Creating a temporary directory in which to duplicate the tree failed.
                Tmp(IoErrorKind),

                /// Duplicating the tree failed.
                Duplicate(Vec<(PathBuf, Duplicate)>),

                /// Relocating the duplicated tree to its final dstination failed.
                Move(IoErrorKind),
            }

            #[derive(Serialize, Deserialize, Debug)]
            pub enum Duplicate {
                /// Loading the parent failed.
                Load {
                    properties: Option<IoSerde>,
                    settings: Option<IoSerde>,
                },

                /// Saving the child failed.
                Save(SaveContainer),
            }

            #[derive(Serialize, Deserialize, Debug)]
            pub enum SaveContainer {
                CreateDir(IoErrorKind),
                SaveFiles {
                    properties: Option<IoErrorKind>,
                    assets: Option<IoErrorKind>,
                    settings: Option<IoErrorKind>,
                },
            }
        }
    }
}

pub mod container {
    pub mod bulk {
        use super::super::{
            bulk::{MetadataAction, TagsAction},
            serde_opt_opt_str,
        };
        use serde::{Deserialize, Serialize};
        use syre_core::{project::AnalysisAssociation, types::ResourceId};

        #[derive(Serialize, Deserialize, Default, Debug)]
        pub struct PropertiesUpdate {
            pub name: Option<String>,

            #[serde(with = "serde_opt_opt_str")]
            pub kind: Option<Option<String>>,

            #[serde(with = "serde_opt_opt_str")]
            pub description: Option<Option<String>>,
            pub tags: TagsAction,
            pub metadata: MetadataAction,
        }

        #[derive(Serialize, Deserialize, Default, Debug)]
        pub struct AnalysisAssociationAction {
            /// Add new associations, ignore if already present.
            pub add: Vec<AnalysisAssociation>,

            /// Update existing associations, ignore if not present.
            pub update: Vec<AnalysisAssociationUpdate>,

            /// Remove existing associations, ignore if not present.
            pub remove: Vec<ResourceId>,
        }

        #[derive(Serialize, Deserialize, Clone, Debug)]
        pub struct AnalysisAssociationUpdate {
            analysis: ResourceId,
            pub autorun: Option<bool>,
            pub priority: Option<i32>,
        }

        impl AnalysisAssociationUpdate {
            pub fn new(analysis: ResourceId) -> Self {
                Self {
                    analysis,
                    autorun: None,
                    priority: None,
                }
            }

            pub fn analysis(&self) -> &ResourceId {
                &self.analysis
            }
        }

        pub mod error {
            use serde::{Deserialize, Serialize};
            use std::{io, path::PathBuf};
            use syre_local::error::IoSerde;

            /// Error renaming container.
            #[derive(Serialize, Deserialize, Debug)]
            pub enum Rename {
                ProjectNotFound,

                /// Renaming would cause a name collision.
                NameCollision(Vec<PathBuf>),
            }

            /// Error updating containers.
            #[derive(Serialize, Deserialize, Debug)]
            pub enum Update {
                Load(IoSerde),
                Save(#[serde(with = "io_error_serde::ErrorKind")] io::ErrorKind),
            }
        }
    }

    pub mod error {
        use serde::{Deserialize, Serialize};
        use std::io;
        use syre_local::error::IoSerde;

        /// Error renaming container.
        #[derive(Serialize, Deserialize, Debug)]
        pub enum Rename {
            ProjectNotFound,
            NameCollision,
            Rename(#[serde(with = "io_error_serde::ErrorKind")] io::ErrorKind),

            /// Could not update project's data root.
            /// Only applicable when renaming the root node.
            DataRoot(#[serde(with = "io_error_serde::ErrorKind")] io::ErrorKind),
        }

        /// Error updating container.
        #[derive(Serialize, Deserialize, Debug)]
        pub enum Update {
            ProjectNotFound,
            Load(IoSerde),
            Save(#[serde(with = "io_error_serde::ErrorKind")] io::ErrorKind),
        }
    }
}

pub mod asset {
    pub mod bulk {
        use std::path::PathBuf;

        use super::super::{
            bulk::{MetadataAction, TagsAction},
            serde_opt_opt_str,
        };
        use serde::{Deserialize, Serialize};
        use syre_core::types::ResourceId;

        #[derive(Serialize, Deserialize, Default, Debug)]
        pub struct ContainerAssets {
            pub container: PathBuf,
            pub assets: Vec<ResourceId>,
        }

        impl From<(PathBuf, Vec<ResourceId>)> for ContainerAssets {
            fn from((container, assets): (PathBuf, Vec<ResourceId>)) -> Self {
                Self { container, assets }
            }
        }

        #[derive(Serialize, Deserialize, Default, Debug)]
        pub struct PropertiesUpdate {
            #[serde(with = "serde_opt_opt_str")]
            pub name: Option<Option<String>>,

            #[serde(with = "serde_opt_opt_str")]
            pub kind: Option<Option<String>>,

            #[serde(with = "serde_opt_opt_str")]
            pub description: Option<Option<String>>,
            pub tags: TagsAction,
            pub metadata: MetadataAction,
        }

        pub mod error {
            use serde::{Deserialize, Serialize};
            use std::io;
            use syre_core::types::ResourceId;
            use syre_local::error::IoSerde;

            /// Error updating containers.
            #[derive(Serialize, Deserialize, Debug)]
            pub enum Update {
                Load(IoSerde),
                NotFound(Vec<ResourceId>),
                Save(#[serde(with = "io_error_serde::ErrorKind")] io::ErrorKind),
            }
        }
    }

    pub mod error {
        use serde::{Deserialize, Serialize};
        use std::io;
        use syre_local::error::IoSerde;

        /// Error updating asset.
        #[derive(Serialize, Deserialize, Debug)]
        pub enum Update {
            ProjectNotFound,
            Load(IoSerde),
            Save(#[serde(with = "io_error_serde::ErrorKind")] io::ErrorKind),
        }
    }
}

pub mod bulk {
    use super::serde_opt_opt_str;
    use serde::{Deserialize, Serialize};
    use syre_core::types::Value;

    #[derive(Serialize, Deserialize, Default, Debug)]
    pub struct TagsAction {
        pub insert: Vec<String>,
        pub remove: Vec<String>,
    }

    #[derive(Serialize, Deserialize, Default, Debug)]
    pub struct MetadataAction {
        /// Add new data, ignore if already present.
        pub add: Vec<(String, Value)>,

        /// Update existing data, ignore if not present.
        pub update: Vec<(String, Value)>,

        /// Remove data, ignore if not present.
        pub remove: Vec<String>,
    }

    #[derive(Serialize, Deserialize, Default, Debug)]
    pub struct PropertiesUpdate {
        #[serde(with = "serde_opt_opt_str")]
        pub kind: Option<Option<String>>,

        #[serde(with = "serde_opt_opt_str")]
        pub description: Option<Option<String>>,
        pub tags: TagsAction,
        pub metadata: MetadataAction,
    }
}

pub mod error {
    use serde::{Deserialize, Serialize};
    use std::{io, path::PathBuf};

    #[derive(Serialize, Deserialize, Debug)]
    pub struct ProjectNotFound;

    /// [`std::io::ErrorKind`] wrapper to allow for serialization.
    #[derive(Serialize, Deserialize, derive_more::From, Debug)]
    pub struct IoErrorKind(#[serde(with = "io_error_serde::ErrorKind")] pub io::ErrorKind);
    impl Into<io::ErrorKind> for IoErrorKind {
        fn into(self) -> io::ErrorKind {
            self.0
        }
    }
    impl From<io::Error> for IoErrorKind {
        fn from(value: io::Error) -> Self {
            Self(value.kind())
        }
    }

    /// `trash::Error` wrapper to allow for serialization.
    #[derive(Serialize, Deserialize, Debug)]
    pub enum Trash {
        Unknown {
            description: String,
        },

        Os {
            code: i32,
            description: String,
        },

        #[cfg(all(
            unix,
            not(target_os = "macos"),
            not(target_os = "ios"),
            not(target_os = "android")
        ))]
        FileSystem {
            path: PathBuf,
            // source: std::io::Error,
        },

        TargetedRoot,

        CouldNotAccess {
            target: String,
        },

        CanonicalizePath {
            original: PathBuf,
        },
    }

    #[cfg(feature = "server")]
    impl From<trash::Error> for Trash {
        fn from(value: trash::Error) -> Self {
            match value {
                trash::Error::Unknown { description } => Self::Unknown { description },
                trash::Error::Os { code, description } => Self::Os { code, description },
                trash::Error::TargetedRoot => Self::TargetedRoot,
                trash::Error::CouldNotAccess { target } => Self::CouldNotAccess { target },
                trash::Error::CanonicalizePath { original } => Self::CanonicalizePath { original },
                trash::Error::ConvertOsString { .. } => todo!(),
                trash::Error::RestoreCollision { .. } | trash::Error::RestoreTwins { .. } => {
                    unreachable!("should not occur")
                }
            }
        }
    }
}

mod serde_opt_opt_str {
    use serde::{de, Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<Option<String>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            None => serializer.serialize_none(),
            Some(opt_str) => match opt_str {
                None => serializer.serialize_str(""),
                Some(val) => serializer.serialize_str(&val),
            },
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let visitor = OptOptStrVisitor {};
        deserializer.deserialize_option(visitor)
    }

    struct OptOptStrVisitor;
    impl<'de> de::Visitor<'de> for OptOptStrVisitor {
        type Value = Option<Option<String>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an optional string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.is_empty() {
                Ok(Some(None))
            } else {
                Ok(Some(Some(v.to_string())))
            }
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            let visitor = OptOptStrVisitor {};
            deserializer.deserialize_str(visitor)
        }
    }
}
