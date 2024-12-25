pub(self) mod actions;
pub(self) mod canvas;
pub(self) mod common;
mod layers;
mod project_bar;
mod properties;
mod settings;
mod state;
mod workspace;

pub(self) use canvas::{Canvas, CONTAINER_WIDTH};
pub(self) use layers::LayersNav;
pub(self) use project_bar::ProjectBar;
pub(self) use properties::PropertiesBar;
pub(self) use settings::Settings;
pub use workspace::Workspace;
