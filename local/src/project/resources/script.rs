//! Local [`Script`].
use crate::system::config::Config;
use crate::Result;
use std::path::PathBuf;
use syre_core::error::Error as CoreError;
use syre_core::project::Script as CoreScript;

pub struct Script;
impl Script {
    /// Creates a new [`Script`] with the `creator` field matching the current active creator.
    pub fn new(path: impl Into<PathBuf>) -> Result<CoreScript> {
        let config = Config::load()?;
        let creator = config.user.clone().map(|c| c.into());

        let mut script = match CoreScript::from_path(path) {
            Ok(script) => script,
            Err(err) => return Err(CoreError::Analysis(err).into()),
        };

        script.creator = creator;
        Ok(script)
    }
}
