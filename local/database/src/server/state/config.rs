//! Application configuration state.
use crate::state::{ConfigState, ManifestState};
pub use action::Action;
use std::path::PathBuf;
use syre_core::system::User;
use syre_local::{system::resources::Config as LocalConfig, Reducible};

/// Application config state.
#[derive(Debug)]
pub struct State {
    /// Users.
    user_manifest: ManifestState<User>,

    /// Project paths.
    project_manifest: ManifestState<PathBuf>,

    /// Project paths.
    local_config: ConfigState<LocalConfig>,
}

impl State {
    pub fn new(
        user_manifest: ManifestState<User>,
        project_manifest: ManifestState<PathBuf>,
        local_config: ConfigState<LocalConfig>,
    ) -> Self {
        Self {
            user_manifest,
            project_manifest,
            local_config,
        }
    }

    pub fn user_manifest(&self) -> &ManifestState<User> {
        &self.user_manifest
    }

    pub fn project_manifest(&self) -> &ManifestState<PathBuf> {
        &self.project_manifest
    }

    pub fn local_config(&self) -> &ConfigState<LocalConfig> {
        &self.local_config
    }
}

impl Reducible for State {
    type Action = Action;
    fn reduce(&mut self, action: Self::Action) {
        match action {
            Action::UserManifest(action) => match action {
                action::DataResource::SetOk(value) => self.user_manifest = ManifestState::Ok(value),
                action::DataResource::SetErr(err) => self.user_manifest = ManifestState::Err(err),
            },
            Action::ProjectManifest(action) => match action {
                action::DataResource::SetOk(value) => {
                    self.project_manifest = ManifestState::Ok(value)
                }
                action::DataResource::SetErr(err) => {
                    self.project_manifest = ManifestState::Err(err)
                }
            },
            Action::LocalConfig(action) => match action {
                action::DataResource::SetOk(value) => self.local_config = ConfigState::Ok(value),
                action::DataResource::SetErr(err) => self.local_config = ConfigState::Err(err),
            },
        }
    }
}

pub mod action {
    use std::path::PathBuf;
    use syre_core::system::User;
    use syre_local::{error::IoSerde, system::resources::Config as LocalConfig};

    #[derive(Debug)]
    pub enum Action {
        UserManifest(DataResource<Vec<User>>),
        ProjectManifest(DataResource<Vec<PathBuf>>),
        LocalConfig(DataResource<LocalConfig>),
    }

    #[derive(Debug)]
    pub enum DataResource<T> {
        SetOk(T),
        SetErr(IoSerde),
    }
}
