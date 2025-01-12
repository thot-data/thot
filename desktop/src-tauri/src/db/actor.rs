//! Actor for listening to database updates.
use super::FS_EVENT_TOPIC;
use crate::state;
use std::collections::HashMap;
use syre_desktop_lib as lib;
use syre_local as local;
use syre_local_database as db;
use tauri::{Emitter, EventTarget, Manager};
use uuid::Uuid;

/// Builder for [`Actor`].
pub struct Builder {
    app: tauri::AppHandle,
}

impl Builder {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }

    /// Create a new actor that listens to database updates.
    /// The actor immediately begins listening.
    pub fn run(self) {
        let zmq_context = zmq::Context::new();
        let zmq_socket = zmq_context.socket(zmq::SUB).unwrap();
        zmq_socket
            .set_subscribe(db::constants::PUB_SUB_TOPIC.as_bytes())
            .unwrap();

        zmq_socket
            .connect(&db::common::zmq_url(zmq::SUB).unwrap())
            .unwrap();

        let actor = Actor {
            app: self.app,
            zmq_socket,
            db: db::Client::new(),
        };
        actor.run()
    }
}

/// Actor that listens to and handles updates published from
/// a syre local database.
pub struct Actor {
    /// Tauri app handle.
    app: tauri::AppHandle,

    /// Socket to listen for updates on.
    zmq_socket: zmq::Socket,

    /// Local database client.
    db: db::Client,
}

impl Actor {
    /// Listen for database updates and send them to main window.
    fn run(&self) {
        'main: loop {
            let messages = match self.zmq_socket.recv_multipart(0) {
                Ok(msg) => msg,
                Err(err) => {
                    tracing::error!(?err);
                    continue;
                }
            };

            let messages = messages
                .into_iter()
                .map(|msg| zmq::Message::try_from(msg).unwrap())
                .collect::<Vec<_>>();

            let Some(topic) = messages.get(0) else {
                tracing::error!("could not get topic from message {messages:?}");
                continue;
            };

            let Some(topic) = topic.as_str() else {
                tracing::error!("could not convert topic to str");
                continue;
            };

            let mut message = String::new();
            for msg in messages.iter().skip(1) {
                let Some(msg) = msg.as_str() else {
                    tracing::error!("could not convert message to str");
                    continue 'main;
                };

                message.push_str(msg);
            }

            let updates: Vec<db::event::Update> = match serde_json::from_str(&message) {
                Ok(events) => events,
                Err(err) => {
                    tracing::error!(?message);
                    tracing::error!(?err);
                    continue;
                }
            };

            self.handle_updates(topic, updates);
        }
    }
}

impl Actor {
    fn handle_updates(&self, topic: &str, updates: Vec<db::event::Update>) {
        tracing::debug!(?updates);
        let topic = topic.replace("local-database", "database/update");

        let events = updates
            .into_iter()
            .flat_map(|event| self.process_event(&topic, event))
            .collect::<Vec<_>>();

        tracing::debug!(?events);
        let mut grouped = HashMap::with_capacity(events.len());
        for (topic, update) in events {
            let entry = grouped.entry(topic).or_insert(vec![]);
            entry.push(update);
        }

        for (topic, events) in grouped {
            self.emit_events_default(topic, events);
        }
    }

    fn process_event(
        &self,
        topic: impl AsRef<str>,
        event: db::event::Update,
    ) -> Vec<(String, lib::Event)> {
        match event.kind() {
            db::event::UpdateKind::App(_) => self.process_event_app(topic, event),
            db::event::UpdateKind::Project { .. } => self.process_event_project(topic, event),
        }
    }

    /// Emits events to windows listening to the [`crate::db::FS_EVENT_TOPIC`].
    ///
    /// # Arguments
    /// + `topic`: Event name.
    fn emit_events_default(&self, topic: impl AsRef<str>, events: Vec<lib::Event>) {
        if let Err(err) = self.app.emit_to(
            EventTarget::webview_window(FS_EVENT_TOPIC),
            topic.as_ref(),
            events,
        ) {
            tracing::error!(?err);
        }
    }
}

impl Actor {
    fn process_event_app(
        &self,
        topic: impl AsRef<str>,
        event: db::event::Update,
    ) -> Vec<(String, lib::Event)> {
        let db::event::UpdateKind::App(update) = event.kind() else {
            panic!("invalid event kind");
        };

        match update {
            db::event::App::UserManifest(_) => self.process_event_app_user_manifest(topic, event),
            db::event::App::ProjectManifest(_) => {
                self.process_event_app_project_manifest(topic, event)
            }
            db::event::App::LocalConfig(_) => self.process_event_app_local_config(topic, event),
        }
    }

