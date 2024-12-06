//! System settings for Syre.
//!
//! This includes modules for tracking
//! + Projects
//! + Scripts
//! + Users
pub mod project_manifest;
// pub mod scripts;
// pub mod templates;
pub mod user_manifest;

// Reexports
pub use project_manifest::ProjectManifest;
// pub use scripts::Scripts;
// pub use templates::Templates;
pub use user_manifest::UserManifest;
