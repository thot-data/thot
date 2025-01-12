//! App state.
use crate::fs_action;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use syre_core::types::ResourceId;
use syre_local_database as db;

/// App state.
pub struct State {
    /// Active user.
    user: Slice<Option<User>>,

    /// Pending actions.
    actions: Slice<Vec<fs_action::Action>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            user: new_slice(None),
            actions: new_slice(vec![]),
        }
    }

    pub fn user(&self) -> Slice<Option<User>> {
        self.user.clone()
    }

    pub fn actions(&self) -> &Slice<Vec<fs_action::Action>> {
        &self.actions
    }
}

#[derive(Debug)]
pub struct User {
    rid: ResourceId,

    /// Project paths associated with the current user.
    projects: Slice<Vec<PathBuf>>,
}

impl User {
    pub fn new(user: ResourceId, projects: Vec<PathBuf>) -> Self {
        Self {
            rid: user,
            projects: new_slice(projects),
        }
    }

    pub fn rid(&self) -> &ResourceId {
        &self.rid
    }

    pub fn projects(&self) -> Slice<Vec<PathBuf>> {
        self.projects.clone()
    }
}

pub fn load_user_state(db: &db::Client, user: &ResourceId) -> Vec<PathBuf> {
    db.user()
        .projects(user.clone())
        .unwrap()
        .into_iter()
        .map(|(path, _)| path)
        .collect()
}

/// Slice of the state.
///
/// `Arc<Mutex<T>>` alias for convenience.
pub type Slice<T> = Arc<Mutex<T>>;
pub fn new_slice<T>(obj: T) -> Slice<T> {
    Arc::new(Mutex::new(obj))
}

pub enum AnalyzerAction {
    Cancel,
    Kill,
}
