use super::{super::ConversionError, FsWatcher};
use crate::{error, server};
use notify_debouncer_full::DebouncedEvent;
use rayon::{iter::Either, prelude::*};
use std::path::PathBuf;

impl FsWatcher {
    pub fn windows_postprocess_fs_conversion<'a>(
        &self,
        mut events: Vec<server::Event<'a>>,
        errors: Vec<ConversionError<'a>>,
    ) -> (Vec<server::Event<'a>>, Vec<ConversionError<'a>>) {
        let (processed_events, errors) = self.windows_handle_not_found_errors(errors);

        events.extend(processed_events);
        (events, errors)
    }

    /// On Windows, if a root path is moved to the recycle bin
    /// all its children emit `Modify(Any)` events (see https://github.com/notify-rs/notify/issues/554).
    /// These events can not be processed because the files are no longer present, resulting in `NotFound` errors.
    /// This checks for these errors, and if present, checks if the resource is in the recycle bin.
    /// If so, it replaces all the relevant events with a `Remove` event and removes any nested events.
    ///
    /// # Returns
    /// Events derived from the errors, and any remaining errors.
    fn windows_handle_not_found_errors<'a>(
        &self,
        errors: Vec<ConversionError<'a>>,
    ) -> (Vec<server::Event<'a>>, Vec<ConversionError<'a>>) {
        let roots = self.roots.lock().unwrap();
        let mut root_paths = errors
            .iter()
            .filter_map(|err| {
                if !matches!(err.kind, error::Process::NotFound) {
                    tracing::debug!("not notfound");
                    return None;
                }

                let [event] = err.events[..] else {
                    panic!("invalid events");
                };

                let [path] = &event.paths[..] else {
                    panic!("invalid paths");
                };

                roots.iter().find(|root| path.strip_prefix(root).is_ok())
            })
            .collect::<Vec<_>>();

        root_paths.sort();
        root_paths.dedup();
        let trash_root_paths = root_paths
            .into_iter()
            .filter(|root_path| {
                let (tx, rx) = crossbeam::channel::bounded(1);
                self.final_path(root_path, tx);
                match rx.recv().unwrap() {
                    Ok(Some(final_path)) => in_trash::in_trash(&final_path),
                    Ok(None) => false,
                    Err(err) => {
                        panic!("{err:?}");
                    }
                }
            })
            .collect::<Vec<_>>();

        let (trash_events, errors): (Vec<_>, Vec<_>) =
            errors.into_par_iter().partition_map(|err| {
                if !matches!(err.kind, error::Process::NotFound) {
                    return Either::Right(err);
                }

                let [event] = err.events[..] else {
                    panic!("invalid events");
                };

                let [path] = &event.paths[..] else {
                    panic!("invalid paths");
                };

                if trash_root_paths
                    .iter()
                    .any(|root_path| path.strip_prefix(root_path).is_ok())
                {
                    Either::Left(event)
                } else {
                    Either::Right(err)
                }
            });

        let mut trash_event_parents: Vec<(&PathBuf, Vec<&DebouncedEvent>)> =
            Vec::with_capacity(trash_events.len());
        for event in trash_events {
            let [path] = &event.paths[..] else {
                panic!("invalid paths");
            };

            let root_path = trash_root_paths
                .iter()
                .find(|root_path| path.strip_prefix(root_path).is_ok())
                .unwrap();

            if let Some((_, parents)) = trash_event_parents
                .iter_mut()
                .find(|(key_path, _)| key_path == root_path)
            {
                parents.push(event);
            } else {
                trash_event_parents.push((root_path, vec![event]));
            }
        }

        let events = trash_root_paths
            .into_iter()
            .map(|path| {
                let parents = trash_event_parents
                    .iter()
                    .position(|(root_path, _)| *root_path == path)
                    .unwrap();
                let (_, parents) = trash_event_parents.swap_remove(parents);
                let time = parents
                    .iter()
                    .min_by_key(|event| event.time)
                    .unwrap()
                    .time
                    .clone();

                let mut event = server::Event::new(
                    server::EventKind::Folder(server::event::Folder::Removed(path.clone())),
                    time,
                );

                for parent in parents.into_iter() {
                    event = event.add_parent(parent);
                }

                tracing::trace!("project root {path:?} moved to trash");
                event
            })
            .collect();

        (events, errors)
    }
}
