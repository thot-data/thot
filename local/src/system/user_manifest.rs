use super::collections::user_manifest::UserManifest;
use super::config::Config;
use crate::error::{Error, IoSerde, Result, Users as UsersError};
use std::result::Result as StdResult;
use syre_core::error::{Error as CoreError, Resource as ResourceError};
use syre_core::system::User;
use syre_core::types::ResourceId;

// *************
// *** Users ***
// *************

/// Returns a user by the given id if it exists, otherwise returns an error.
pub fn user_by_id(rid: &ResourceId) -> StdResult<Option<User>, IoSerde> {
    let users = UserManifest::load()?;
    Ok(users.get(&rid).cloned())
}

/// Returns a user by the given email if it exists.
///
/// # Errors
/// + [`UsersError::DuplicateEmail`]: If multiple users are registered with the given email.
pub fn user_by_email(email: impl Into<String>) -> Result<Option<User>> {
    let email = email.into();
    let users = UserManifest::load()?;
    let users: Vec<&User> = users.iter().filter(|user| user.email == email).collect();

    match users.len() {
        0 => Ok(None),
        1 => Ok(Some(users[0].clone())),
        _ => Err(Error::Users(UsersError::DuplicateEmail(email))),
    }
}

/// Adds a user to the system settings.
pub fn add_user(user: User) -> Result {
    // validate email
    if !validator::ValidateEmail::validate_email(&user.email) {
        return Err(UsersError::InvalidEmail(user.email).into());
    }

    let mut users = UserManifest::load_or_default()?;

    // check if email already exists
    let user_count = user_count_by_email(&user.email, &users);
    if user_count > 0 {
        // same email already exists
        return Err(UsersError::DuplicateEmail(user.email.to_string()).into());
    }

    // add user
    users.push(user);
    users.save()?;
    Ok(())
}

/// Delete a user by id.
pub fn delete_user(rid: &ResourceId) -> StdResult<(), IoSerde> {
    let mut users = UserManifest::load()?;
    let mut config = Config::load()?;

    users.remove(&rid);

    // unset as active user, if required
    if config.user.as_ref() == Some(rid) {
        config.user = None;
        config.save()?;
    }

    users.save()?;
    Ok(())
}

pub fn delete_user_by_email(email: &str) -> Result {
    let Some(user) = user_by_email(email)? else {
        return Ok(());
    };

    delete_user(user.rid())?;
    Ok(())
}

/// Update the user with the given id.
///  
/// # Errors
/// + [`ResourceError::DoesNotExist`]: A [`User`] with the given id does not exist.
/// + [`UsersError::InvalidEmail`]: The updated email is invalid.
pub fn update_user(user: User) -> Result {
    // validate email
    if !validator::ValidateEmail::validate_email(&user.email) {
        return Err(UsersError::InvalidEmail(user.email).into());
    }

    let mut users = UserManifest::load()?;
    validate_id_is_present(user.rid(), &users)?;

    users.push(user);
    users.save()?;
    Ok(())
}

/// Gets the active user.
pub fn get_active_user() -> StdResult<Option<User>, IoSerde> {
    let config = Config::load_or_default()?;
    let Some(active_user) = config.user.as_ref() else {
        return Ok(None);
    };

    user_by_id(&active_user)
}

/// Sets the active user in the system settings.
///
/// # Errors
/// + If the user represented by the id is not registered.
pub fn set_active_user(rid: &ResourceId) -> Result {
    // ensure valid users
    let users = UserManifest::load()?;
    validate_id_is_present(rid, &users)?;

    // set active user
    let mut config = Config::load_or_default()?;
    config.user = Some(rid.clone());
    config.save()?;
    Ok(())
}

/// Sets the active user by email.
///
/// # Errors
/// + If the user represented by the email is not registered.
pub fn set_active_user_by_email(email: &str) -> Result {
    let user = user_by_email(email)?;
    let Some(user) = user else {
        return Err(
            CoreError::Resource(ResourceError::does_not_exist("email does not exist")).into(),
        );
    };

    let mut config = Config::load()?;
    config.user = Some(user.rid().clone());
    config.save()?;
    Ok(())
}

/// Unsets the active user.
pub fn unset_active_user() -> StdResult<(), IoSerde> {
    let mut config = Config::load()?;
    config.user = None;
    config.save()?;
    Ok(())
}

// *************************
// *** private functions ***
// *************************

/// Returns the number of users with the given email.
fn user_count_by_email(email: &str, users: &UserManifest) -> usize {
    // ensure valid users
    users
        .iter()
        .filter(|user| user.email == email)
        .collect::<Vec<&User>>()
        .len()
}

/// Validates that a user exists.
fn validate_id_is_present(rid: &ResourceId, store: &UserManifest) -> Result {
    if store.get(rid).is_none() {
        return Err(
            CoreError::Resource(ResourceError::does_not_exist("`User` does not exist.")).into(),
        );
    }

    Ok(())
}

#[cfg(test)]
#[path = "./user_manifest_test.rs"]
mod user_manifest_test;
