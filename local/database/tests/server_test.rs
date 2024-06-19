#![feature(assert_matches)]

use crossbeam::channel::Sender;
use rand::Rng;
use std::{assert_matches::assert_matches, fs, io, thread, time::Duration};
use syre_core::project::Script;
use syre_local::{
    error::IoSerde,
    file_resource::LocalResource,
    project::resources::{Analyses, Project},
    system::collections::ProjectManifest,
    types::AnalysisKind,
};
use syre_local_database::{
    event,
    server::{state, Config},
    types::PortNumber,
    Update,
};

const RECV_TIMEOUT: Duration = Duration::from_millis(500);
const ACTION_SLEEP_TIME: Duration = Duration::from_millis(200);

#[test_log::test]
fn test_server_state_and_updates() {
    let mut rng = rand::thread_rng();
    let dir = tempfile::tempdir().unwrap();
    let user_manifest = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
    let project_manifest = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
    let config = Config::new(
        user_manifest.path(),
        project_manifest.path(),
        rng.gen_range(1024..PortNumber::max_value()),
    );

    let (update_tx, update_rx) = crossbeam::channel::unbounded();
    let update_listener = UpdateListener::new(update_tx, config.update_port());
    thread::spawn(move || update_listener.run());

    let db = syre_local_database::server::Builder::new(config);
    thread::spawn(move || db.run().unwrap());
    let db = syre_local_database::Client::new();

    let user_manifest_state = db.state().user_manifest().unwrap();
    assert_matches!(user_manifest_state, Err(IoSerde::Serde(_)));

    let project_manifest_state = db.state().project_manifest().unwrap();
    assert_matches!(project_manifest_state, Err(IoSerde::Serde(_)));

    // TODO: Handle user manifest
    // fs::write(user_manifest.path(), "{}").unwrap();
    // let update = update_rx.recv_timeout(RECV_TIMEOUT).unwrap();
    // assert_eq!(update.len(), 1);
    // assert_matches!(
    //     update[0].kind(),
    //     event::UpdateKind::App(event::App::UserManifest(event::UserManifest::Repaired))
    // );

    fs::write(project_manifest.path(), "[]").unwrap();
    thread::sleep(ACTION_SLEEP_TIME);

    let project_manifest_state = db.state().project_manifest().unwrap();
    assert_matches!(project_manifest_state, Ok(paths) if paths.is_empty());

    let update = update_rx.recv_timeout(RECV_TIMEOUT).unwrap();
    assert_eq!(update.len(), 1);
    assert_matches!(
        update[0].kind(),
        event::UpdateKind::App(event::App::ProjectManifest(
            event::ProjectManifest::Repaired
        ))
    );

    let project = tempfile::tempdir().unwrap();
    let mut project_manifest = ProjectManifest::load_from(project_manifest.path()).unwrap();
    project_manifest.push(project.path().to_path_buf());
    project_manifest.save().unwrap();
    thread::sleep(ACTION_SLEEP_TIME);

    let project_manifest_state = db.state().project_manifest().unwrap();
    assert_matches!(project_manifest_state, Ok(paths) if *paths == *project_manifest);
    let projects_state = db.state().projects().unwrap();
    assert_eq!(projects_state.len(), 1);
    assert_eq!(projects_state[0].path(), project.path());
    let project_state = &projects_state[0].fs_resource();
    let state::project::FolderResource::Present(project_state) = project_state else {
        panic!();
    };
    assert_matches!(
        project_state.properties(),
        state::project::DataResource::Err(IoSerde::Io(err))
        if *err == io::ErrorKind::NotFound
    );
    assert_matches!(
        project_state.settings(),
        state::project::DataResource::Err(IoSerde::Io(err))
        if *err == io::ErrorKind::NotFound
    );
    assert_matches!(
        project_state.analyses(),
        state::project::DataResource::Err(IoSerde::Io(err))
        if *err == io::ErrorKind::NotFound
    );
    assert_matches!(
        project_state.graph(),
        state::project::FolderResource::Absent
    );

    let update = update_rx.recv_timeout(RECV_TIMEOUT).unwrap();
    assert_eq!(update.len(), 1);
    assert_matches!(
        update[0].kind(),
        event::UpdateKind::App(event::App::ProjectManifest(
                event::ProjectManifest::Added(paths)
        )) if *paths == *project_manifest
    );

    let mut project = Project::new(project.path()).unwrap();
    project.save().unwrap();
    thread::sleep(ACTION_SLEEP_TIME);

    let projects_state = db.state().projects().unwrap();
    assert_eq!(projects_state.len(), 1);
    let project_state = &projects_state[0].fs_resource();
    let state::project::FolderResource::Present(project_state) = project_state else {
        panic!();
    };
    assert!(project_state.properties().is_ok());
    assert!(project_state.settings().is_ok());
    assert_matches!(
        project_state.analyses(),
        state::project::DataResource::Err(IoSerde::Io(err))
        if *err == io::ErrorKind::NotFound
    );
    assert_matches!(
        project_state.graph(),
        state::project::FolderResource::Absent
    );

    let update = update_rx.recv_timeout(RECV_TIMEOUT).unwrap();
    assert_eq!(update.len(), 2);
    assert!(update.iter().any(|update| {
        let event::UpdateKind::Project {
            project: id,
            path: _,
            update,
        } = update.kind()
        else {
            return false;
        };

        let Some(id) = id.as_ref() else {
            return false;
        };

        if *id != project.rid {
            return false;
        }

        matches!(
            update,
            event::Project::Properties(event::DataResource::Created(_))
        )
    }));
    assert!(update.iter().any(|update| {
        let event::UpdateKind::Project {
            project: id,
            path: _,
            update,
        } = update.kind()
        else {
            return false;
        };

        let Some(id) = id.as_ref() else {
            return false;
        };

        if *id != project.rid {
            return false;
        }

        matches!(
            update,
            event::Project::Settings(event::DataResource::Created(_))
        )
    }));

    project.description = Some("test".to_string());
    project.save().unwrap();
    thread::sleep(ACTION_SLEEP_TIME);

    let projects_state = db.state().projects().unwrap();
    assert_eq!(projects_state.len(), 1);
    let project_state = &projects_state[0].fs_resource();
    let state::project::FolderResource::Present(project_state) = project_state else {
        panic!();
    };
    assert_eq!(
        project_state.properties().as_ref().unwrap().description,
        project.description,
    );
    assert!(project_state.settings().is_ok());

    let update = update_rx.recv_timeout(RECV_TIMEOUT).unwrap();
    assert_eq!(update.len(), 1);
    let event::UpdateKind::Project {
        project: project_id,
        path,
        update,
    } = update[0].kind()
    else {
        panic!();
    };

    assert_eq!(*project_id.as_ref().unwrap(), project.rid);
    assert_eq!(path, project.base_path());
    assert_matches!(
        update,
        event::Project::Properties(event::DataResource::Modified(update))
        if update.description == project.description
    );

    fs::write(syre_local::common::project_file_of(project.base_path()), "").unwrap();
    thread::sleep(ACTION_SLEEP_TIME);

    let projects_state = db.state().projects().unwrap();
    assert_eq!(projects_state.len(), 1);
    let project_state = &projects_state[0].fs_resource();
    let state::project::FolderResource::Present(project_state) = project_state else {
        panic!();
    };
    assert_matches!(project_state.properties(), Err(IoSerde::Serde(_)));
    assert!(project_state.settings().is_ok());

    let update = update_rx.recv_timeout(RECV_TIMEOUT).unwrap();
    assert_eq!(update.len(), 1);
    let event::UpdateKind::Project {
        project: project_id,
        path,
        update,
    } = update[0].kind()
    else {
        panic!();
    };

    assert_eq!(*project_id.as_ref().unwrap(), project.rid);
    assert_eq!(path, project.base_path());
    assert_matches!(
        update,
        event::Project::Properties(event::DataResource::Corrupted(err))
        if matches!(err, IoSerde::Serde(_))
    );

    fs::write(
        syre_local::common::project_settings_file_of(project.base_path()),
        "",
    )
    .unwrap();
    thread::sleep(ACTION_SLEEP_TIME);

    let projects_state = db.state().projects().unwrap();
    assert_eq!(projects_state.len(), 1);
    let project_state = &projects_state[0].fs_resource();
    let state::project::FolderResource::Present(project_state) = project_state else {
        panic!();
    };
    assert_matches!(project_state.settings(), Err(IoSerde::Serde(_)));

    let update = update_rx.recv_timeout(RECV_TIMEOUT).unwrap();
    assert_eq!(update.len(), 1);
    let event::UpdateKind::Project {
        project: project_id,
        path,
        update,
    } = update[0].kind()
    else {
        panic!();
    };

    assert_eq!(path, project.base_path());
    assert_matches!(
        update,
        event::Project::Settings(event::DataResource::Corrupted(err))
        if matches!(err, IoSerde::Serde(_))
    );

    project.save().unwrap();
    thread::sleep(ACTION_SLEEP_TIME);

    let projects_state = db.state().projects().unwrap();
    assert_eq!(projects_state.len(), 1);
    let project_state = &projects_state[0].fs_resource();
    let state::project::FolderResource::Present(project_state) = project_state else {
        panic!();
    };
    assert!(project_state.properties().is_ok());
    assert!(project_state.settings().is_ok());
    assert_matches!(
        project_state.analyses(),
        state::project::DataResource::Err(IoSerde::Io(err))
        if *err == io::ErrorKind::NotFound
    );
    assert_matches!(
        project_state.graph(),
        state::project::FolderResource::Absent
    );

    let update = update_rx.recv_timeout(RECV_TIMEOUT).unwrap();
    assert_eq!(update.len(), 2);
    let properties_update = update
        .iter()
        .find(|update| {
            let event::UpdateKind::Project {
                project: id,
                path: _,
                update,
            } = update.kind()
            else {
                return false;
            };

            let Some(id) = id.as_ref() else {
                return false;
            };

            if *id != project.rid {
                return false;
            }

            matches!(
                update,
                event::Project::Properties(event::DataResource::Repaired(_))
            )
        })
        .unwrap();
    let event::UpdateKind::Project {
        update: properties_update,
        ..
    } = properties_update.kind()
    else {
        panic!()
    };
    let event::Project::Properties(event::DataResource::Repaired(properties)) = properties_update
    else {
        panic!();
    };
    assert_eq!(properties.description, project.description);
    assert!(update.iter().any(|update| {
        let event::UpdateKind::Project {
            project: id,
            path: _,
            update,
        } = update.kind()
        else {
            return false;
        };

        let Some(id) = id.as_ref() else {
            return false;
        };

        if *id != project.rid {
            return false;
        }

        matches!(
            update,
            event::Project::Settings(event::DataResource::Repaired(_))
        )
    }));

    let mut analyses = Analyses::new(project.base_path());
    analyses.save().unwrap();
    thread::sleep(ACTION_SLEEP_TIME);

    let projects_state = db.state().projects().unwrap();
    assert_eq!(projects_state.len(), 1);
    let project_state = &projects_state[0].fs_resource();
    let state::project::FolderResource::Present(project_state) = project_state else {
        panic!();
    };
    assert!(project_state.properties().is_ok());
    assert!(project_state.settings().is_ok());
    assert!(project_state.analyses().is_ok());
    assert_matches!(
        project_state.graph(),
        state::project::FolderResource::Absent
    );

    let update = update_rx.recv_timeout(RECV_TIMEOUT).unwrap();
    assert_eq!(update.len(), 1);
    let event::UpdateKind::Project {
        project: project_id,
        path,
        update,
    } = update[0].kind()
    else {
        panic!();
    };

    assert_eq!(*project_id.as_ref().unwrap(), project.rid);
    assert_eq!(path, project.base_path());
    assert_matches!(
        update,
        event::Project::Analyses(event::DataResource::Created(_))
    );

    fs::write(analyses.path(), "").unwrap();
    thread::sleep(ACTION_SLEEP_TIME);

    let projects_state = db.state().projects().unwrap();
    assert_eq!(projects_state.len(), 1);
    let project_state = &projects_state[0].fs_resource();
    let state::project::FolderResource::Present(project_state) = project_state else {
        panic!();
    };
    assert_matches!(project_state.analyses(), Err(IoSerde::Serde(_)));

    let update = update_rx.recv_timeout(RECV_TIMEOUT).unwrap();
    assert_eq!(update.len(), 1);
    let event::UpdateKind::Project {
        project: _,
        path,
        update,
    } = update[0].kind()
    else {
        panic!();
    };

    assert_eq!(path, project.base_path());
    assert_matches!(
        update,
        event::Project::Analyses(event::DataResource::Corrupted(err))
        if matches!(err, IoSerde::Serde(_))
    );

    let script = Script::from_path("test.py").unwrap();
    analyses.insert(script.rid.clone(), AnalysisKind::Script(script.clone()));
    analyses.save().unwrap();
    thread::sleep(ACTION_SLEEP_TIME);

    let projects_state = db.state().projects().unwrap();
    assert_eq!(projects_state.len(), 1);
    let project_state = &projects_state[0].fs_resource();
    let state::project::FolderResource::Present(project_state) = project_state else {
        panic!();
    };
    assert!(project_state.properties().is_ok());
    assert!(project_state.settings().is_ok());
    assert!(project_state.analyses().is_ok());
    assert_matches!(
        project_state.graph(),
        state::project::FolderResource::Absent
    );

    let update = update_rx.recv_timeout(RECV_TIMEOUT).unwrap();
    assert_eq!(update.len(), 1);
    let event::UpdateKind::Project {
        project: project_id,
        path,
        update,
    } = update[0].kind()
    else {
        panic!();
    };

    assert_eq!(*project_id.as_ref().unwrap(), project.rid);
    assert_eq!(path, project.base_path());
    let event::Project::Analyses(event::DataResource::Repaired(analyses_state)) = update else {
        panic!();
    };
    assert_eq!(analyses_state.len(), 1);
    assert_eq!(*analyses_state[0], AnalysisKind::Script(script));

    project.set_analysis_root("analysis");
}

