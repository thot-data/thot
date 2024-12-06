pub mod container {
    use serde::{Deserialize, Serialize};
    use syre_core::types::ResourceId;

    #[derive(Serialize, Deserialize, Debug)]
    pub enum Action {
        /// Add an analysis association to the Container.
        AddAnalysisAssociation(ResourceId),
    }
}
