pub use project::Settings as Project;
use serde::{de::DeserializeOwned, Serialize};
use std::{fs, io, path::Path};
use syre_local as local;
pub use user::Settings as User;

pub mod user {
    use super::{json_load, json_save};
    use crate::common;
    use serde::{Deserialize, Serialize};
    use std::{io, path::PathBuf};
    use syre_core::types::ResourceId;
    use syre_desktop_lib as lib;
    use syre_local::{self as local, file_resource::UserResource, system::config::runner_settings};

    /// All settings for a user.
    #[derive(Debug)]
    pub struct Settings {
        pub desktop: Result<Desktop, local::error::IoSerde>,
        pub runner: Result<runner_settings::Settings, local::error::IoSerde>,
    }

    impl Settings {
        pub fn load(user: &ResourceId) -> Self {
            let desktop = Desktop::load(user);
            let runner = Runner::load(user);
            Self { desktop, runner }
        }

        // Replaces settings whose files were not found with default values.
        pub fn replace_not_found_with_default(self) -> Self {
            let Self {
                mut desktop,
                mut runner,
            } = self;

            if let Err(local::error::IoSerde::Io(io::ErrorKind::NotFound)) = desktop {
                desktop = Ok(Desktop::default())
            }
            if let Err(local::error::IoSerde::Io(io::ErrorKind::NotFound)) = runner {
                runner = Ok(runner_settings::Settings::default())
            }

            Self { desktop, runner }
        }
    }

    impl Into<lib::settings::User> for Settings {
        fn into(self) -> lib::settings::User {
            lib::settings::User {
                desktop: self.desktop.map(|settings| settings.into()),
                runner: self.runner.map(|settings| settings.into()),
            }
        }
    }

    /// Desktop settings.
    #[derive(Serialize, Deserialize, derive_more::Into, Debug, Clone)]
    pub struct Desktop {
        /// Input element debounce time in milliseconds.
        pub input_debounce_ms: usize,
    }

    impl Desktop {
        const SETTINGS_DIR: &'static str = "settings";

        pub fn new() -> Self {
            Self::default()
        }

        pub fn load(user: &ResourceId) -> Result<Self, local::error::IoSerde> {
            let path = Self::path(user)?;
            json_load(&path)
        }

        pub fn save(&self, user: &ResourceId) -> Result<(), io::Error> {
            let path = Self::path(user)?;
            json_save(&self, path)
        }

        fn path(user: &ResourceId) -> Result<PathBuf, io::Error> {
            let mut file = PathBuf::from(user.to_string());
            file.set_extension("json");

            let base_path = Self::base_path().map_err(|err| err.kind())?;
            Ok(base_path.join(file))
        }

        fn base_path() -> Result<PathBuf, io::Error> {
            let base_path = common::config_dir_path()?;
            Ok(base_path.join(Self::SETTINGS_DIR))
        }
    }

    impl Default for Desktop {
        fn default() -> Self {
            Self {
                input_debounce_ms: 250,
            }
        }
    }

    impl Into<lib::settings::user::Desktop> for Desktop {
        fn into(self) -> lib::settings::user::Desktop {
            lib::settings::user::Desktop {
                input_debounce_ms: self.input_debounce_ms,
            }
        }
    }

    impl From<lib::settings::user::Desktop> for Desktop {
        fn from(value: lib::settings::user::Desktop) -> Self {
            Self {
                input_debounce_ms: value.input_debounce_ms,
            }
        }
    }

    pub struct Runner;
    impl Runner {
        pub fn load(user: &ResourceId) -> Result<runner_settings::Settings, local::error::IoSerde> {
            let path = Self::user_path(user)?;
            json_load(&path)
        }

        pub fn save(
            user: &ResourceId,
            settings: impl Into<runner_settings::Settings>,
        ) -> Result<(), io::Error> {
            let path = Self::user_path(user)?;
            let settings: runner_settings::Settings = settings.into();
            json_save(&settings, &path)
        }

        // Absolute path to the user's settings file.
        fn user_path(user: &ResourceId) -> Result<PathBuf, io::Error> {
            let base_path = runner_settings::RunnerSettings::base_path()?;
            let mut file = PathBuf::from(user.to_string());
            file.set_extension("json");
            Ok(base_path.join(file))
        }
    }
}

pub mod project {
    use super::{json_load, json_save};
    use std::{io, path::Path};
    use syre_desktop_lib as lib;
    use syre_local::{self as local, common, project::config::runner_settings};

    /// All settings for a user.
    #[derive(Debug)]
    pub struct Settings {
        pub runner: Result<runner_settings::Settings, local::error::IoSerde>,
    }

    impl Settings {
        pub fn load(project: impl AsRef<Path>) -> Self {
            let runner = Runner::load(project);
            Self { runner }
        }

        // Replaces settings whose files were not found with default values.
        pub fn replace_not_found_with_default(self) -> Self {
            let Self { mut runner } = self;

            if let Err(local::error::IoSerde::Io(io::ErrorKind::NotFound)) = runner {
                runner = Ok(runner_settings::Settings::default())
            }

            Self { runner }
        }
    }

    impl Into<lib::settings::Project> for Settings {
        fn into(self) -> lib::settings::Project {
            lib::settings::Project {
                runner: self.runner.map(|settings| settings.into()),
            }
        }
    }

    pub struct Runner;
    impl Runner {
        pub fn load(
            project: impl AsRef<Path>,
        ) -> Result<runner_settings::Settings, local::error::IoSerde> {
            let path = common::project_runner_settings_file_of(project);
            json_load(&path)
        }

        pub fn save(
            project: impl AsRef<Path>,
            settings: impl Into<runner_settings::Settings>,
        ) -> Result<(), io::Error> {
            let path = common::project_runner_settings_file_of(project);
            let settings: runner_settings::Settings = settings.into();
            json_save(&settings, &path)
        }
    }
}

/// Loading data from a JSON file.
fn json_load<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, local::error::IoSerde> {
    let file = fs::File::open(path)?;
    let reader = io::BufReader::new(file);
    Ok(serde_json::from_reader(reader)?)
}

/// Save data as JSON to a file.
fn json_save<T: Serialize>(obj: &T, path: impl AsRef<Path>) -> Result<(), io::Error> {
    fs::create_dir_all(path.as_ref().parent().unwrap())?;
    fs::write(path, serde_json::to_string_pretty(obj).unwrap()).map(|_| ())
}