    fn process_event_app_local_config(
        &self,
        topic: impl AsRef<str>,
        event: db::event::Update,
    ) -> Vec<(String, lib::Event)> {
        let db::event::UpdateKind::App(db::event::App::LocalConfig(update)) = event.kind() else {
            panic!("invalid event kind");
        };

        match update {
            db::event::LocalConfig::Ok(config) => {
                return self.handle_local_config_update(event.id().clone(), config);
            }
            db::event::LocalConfig::Error => {
                let state = self.app.state::<crate::State>();
                let state_user = state.user();
                let mut state_user = state_user.lock().unwrap();
                if state_user.is_some() {
                    let _ = state_user.take();
                    return vec![(
                        lib::event::topic::USER.to_string(),
                        lib::Event::new(lib::EventKind::User(None), event.id().clone()),
                    )];
                }
            }
            db::event::LocalConfig::Updated => {
                let db::state::ConfigState::Ok(config) = self.db.state().local_config().unwrap()
                else {
                    panic!("invalid state");
                };

                return self.handle_local_config_update(event.id().clone(), &config);
            }
        }

        vec![]
    }

    fn handle_local_config_update(
        &self,
        event: Uuid,
        config: &local::system::resources::local::Config,
    ) -> Vec<(String, lib::Event)> {
        let state = self.app.state::<crate::State>();
        let state_user = state.user();
        let mut state_user = state_user.lock().unwrap();
        match (state_user.as_ref(), config.user.as_ref()) {
            (None, None) => {
                vec![]
            }
            (Some(_), None) => {
                let _ = state_user.take();
                vec![(
                    lib::event::topic::USER.to_string(),
                    lib::Event::new(lib::EventKind::User(None), event),
                )]
            }
            (None, Some(user)) => {
                let projects = state::load_user_state(&self.db, &user);
                let _ = state_user.insert(state::User::new(user.clone(), projects));

                let user = self.db.user().get(user.clone()).unwrap();
                assert!(user.is_some());
                vec![(
                    lib::event::topic::USER.to_string(),
                    lib::Event::new(lib::EventKind::User(user), event),
                )]
            }
            (Some(user_state), Some(user_update)) => {
                if user_state.rid() == user_update {
                    return vec![];
                }

                let projects = state::load_user_state(&self.db, &user_update);
                let _ = state_user.insert(state::User::new(user_update.clone(), projects));

                let user = self.db.user().get(user_update.clone()).unwrap();
                assert!(user.is_some());
                vec![(
                    lib::event::topic::USER.to_string(),
                    lib::Event::new(lib::EventKind::User(user), event),
                )]
            }
        }
    }

    fn process_event_app_user_manifest(
        &self,
        topic: impl AsRef<str>,
        event: db::event::Update,
    ) -> Vec<(String, lib::Event)> {
        let db::event::UpdateKind::App(db::event::App::UserManifest(update)) = event.kind() else {
            panic!("invalid event kind");
        };

        let state = self.app.state::<crate::State>();
        let user = state.user();
        let mut user = user.lock().unwrap();
        let Some(ref active_user) = *user else {
            return vec![];
        };

        match update {
            db::event::UserManifest::Ok(manifest)
            | db::event::UserManifest::Added(manifest)
            | db::event::UserManifest::Updated(manifest) => {
                if let Some(user) = manifest.iter().find(|user| user.rid() == active_user.rid()) {
                    vec![(
                        lib::event::topic::USER.to_string(),
                        lib::Event::new(
                            lib::EventKind::User(Some(user.clone())),
                            event.id().clone(),
                        ),
                    )]
                } else {
                    vec![]
                }
            }
            db::event::UserManifest::Error => todo!(),
            db::event::UserManifest::Removed(manifest) => {
                *user = None;
                vec![(
                    lib::event::topic::USER.to_string(),
                    lib::Event::new(lib::EventKind::User(None), event.id().clone()),
                )]
            }
        }
    }

    fn process_event_app_project_manifest(
        &self,
        topic: impl AsRef<str>,
        event: db::event::Update,
    ) -> Vec<(String, lib::Event)> {
        let db::event::UpdateKind::App(db::event::App::ProjectManifest(update)) = event.kind()
        else {
            panic!("invalid event kind");
        };

        match update {
            db::event::ProjectManifest::Added(_) => {
                self.process_event_app_project_manifest_added(topic, event)
            }
            db::event::ProjectManifest::Removed(_) => {
                self.process_event_app_project_manifest_removed(topic, event)
            }
            db::event::ProjectManifest::Repaired => {
                todo!()
            }
            db::event::ProjectManifest::Corrupted => {
                vec![(
                    lib::event::topic::PROJECT_MANIFEST.to_string(),
                    lib::Event::new(
                        lib::event::ProjectManifest::Corrupted.into(),
                        event.id().clone(),
                    ),
                )]
            }
        }
    }

