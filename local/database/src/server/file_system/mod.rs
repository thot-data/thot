mod analysis;
mod app;
mod asset;
mod container;
mod file;
mod folder;
mod graph;
mod project;

use crate::{Database, Update};
use syre_fs_watcher::EventKind;

impl Database {
    pub fn process_file_system_events(
        &mut self,
        events: Vec<syre_fs_watcher::Event>,
    ) -> Vec<Update> {
        events
            .into_iter()
            .flat_map(|event| self.process_event(event))
            .collect()
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn process_event(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        tracing::trace!(?event);
        match event.kind() {
            EventKind::Config(_) => self.handle_fs_event_config(event),
            EventKind::Project(_) => self.handle_fs_event_project(event),
            EventKind::Graph(_) => self.handle_fs_event_graph(event),
            EventKind::GraphResource(_) => self.handle_fs_event_graph_resource(event),
            EventKind::Container(_) => self.handle_fs_event_container(event),
            EventKind::AssetFile(_) => self.handle_fs_event_asset_file(event),
            EventKind::AnalysisFile(_) => self.handle_fs_event_analysis_file(event),
            EventKind::File(_) => self.handle_fs_event_file(event),
            EventKind::Folder(_) => self.handle_fs_event_folder(event),
            EventKind::Any(_) => todo!(),
            EventKind::OutOfSync => todo!(),
        }
    }
}
