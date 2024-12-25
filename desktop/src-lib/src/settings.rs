pub use project::Settings as Project;
pub use user::Settings as User;

pub mod user {
    use serde::{Deserialize, Serialize};
    use std::{io, path::PathBuf};
    use syre_local as local;

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Settings {
        pub desktop: Result<Desktop, local::error::IoSerde>,
        pub runner: Result<Runner, local::error::IoSerde>,
    }

    impl Settings {
        pub fn replace_not_found_with_default(&mut self) {
            if let Err(err) = &self.desktop {
                if matches!(err, local::error::IoSerde::Io(io::ErrorKind::NotFound)) {
                    self.desktop = Ok(Desktop::default());
                }
            }

            if let Err(err) = &self.runner {
                if matches!(err, local::error::IoSerde::Io(io::ErrorKind::NotFound)) {
                    self.runner = Ok(Runner::default());
                }
            }
        }
    }

    impl Default for Settings {
        fn default() -> Self {
            Self {
                desktop: Ok(Default::default()),
                runner: Ok(Default::default()),
            }
        }
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Desktop {
        /// Form input debounce in milliseconds.
        pub input_debounce_ms: usize,
    }

    impl Default for Desktop {
        fn default() -> Self {
            Self {
                input_debounce_ms: 250,
            }
        }
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    pub struct Runner {
        pub python_path: Option<PathBuf>,
        pub r_path: Option<PathBuf>,
        pub continue_on_error: bool,
    }

    impl From<local::system::config::runner_settings::Settings> for Runner {
        fn from(value: local::system::config::runner_settings::Settings) -> Self {
            Self {
                python_path: value.python_path,
                r_path: value.r_path,
                continue_on_error: value.continue_on_error,
            }
        }
    }

    impl Into<local::system::config::runner_settings::Settings> for Runner {
        fn into(self) -> local::system::config::runner_settings::Settings {
            local::system::config::runner_settings::Settings {
                python_path: self.python_path,
                r_path: self.r_path,
                continue_on_error: self.continue_on_error,
            }
        }
    }
}

pub mod project {
    use serde::{Deserialize, Serialize};
    use std::{io, path::PathBuf};
    use syre_local as local;

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Settings {
        pub runner: Result<Runner, local::error::IoSerde>,
    }

    impl Settings {
        pub fn replace_not_found_with_default(&mut self) {
            if let Err(err) = &self.runner {
                if matches!(err, local::error::IoSerde::Io(io::ErrorKind::NotFound)) {
                    self.runner = Ok(Runner::default());
                }
            }
        }
    }

    impl Default for Settings {
        fn default() -> Self {
            Self {
                runner: Ok(Default::default()),
            }
        }
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    pub struct Runner {
        pub python_path: Option<PathBuf>,
        pub r_path: Option<PathBuf>,
        pub continue_on_error:Option<bool>,
    }

    impl From<local::project::config::runner_settings::Settings> for Runner {
        fn from(value: local::project::config::runner_settings::Settings) -> Self {
            Self {
                python_path: value.python_path,
                r_path: value.r_path,
                continue_on_error: value.continue_on_error,
            }
        }
    }

    impl Into<local::project::config::runner_settings::Settings> for Runner {
        fn into(self) -> local::project::config::runner_settings::Settings {
            local::project::config::runner_settings::Settings {
                python_path: self.python_path,
                r_path: self.r_path,
                continue_on_error: self.continue_on_error,
            }
        }
    }
}