    fn process_event_app_project_manifest_added(
        &self,
        topic: impl AsRef<str>,
        event: db::event::Update,
    ) -> Vec<(String, lib::Event)> {
        let db::event::UpdateKind::App(db::event::App::ProjectManifest(
            db::event::ProjectManifest::Added(paths),
        )) = event.kind()
        else {
            panic!("invalid event kind");
        };

        let state = self.app.state::<crate::State>();
        let user = state.user();
        let user = user.lock().unwrap();
        let Some(ref user) = *user else {
            return vec![];
        };

        let projects = self
            .db
            .project()
            .get_many(paths.clone())
            .unwrap()
            .iter()
            .filter_map(|project| {
                let db::state::FolderResource::Present(state) = project.fs_resource() else {
                    return None;
                };

                let db::state::DataResource::Ok(settings) = state.settings() else {
                    return None;
                };

                let Some(permissions) = settings.permissions.get(user.rid()) else {
                    return None;
                };

                if permissions.any() {
                    Some((project.path().clone(), state.clone()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if projects.is_empty() {
            return vec![];
        }

        let user_projects = projects
            .into_iter()
            .filter(|(_path, data)| {
                let db::state::DataResource::Ok(settings) = data.settings() else {
                    return false;
                };

                let Some(permissions) = settings.permissions.get(user.rid()) else {
                    return false;
                };

                permissions.any()
            })
            .collect::<Vec<_>>();

        user.projects()
            .lock()
            .unwrap()
            .extend(user_projects.iter().map(|(path, _)| path.clone()));

        vec![(
            lib::event::topic::PROJECT_MANIFEST.to_string(),
            lib::Event::new(
                lib::event::ProjectManifest::Added(user_projects).into(),
                event.id().clone(),
            ),
        )]
    }

    fn process_event_app_project_manifest_removed(
        &self,
        topic: impl AsRef<str>,
        event: db::event::Update,
    ) -> Vec<(String, lib::Event)> {
        let db::event::UpdateKind::App(db::event::App::ProjectManifest(
            db::event::ProjectManifest::Removed(paths),
        )) = event.kind()
        else {
            panic!("invalid event kind");
        };

        let state = self.app.state::<crate::State>();
        let user_state = state.user();
        let mut user_state = user_state.lock().unwrap();
        let Some(user_state) = user_state.as_mut() else {
            return vec![];
        };

        let user_projects = user_state.projects();
        let mut user_projects = user_projects.lock().unwrap();
        let mut removed = Vec::with_capacity(paths.len());
        for path in paths {
            if let Some(index) = user_projects.iter().position(|project| project == path) {
                removed.push(user_projects.swap_remove(index));
            }
        }

        if removed.is_empty() {
            vec![]
        } else {
            vec![(
                lib::event::topic::PROJECT_MANIFEST.to_string(),
                lib::Event::new(
                    lib::event::ProjectManifest::Removed(removed).into(),
                    event.id().clone(),
                ),
            )]
        }
    }
}

impl Actor {
    fn process_event_project(
        &self,
        topic: impl AsRef<str>,
        event: db::event::Update,
    ) -> Vec<(String, lib::Event)> {
        let db::event::UpdateKind::Project {
            project, update, ..
        } = event.kind()
        else {
            panic!("invalid event kind");
        };

        let state = self.app.state::<crate::State>();
        let actions = state.actions();
        let mut actions = actions.lock().unwrap();
        let event_actions = actions
            .extract_if(.., |action| action.matches(&event))
            .collect::<Vec<_>>();
        drop(actions);
        for action in event_actions {
            action.call(&event);
        }

        match update {
            db::event::Project::FolderRemoved
            | db::event::Project::Moved(_)
            | db::event::Project::Settings(_) => todo!(),

            db::event::Project::Properties(_) => vec![(
                lib::event::topic::project(project.as_ref().unwrap()),
                lib::Event::new(update.clone().into(), event.id().clone()),
            )],

            db::event::Project::Analyses(_)
            | db::event::Project::Graph(_)
            | db::event::Project::Container { .. }
            | db::event::Project::Asset { .. }
            | db::event::Project::AssetFile(_)
            | db::event::Project::AnalysisFile(_) => {
                vec![(
                    lib::event::topic::graph(project.as_ref().unwrap()),
                    lib::Event::new(update.clone().into(), event.id().clone()),
                )]
            }
        }
    }
}
