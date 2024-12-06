use syre_core::system::User;
use syre_local::{error::IoSerde, system::user_manifest};

#[tauri::command]
pub fn register_user(email: String, name: Option<String>) -> syre_local::Result<User> {
    let user = if let Some(name) = name {
        User::with_name(email, name)
    } else {
        User::new(email)
    };

    user_manifest::add_user(user.clone())?;
    user_manifest::set_active_user(user.rid())?;

    Ok(user)
}

#[tauri::command]
pub fn login(email: String) -> syre_local::Result<User> {
    let user = user_manifest::user_by_email(&email)?;
    let Some(user) = user else {
        return Err(syre_core::error::Error::Resource(
            syre_core::error::Resource::DoesNotExist(email).into(),
        )
        .into());
    };

    user_manifest::set_active_user(user.rid())?;
    Ok(user)
}

#[tauri::command]
pub fn logout() -> Result<(), IoSerde> {
    user_manifest::unset_active_user()
}
