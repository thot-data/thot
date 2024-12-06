use super::*;
use crate::{
    server::{path_watcher, Config},
    Command, Error, Event,
};
use crossbeam::channel::{Receiver, Sender};
use notify::{
    event::{ModifyKind, RenameMode},
    EventKind as NotifyEventKind,
};
use std::{fs, time::Instant};

// NB: Flaky test. Not sure why.
#[test_log::test]
fn watcher_group_notify_events_should_work() {
    let dir = tempfile::TempDir::new().unwrap();
    let dir_sub = tempfile::TempDir::new_in(dir.path()).unwrap();
    let f_from = tempfile::NamedTempFile::new_in(dir_sub.path()).unwrap();
    let f_any_from = tempfile::NamedTempFile::new_in(dir_sub.path()).unwrap();
    let f_alone = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
    let d_from = tempfile::TempDir::new_in(dir_sub.path()).unwrap();
    let d_any_from = tempfile::TempDir::new_in(dir_sub.path()).unwrap();
    let d_alone = tempfile::TempDir::new_in(dir.path()).unwrap();

    let e_f_from = notify::Event::new(NotifyEventKind::Modify(ModifyKind::Name(RenameMode::From)))
        .add_path(f_from.path().to_path_buf());

    let e_f_any_from =
        notify::Event::new(NotifyEventKind::Modify(ModifyKind::Name(RenameMode::Any)))
            .add_path(f_any_from.path().to_path_buf());

    let e_f_alone = notify::Event::new(NotifyEventKind::Modify(ModifyKind::Name(RenameMode::To)))
        .add_path(f_alone.path().to_path_buf());

    let e_d_from = notify::Event::new(NotifyEventKind::Modify(ModifyKind::Name(RenameMode::From)))
        .add_path(d_from.path().to_path_buf());

    let e_d_any_from =
        notify::Event::new(NotifyEventKind::Modify(ModifyKind::Name(RenameMode::Any)))
            .add_path(d_any_from.path().to_path_buf());

    let e_d_alone = notify::Event::new(NotifyEventKind::Modify(ModifyKind::Name(RenameMode::To)))
        .add_path(d_alone.path().to_path_buf());

    let (_, command_rx) = crossbeam::channel::unbounded();
    let (event_tx, _) = crossbeam::channel::unbounded();
    let watcher = build_watcher(command_rx, event_tx, Config::try_default().unwrap());
    watcher.handle_command(Command::Watch(dir.path().to_path_buf()));

    let mut f_to_path = f_from.path().to_path_buf();
    let mut f_any_to_path = f_any_from.path().to_path_buf();
    let mut d_to_path = d_from.path().to_path_buf();
    let mut d_any_to_path = d_any_from.path().to_path_buf();
    f_to_path.set_file_name(format!(
        "{}-to",
        f_to_path.file_name().unwrap().to_str().unwrap()
    ));
    f_any_to_path.set_file_name(format!(
        "{}-to",
        f_any_to_path.file_name().unwrap().to_str().unwrap()
    ));
    d_to_path.set_file_name(format!(
        "{}-to",
        d_to_path.file_name().unwrap().to_str().unwrap()
    ));
    d_any_to_path.set_file_name(format!(
        "{}-to",
        d_any_to_path.file_name().unwrap().to_str().unwrap()
    ));
    fs::rename(f_from.path(), &f_to_path).unwrap();
    fs::rename(f_any_from.path(), &f_any_to_path).unwrap();
    fs::rename(d_from.path(), &d_to_path).unwrap();
    fs::rename(d_any_from.path(), &d_any_to_path).unwrap();

    let e_f_to = notify::Event::new(NotifyEventKind::Modify(ModifyKind::Name(RenameMode::To)))
        .add_path(f_to_path.clone());

    let e_f_any_to = notify::Event::new(NotifyEventKind::Modify(ModifyKind::Name(RenameMode::Any)))
        .add_path(f_any_to_path.clone());

    let e_d_to = notify::Event::new(NotifyEventKind::Modify(ModifyKind::Name(RenameMode::To)))
        .add_path(d_to_path.clone());

    let e_d_any_to = notify::Event::new(NotifyEventKind::Modify(ModifyKind::Name(RenameMode::Any)))
        .add_path(d_any_to_path.clone());

    let events = vec![
        e_f_from,
        e_f_to,
        e_f_alone,
        e_d_from,
        e_d_to,
        e_d_alone,
        e_f_any_from,
        e_f_any_to,
        e_d_any_from,
        e_d_any_to,
    ];

    let events = events
        .into_iter()
        .map(|e| notify_debouncer_full::DebouncedEvent::new(e, Instant::now()))
        .collect::<Vec<DebouncedEvent>>();
    let (converted, remaining) = watcher.group_events(events.iter().collect());
    assert_eq!(converted.len(), 4);
    assert_eq!(remaining.len(), 2);
}

fn build_watcher(
    command_rx: Receiver<Command>,
    event_tx: Sender<Result<Vec<Event>, Vec<Error>>>,
    config: Config,
) -> FsWatcher {
    use crate::server::actor::FileSystemActor;
    use notify_debouncer_full::FileIdMap;
    use std::{
        sync::{Arc, Mutex},
        thread,
    };

    let (fs_tx, fs_rx) = crossbeam::channel::unbounded();
    let (fs_command_tx, fs_command_rx) = crossbeam::channel::unbounded();
    let (path_watcher_tx, path_watcher_rx) = crossbeam::channel::unbounded();
    let (path_watcher_command_tx, path_watcher_command_rx) = crossbeam::channel::unbounded();
    let mut file_system_actor = FileSystemActor::new(fs_tx, fs_command_rx);
    let mut path_watcher = path_watcher::Watcher::new(path_watcher_tx, path_watcher_command_rx);
    thread::Builder::new()
        .name("syre file system watcher actor".to_string())
        .spawn(move || file_system_actor.run())
        .unwrap();

    thread::Builder::new()
        .name("syre local file system watcher path watcher".to_string())
        .spawn(move || path_watcher.run())
        .unwrap();

    FsWatcher {
        event_tx,
        command_rx,
        command_tx: fs_command_tx,
        event_rx: fs_rx,
        path_watcher_command_tx,
        path_watcher_rx,
        file_ids: Arc::new(Mutex::new(FileIdMap::new())),
        roots: Mutex::new(vec![]),
        app_config: config,
        shutdown: Mutex::new(false),
    }
}
