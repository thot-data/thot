use crate::error::AnalysisError;
use crate::types::ResourceId;
use chrono::prelude::*;
use has_id::HasId;
use serde_json::Value as JsValue;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "serde")]
use has_id::HasIdSerde;

// **************
// *** Script ***
// **************

/// Represents a Script belonging to a specific project.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, HasIdSerde))]
#[derive(HasId, PartialEq, Debug, Clone)]
pub struct Script {
    #[id]
    rid: ResourceId,
    pub path: PathBuf,
    pub name: Option<String>,
    pub description: Option<String>,
    pub env: ScriptEnv,
    pub creator: Option<ResourceId>,
    created: DateTime<Utc>,
}

impl Script {
    pub fn new(path: impl Into<PathBuf>, env: ScriptEnv) -> Self {
        Script {
            rid: ResourceId::new(),
            path: path.into(),
            name: None,
            description: None,
            creator: None,
            created: Utc::now(),
            env,
        }
    }

    pub fn from_path(path: impl Into<PathBuf>) -> StdResult<Script, AnalysisError> {
        let path = path.into();
        let Some(file_name) = path.file_name() else {
            return Err(AnalysisError::UnknownLanguage(None));
        };

        let env = ScriptEnv::from_path(Path::new(file_name))?;
        Ok(Script {
            rid: ResourceId::new(),
            path,
            name: None,
            description: None,
            creator: None,
            created: Utc::now(),
            env,
        })
    }

    pub fn rid(&self) -> &ResourceId {
        &self.rid
    }

    /// Returns the date-time the script was created.
    /// This does not refer to the creation date-time of the script file,
    /// but rather the abstract Script object.
    pub fn created(&self) -> &DateTime<Utc> {
        &self.created
    }
}

#[cfg(feature = "runner")]
impl crate::runner::Runnable for Script {
    fn command(&self) -> std::process::Command {
        #[cfg(target_os = "windows")]
        let mut out = std::process::Command::new("cmd");

        #[cfg(target_os = "windows")]
        out.args(["/c", &self.env.cmd]);

        #[cfg(not(target_os = "windows"))]
        let mut out = std::process::Command::new(&self.env.cmd);

        out.arg(self.path.as_path())
            .args(&self.env.args)
            .envs(&self.env.env);

        out
    }
}

// ******************
// *** Script Env ***
// ******************

/// Defines the environment the script should run in.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Debug, Clone)]
pub struct ScriptEnv {
    /// Language of the script.
    pub language: ScriptLang,

    /// Command to run the script.
    pub cmd: String,

    /// Arguments passed to the command (`cmd`).
    pub args: Vec<String>,

    /// Environment variables.
    pub env: HashMap<String, String>,
}

impl ScriptEnv {
    pub fn new(language: ScriptLang, cmd: impl Into<String>) -> Self {
        Self {
            language,
            cmd: cmd.into(),
            args: Vec::new(),
            env: HashMap::new(),
        }
    }

    /// Creates a new script environment for the given script.
    pub fn from_path(script: &Path) -> StdResult<Self, AnalysisError> {
        let path_ext = script.extension();
        if path_ext.is_none() {
            return Err(AnalysisError::UnknownLanguage(None));
        }

        // lang
        let path_ext = path_ext.unwrap();
        let language = ScriptLang::from_extension(path_ext);
        if language.is_none() {
            return Err(AnalysisError::UnknownLanguage(Some(
                path_ext.to_str().unwrap().to_string(),
            )));
        }
        let language = language.unwrap();

        // cmd
        let cmd = match &language {
            ScriptLang::Python => "python3",
            ScriptLang::R => "Rscript",
        }
        .to_string();

        // args
        let args = Vec::new();

        // env
        let env = HashMap::new();

        Ok(ScriptEnv {
            language,
            cmd,
            args,
            env,
        })
    }
}

// *******************
// *** Script Lang ***
// *******************

/// Defines the language of the script.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptLang {
    Python,
    R,
}

impl ScriptLang {
    /// Returns the language type from a file extension
    /// or `None` if none match.
    #[tracing::instrument]
    pub fn from_extension(ext: &OsStr) -> Option<Self> {
        let ext = ext.to_ascii_lowercase();
        let Some(ext) = ext.as_os_str().to_str() else {
            return None;
        };

        match ext {
            "py" => Some(Self::Python),
            "r" => Some(Self::R),
            _ => None,
        }
    }

    /// Returns a list of supported extensions.
    pub fn supported_extensions() -> Vec<&'static str> {
        vec!["py", "r"]
    }
}

// *************************
// *** Script Parameters ***
// *************************

pub type ScriptParameters = HashMap<String, JsValue>;
