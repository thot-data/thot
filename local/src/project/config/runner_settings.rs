//! Runner settings.
use crate::common;
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},num::NonZeroUsize, 
};

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Settings {
    /// Path to python executable runner should use.
    pub python_path: Option<PathBuf>,

    /// Path to R executable runner should use.
    pub r_path: Option<PathBuf>,

    /// Continue or halt analysis when an error occurs.
    /// If `None`, defer setting.
    pub continue_on_error: Option<bool>,

    /// Maximum number of tasks to use during analysis.
    pub max_tasks: Option<NonZeroUsize>,
}

impl Settings {
    /// # Arguments
    /// 1. `base_path`: Base path of the project.
    pub fn save(&self, base_path: impl AsRef<Path>) -> Result<(), io::Error> {
        let path = common::project_runner_settings_file_of(base_path);
        fs::create_dir_all(path.parent().expect("invalid project path"))?;
        fs::write(path, serde_json::to_string_pretty(self).unwrap())?;
        Ok(())
    }
}
