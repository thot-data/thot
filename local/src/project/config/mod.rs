pub mod container;
pub mod runner_settings;
pub mod settings;

pub use analysis::Store as AnalysisStore;
pub use container::{
    Assets, Settings as ContainerSettings, StoredProperties as StoredContainerProperties,
};
pub use runner_settings::Settings as RunnerSettings;
pub use settings::Settings;

pub mod analysis {
    use crate::types::AnalysisKind;
    use std::collections::HashMap;
    use syre_core::types::ResourceId;

    pub type Store = HashMap<ResourceId, AnalysisKind>;
}
