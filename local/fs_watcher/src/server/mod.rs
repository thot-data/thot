//! File system event handler.
mod actor;
pub mod event;
mod path_watcher;
pub(crate) mod watcher;

pub use event::{Event, EventKind};
pub use watcher::{config, Builder, Config, FsWatcher};

pub enum ConversionResult<'a> {
    Ok(event::Event<'a>),
    Err(ConversionError<'a>),
}

#[derive(Debug)]
pub struct ConversionError<'a> {
    events: Vec<&'a notify_debouncer_full::DebouncedEvent>,
    kind: crate::error::Process,
}

impl<'a> Into<crate::Error> for ConversionError<'a> {
    fn into(self) -> crate::Error {
        crate::Error::Processing {
            events: self.events.into_iter().cloned().collect(),
            kind: self.kind,
        }
    }
}
