mod app;

use crate::{event::Update, Database};
use syre_fs_watcher::EventKind;

impl Database {
    pub fn process_events(&mut self, events: Vec<syre_fs_watcher::Event>) -> Vec<Update> {
        events
            .into_iter()
            .flat_map(|event| self.process_event(event))
            .collect()
    }

    fn process_event(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        match event.kind() {
            EventKind::Config(_) => self.handle_fs_event_config(event),
            EventKind::Project(_) => todo!(),
            EventKind::Graph(_) => todo!(),
            EventKind::Container(_) => todo!(),
            EventKind::AssetFile(_) => todo!(),
            EventKind::AnalysisFile(_) => todo!(),
            EventKind::File(_) => todo!(),
            EventKind::Folder(_) => todo!(),
            EventKind::Any(_) => todo!(),
            EventKind::OutOfSync => todo!(),
        }
    }
}
