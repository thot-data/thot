pub enum Event {
    Command {
        cmd: crate::Command,
        tx: crossbeam::channel::Sender<serde_json::Value>,
    },

    FileSystem(syre_fs_watcher::EventResult),
}
