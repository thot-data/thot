use crate::{event as update, server, state, Database, Update};
use std::assert_matches::assert_matches;
use syre_core as core;
use syre_fs_watcher::{event, EventKind};
use syre_local::{self as local, TryReducible};

impl Database {
    pub(super) fn handle_fs_event_analysis_file(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::AnalysisFile(kind) = event.kind() else {
            panic!("invalid event kind");
        };

        match kind {
            syre_fs_watcher::event::ResourceEvent::Created => {
                self.handle_fs_event_analysis_file_created(event)
            }
            syre_fs_watcher::event::ResourceEvent::Removed => {
                self.handle_fs_event_analysis_file_removed(event)
            }
            syre_fs_watcher::event::ResourceEvent::Renamed => todo!(),
            syre_fs_watcher::event::ResourceEvent::Moved => todo!(),
            syre_fs_watcher::event::ResourceEvent::MovedProject => todo!(),
            syre_fs_watcher::event::ResourceEvent::Modified(_) => {
                self.handle_fs_event_analysis_file_modified(event)
            }
        }
    }
}

impl Database {
    fn handle_fs_event_analysis_file_created(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::AnalysisFile(event::ResourceEvent::Created)
        );

        self.handle_analysis_file_created(event)
    }

    fn handle_fs_event_analysis_file_removed(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::AnalysisFile(event::ResourceEvent::Removed)
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(analyses) = project_state.analyses() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let mut analyses_state = analyses.clone();
        let rel_path = path
            .strip_prefix(
                project
                    .path()
                    .join(properties.analysis_root.as_ref().unwrap()),
            )
            .unwrap();

        if let Some(analysis) = server::state::project::analysis::find_analysis_by_path_mut(
            rel_path,
            &mut analyses_state,
        ) {
            assert!(analysis.is_present());
            analysis.set_absent();

            let project_path = project.path().clone();
            let project_id = properties.rid().clone();
            let analysis_root = properties.analysis_root.clone();
            self.state
                .try_reduce(server::state::Action::Project {
                    path: project_path.clone(),
                    action: server::state::project::Action::SetAnalyses(state::DataResource::Ok(
                        analyses_state,
                    )),
                })
                .unwrap();

            if self.config.handle_fs_resource_changes() {
                let analysis_root = project_path.join(analysis_root.unwrap());
                let mut analyses =
                    local::project::resources::Analyses::load_from(project_path).unwrap();

                let analysis_path = path.strip_prefix(&analysis_root).unwrap();
                analyses.retain(|_, analysis| match analysis {
                    local::types::AnalysisKind::Script(script) => script.path != analysis_path,
                    local::types::AnalysisKind::ExcelTemplate(template) => {
                        template.template.path != analysis_path
                    }
                });
                analyses.save().unwrap();

                vec![]
            } else {
                vec![Update::project_with_id(
                    project_id,
                    project_path,
                    update::AnalysisFile::Removed(path.clone()).into(),
                    event.id().clone(),
                )]
            }
        } else {
            vec![]
        }
    }

    fn handle_fs_event_analysis_file_modified(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        let EventKind::AnalysisFile(event::ResourceEvent::Modified(kind)) = event.kind() else {
            panic!("invalid kind");
        };

        match kind {
            event::ModifiedKind::Data => todo!(),
            event::ModifiedKind::Other => self.handle_fs_event_analysis_file_modified_other(event),
        }
    }

    fn handle_fs_event_analysis_file_modified_other(
        &mut self,
        event: syre_fs_watcher::Event,
    ) -> Vec<Update> {
        assert_matches!(
            event.kind(),
            EventKind::AnalysisFile(event::ResourceEvent::Modified(event::ModifiedKind::Other))
        );

        let [path] = &event.paths()[..] else {
            panic!("invalid paths");
        };

        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(analyses) = project_state.analyses() else {
            return vec![];
        };

        let state::DataResource::Ok(properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let mut analyses_state = analyses.clone();
        let rel_path = path
            .strip_prefix(
                project
                    .path()
                    .join(properties.analysis_root.as_ref().unwrap()),
            )
            .unwrap();

        if let Some(_analysis) = server::state::project::analysis::find_analysis_by_path_mut(
            rel_path,
            &mut analyses_state,
        ) {
            #[cfg(target_os = "windows")]
            {
                vec![]
            }

            #[cfg(not(target_os = "windows"))]
            todo!();
        } else {
            #[cfg(target_os = "windows")]
            {
                vec![]
                // self.handle_analysis_file_created(event)
            }

            #[cfg(not(target_os = "windows"))]
            todo!();
        }
    }

    fn handle_analysis_file_created(&mut self, event: syre_fs_watcher::Event) -> Vec<Update> {
        let path = match &event.paths()[..] {
            [path] => path,
            [_from, to] => to,
            _ => panic!("invalid paths"),
        };

        let project = self.state.find_resource_project_by_path(path).unwrap();
        let state::FolderResource::Present(project_state) = project.fs_resource() else {
            panic!("invalid state");
        };

        let state::DataResource::Ok(analyses) = project_state.analyses() else {
            return vec![];
        };

        let state::DataResource::Ok(properties) = project_state.properties() else {
            panic!("invalid state");
        };

        let mut analyses_state = analyses.clone();
        let rel_path = path
            .strip_prefix(
                project
                    .path()
                    .join(properties.analysis_root.as_ref().unwrap()),
            )
            .unwrap();

        if let Some(analysis) = server::state::project::analysis::find_analysis_by_path_mut(
            rel_path,
            &mut analyses_state,
        ) {
            assert!(!analysis.is_present());
            analysis.set_present();
            self.state
                .try_reduce(server::state::Action::Project {
                    path: project.path().clone(),
                    action: server::state::project::Action::SetAnalyses(state::DataResource::Ok(
                        analyses_state,
                    )),
                })
                .unwrap();

            vec![]
        } else {
            if self.config.handle_fs_resource_changes() {
                let analysis_root = project
                    .path()
                    .join(properties.analysis_root.clone().unwrap());

                let mut analyses =
                    local::project::resources::Analyses::load_from(project.path()).unwrap();

                let analysis_path = path.strip_prefix(&analysis_root).unwrap();
                let Some(ext) = analysis_path.extension() else {
                    return vec![];
                };
                let ext = ext.to_str().unwrap();

                if core::project::ScriptLang::supported_extensions().contains(&ext) {
                    let analysis = core::project::Script::from_path(analysis_path).unwrap();
                    analyses.insert(analysis.rid().clone(), analysis.into());
                    analyses.save().unwrap();

                    vec![]
                } else if core::project::ExcelTemplate::supported_extensions().contains(&ext) {
                    todo!();
                } else {
                    vec![]
                }
            } else {
                vec![Update::project_with_id(
                    properties.rid().clone(),
                    project.path().clone(),
                    update::AnalysisFile::Created(path.clone()).into(),
                    event.id().clone(),
                )]
            }
        }
    }
}
