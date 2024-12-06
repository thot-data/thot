use crossbeam::channel::{select, Receiver, Sender};
use syre_fs_watcher::{Event, EventResult};

pub struct EventValidator {
    expected: Vec<Event>,
    received: Vec<Event>,
    expected_rx: Receiver<Vec<Event>>,
    watcher_rx: Receiver<EventResult>,
    validation_tx: Sender<error::Validation>,
}

impl EventValidator {
    pub fn new(
        watcher_rx: Receiver<EventResult>,
        expected_rx: Receiver<Vec<Event>>,
        validation_tx: Sender<error::Validation>,
    ) -> Self {
        Self {
            expected: vec![],
            received: vec![],
            expected_rx,
            watcher_rx,
            validation_tx,
        }
    }

    pub fn run(&mut self) -> Result<(), ()> {
        loop {
            select! {
                recv(self.watcher_rx) -> events => match events {
                    Ok(events) => self.handle_watcher_events(events)?,
                    Err(err) => {
                        tracing::error!("watcher: {err:}");
                        return Err(());
                    }
                },

                recv(self.expected_rx) -> events => match events {
                    Ok(events) => self.handle_expected_events(events),
                    Err(err) => {
                        tracing::error!("simulator: {err:}");
                        return Err(());
                    }
                },

                default => self.validate_events()
            }
        }
    }
}

impl EventValidator {
    fn handle_watcher_events(&mut self, events: EventResult) -> Result<(), ()> {
        match events {
            Ok(mut events) => {
                self.received.append(&mut events);
                Ok(())
            }

            Err(errors) => {
                tracing::error!(?errors);
                Err(())
            }
        }
    }

    fn handle_expected_events(&mut self, mut events: Vec<Event>) {
        self.expected.append(&mut events);
    }
}

impl EventValidator {
    fn validate_events(&self) {}
}

pub mod error {
    use syre_fs_watcher::Event;

    #[derive(Debug)]
    pub struct Validation {
        pub expected: Event,
        pub received: Event,
    }
}
