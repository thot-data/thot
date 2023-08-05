use crate::system::settings::UserSettings;
use crate::Result;
use thot_core::project::StandardProperties as CoreStandardProperties;
use thot_core::types::{Creator, UserId};

pub struct StandardProperties;

impl StandardProperties {
    /// Creates a new [`StandardProperties`] with fields actively filled from system settings.
    pub fn new() -> Result<CoreStandardProperties> {
        let settings = UserSettings::load()?;
        let creator = match settings.active_user.as_ref() {
            Some(uid) => Some(UserId::Id(uid.clone().into())),
            None => None,
        };

        let creator = Creator::User(creator);
        let mut props = CoreStandardProperties::new();
        props.creator = creator;

        Ok(props)
    }
}
