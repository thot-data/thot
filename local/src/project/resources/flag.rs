use serde::{Deserialize, Serialize};

pub type Id = uuid::Uuid;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Flag {
    id: Id,
    severity: Severity,
    message: String,
}

impl Flag {
    pub fn new(severity: Severity, message: impl Into<String>) -> Self {
        Self {
            id: Id::now_v7(),
            severity,
            message: message.into(),
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(Severity::Info, message)
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, message)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(Severity::Error, message)
    }

    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn message(&self) -> &String {
        &self.message
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum Severity {
    Info,
    Warning,
    Error,
}
