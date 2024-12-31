use crate::{
    common,
    event::{self as update, Update},
    server, state, Database,
};
use std::assert_matches::assert_matches;
use syre_fs_watcher::{event, EventKind};
use syre_local::{self as local, TryReducible};

impl Database {
    pub(super) fn handle_fs_event_folder(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        let EventKind::Folder(kind) = event.kind() else {
            panic!("invalid event kind");
        };

        match kind {
            event::ResourceEvent::Created => self.handle_fs_event_folder_created(event),
            event::ResourceEvent::Modified(_) => self.handle_fs_event_folder_modified(event),
            event::ResourceEvent::Moved => todo!(),
            _ => todo!(),
        }
    }
}

impl Database {
    fn handle_fs_event_folder_created(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Folder(event::ResourceEvent::Created)
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        if self.config.handle_fs_resource_changes() {
            // TODO: May want to return errors if project state is not valid.
            let project = self.state.find_resource_project_by_path(path).unwrap();
            let state::FolderResource::Present(project_data) = project.fs_resource().as_ref()
            else {
                return vec![];
            };

            let state::DataResource::Ok(project_properties) = project_data.properties() else {
                return vec![];
            };

            let data_root = project.path().join(&project_properties.data_root);
            if path.starts_with(&data_root) {
                self.handle_folder_created_container(event)
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }

    fn handle_folder_created_container(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::Folder(event::ResourceEvent::Created)
        );

        let [path] = &event.paths()[..] else {
            unreachable!("invalid paths");
        };

        let path_app_dirs = super::path_app_dir_count(path);
        if path_app_dirs > 0 {
            if path_app_dirs == 1 {
                assert!(!path.ends_with(local::constants::APP_DIR));
            }
            return vec![];
        }

        // TODO: May want to return errors if project state is not valid.
        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_data) = project.fs_resource().as_ref() else {
            unreachable!("invalid state");
        };

        let state::DataResource::Ok(project_properties) = project_data.properties() else {
            unreachable!("invalid state");
        };

        let mut builder = local::project::container::builder::InitOptions::init();
        builder.recurse(true);
        builder.with_new_ids(true);
        builder.with_assets();
        if let Err(err) = builder.build(&path) {
            tracing::error!(?err);
            todo!();
        }

        let local::loader::container::State {
            properties,
            settings,
            assets,
        } = local::loader::container::Loader::load_resources(path);

        let state::FolderResource::Present(project_data) = project.fs_resource().as_ref() else {
            unreachable!("inalid state");
        };

        let state::DataResource::Ok(project_properties) = project_data.properties() else {
            unreachable!("invalid state");
        };

        let data_root_path = project.path().join(&project_properties.data_root);
        let parent_path =
            common::container_graph_path(&data_root_path, path.parent().unwrap()).unwrap();
        let subgraph = server::state::project::graph::State::load(path).unwrap();
        let subgraph_state = subgraph.as_graph();

        let project_path = project.path().clone();
        let project_id = project_properties.rid().clone();
        self.state
            .try_reduce(server::state::Action::Project {
                path: project_path.clone(),
                action: server::state::project::action::Graph::Insert {
                    parent: parent_path.clone(),
                    graph: subgraph,
                }
                .into(),
            })
            .unwrap();

        vec![Update::project_with_id(
            project_id,
            project_path,
            update::Graph::Inserted {
                parent: parent_path,
                graph: subgraph_state,
            }
            .into(),
            event.id().clone(),
        )]
    }

    fn handle_fs_event_folder_modified(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        let EventKind::Folder(event::ResourceEvent::Modified(kind)) = event.kind() else {
            panic!("invalid event kind");
        };

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        match kind {
            event::ModifiedKind::Data => todo!(),
            event::ModifiedKind::Other => vec![],
        }
    }
}
