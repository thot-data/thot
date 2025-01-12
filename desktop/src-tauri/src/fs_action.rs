use syre_local_database as db;

pub type Filter = Box<dyn Fn(&db::event::Update) -> bool + Send>;
pub type Callback = Box<dyn FnOnce(&db::event::Update) + Send>;

pub struct Action {
    filter: Filter,
    callback: Callback,
}

impl Action {
    pub fn new(filter: Filter, callback: Callback) -> Self {
        Self { filter, callback }
    }

    pub fn matches(&self, event: &db::event::Update) -> bool {
        (self.filter)(event)
    }

    pub fn call(self, event: &db::event::Update) {
        (self.callback)(event);
    }
}
