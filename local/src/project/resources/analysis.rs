//! Local [`Script`].
use super::super::config::analysis::Store;
use crate::{common::analyses_file, error, file_resource::LocalResource, types::AnalysisKind};
use serde::Serialize;
use std::{
    fs, io,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    result::Result as StdResult,
};
use syre_core::{
    error::Resource as ResourceError, project::Script, types::resource_map::values_only,
};

#[derive(Serialize, Clone, PartialEq, Debug)]
#[serde(transparent)]
pub struct Analyses {
    #[serde(skip)]
    base_path: PathBuf,

    #[serde(with = "values_only")]
    inner: Store,
}

impl Analyses {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: path.into(),
            inner: Store::new(),
        }
    }

    pub fn new_with(path: impl Into<PathBuf>, analyses: Vec<AnalysisKind>) -> Self {
        let inner = analyses
            .into_iter()
            .map(|analysis| {
                let rid = match analysis {
                    AnalysisKind::Script(ref script) => script.rid().clone(),
                    AnalysisKind::ExcelTemplate(ref template) => template.rid().clone(),
                };

                (rid, analysis)
            })
            .collect();

        Self {
            base_path: path.into(),
            inner,
        }
    }

    pub fn load_from(base_path: impl Into<PathBuf>) -> StdResult<Self, error::IoSerde> {
        let base_path = base_path.into();
        let path = base_path.join(Self::rel_path());
        let fh = fs::OpenOptions::new().read(true).open(path)?;
        let inner = serde_json::from_reader::<_, Vec<AnalysisKind>>(fh)?;
        let inner = inner
            .into_iter()
            .map(|analysis| {
                let rid = match &analysis {
                    AnalysisKind::Script(script) => script.rid().clone(),
                    AnalysisKind::ExcelTemplate(template) => template.rid().clone(),
                };

                (rid, analysis)
            })
            .collect();

        Ok(Self { base_path, inner })
    }

    pub fn save(&self) -> StdResult<(), io::Error> {
        fs::write(self.path(), serde_json::to_string_pretty(&self).unwrap())?;
        Ok(())
    }

    pub fn scripts(&self) -> Vec<&Script> {
        self.values()
            .filter_map(|analysis| match analysis {
                AnalysisKind::Script(analysis) => Some(analysis),
                AnalysisKind::ExcelTemplate(_) => None,
            })
            .collect()
    }

    /// Inserts a script only if its path isn't yet in the collection.
    ///
    /// # Errors
    /// + [`ResourceError::AlreadyExists`] if a script with the same path is
    /// already present.
    pub fn insert_script_unique_path(&mut self, script: Script) -> StdResult<(), ResourceError> {
        if self.scripts_contain_path(&script.path) {
            return Err(ResourceError::already_exists(
                "`Script` with same path is already present",
            ));
        }

        self.insert(script.rid().clone(), script.into());

        Ok(())
    }

    /// Returns whether a script with the given path is registered.
    pub fn scripts_contain_path(&self, path: impl AsRef<Path>) -> bool {
        self.script_by_path(path).is_some()
    }

    /// Gets a script by its path if it is registered.
    pub fn script_by_path(&self, path: impl AsRef<Path>) -> Option<&Script> {
        let path = path.as_ref();
        for script in self.scripts() {
            if script.path == path {
                return Some(script);
            }
        }

        None
    }

    /// Consumes `self`, returning the underlying `Vec`.
    pub fn to_vec(self) -> Vec<AnalysisKind> {
        self.inner.into_values().collect()
    }
}

impl Deref for Analyses {
    type Target = Store;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Analyses {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl LocalResource<Store> for Analyses {
    fn rel_path() -> PathBuf {
        analyses_file()
    }

    fn base_path(&self) -> &Path {
        &self.base_path
    }
}
