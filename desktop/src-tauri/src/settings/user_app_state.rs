//! Application state for startup.
use crate::common;
use crate::error::{DesktopSettingsError, Result};
use std::fs;
use std::io::BufReader;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use thot_core::types::ResourceId;
use thot_desktop_lib::settings::UserSettingsFile;
use thot_desktop_lib::settings::{HasUser, UserAppState as DesktopUserAppState};
use thot_local::file_resource::UserResource;

pub struct UserAppState {
    rel_path: PathBuf,
    app_state: DesktopUserAppState,
}

impl UserAppState {
    pub fn load(user: &ResourceId) -> Result<Self> {
        let rel_path = PathBuf::from(user.to_string());
        let rel_path = rel_path.join(Self::settings_file());

        let path = Self::base_path().join(&rel_path);
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let app_state = serde_json::from_reader(reader)?;

        Ok(Self {
            rel_path: rel_path.into(),
            app_state,
        })
    }

    pub fn save(&self) -> Result {
        let fh = fs::OpenOptions::new().write(true).open(self.path())?;
        Ok(serde_json::to_writer_pretty(fh, &self.app_state)?)
    }

    /// Updates the app state.
    pub fn update(&mut self, app_state: DesktopUserAppState) -> Result {
        // verify correct user
        if app_state.user() != self.app_state.user() {
            return Err(
                DesktopSettingsError::InvalidUpdate("users do not match".to_string()).into(),
            );
        }

        self.app_state = app_state;
        Ok(())
    }
}

impl Deref for UserAppState {
    type Target = DesktopUserAppState;

    fn deref(&self) -> &Self::Target {
        &self.app_state
    }
}

impl Into<DesktopUserAppState> for UserAppState {
    fn into(self) -> DesktopUserAppState {
        self.app_state
    }
}

impl UserResource<DesktopUserAppState> for UserAppState {
    fn base_path() -> PathBuf {
        common::users_config_dir().expect("could not get config path")
    }

    fn rel_path(&self) -> &Path {
        &self.rel_path
    }
}

impl UserSettingsFile for UserAppState {
    fn settings_file() -> PathBuf {
        PathBuf::from("desktop_app_state.json")
    }
}

#[cfg(test)]
#[path = "./user_app_state_test.rs"]
mod user_app_state_test;