struct UpdateListener {
    tx: Sender<Vec<Update>>,
    socket: zmq::Socket,
}

impl UpdateListener {
    pub fn new(tx: Sender<Vec<Update>>, port: PortNumber) -> Self {
        let zmq_context = zmq::Context::new();
        let socket = zmq_context.socket(zmq::SUB).unwrap();
        socket
            .set_subscribe(syre_local_database::constants::PUB_SUB_TOPIC.as_bytes())
            .unwrap();

        socket
            .connect(&syre_local_database::common::localhost_with_port(port))
            .unwrap();

        Self { tx, socket }
    }

    pub fn run(&self) {
        loop {
            let messages = self.socket.recv_multipart(0).unwrap();
            let messages = messages
                .into_iter()
                .map(|msg| zmq::Message::try_from(msg).unwrap())
                .collect::<Vec<_>>();

            let mut message = String::new();
            // skip one for topic
            for msg in messages.iter().skip(1) {
                let msg = msg.as_str().unwrap();
                message.push_str(msg);
            }

            let events: Vec<Update> = serde_json::from_str(&message).unwrap();
            self.tx.send(events).unwrap();
        }
    }
}

mod common {
    use std::fs;
    use std::path::PathBuf;
    use syre_local::project::project;
    use syre_local::project::resources::{Container as LocalContainer, Project as LocalProject};

    pub fn init_project() -> PathBuf {
        let project_dir = tempfile::tempdir().unwrap();
        project::init(project_dir.path()).unwrap();
        project_dir.into_path()
    }

    pub fn init_project_graph(prj: LocalProject) {
        fs::create_dir(prj.data_root_path()).unwrap();
        let root = LocalContainer::new(prj.data_root_path());
        root.save().unwrap();
    }
}
