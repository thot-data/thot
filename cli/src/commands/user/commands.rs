use super::{AddArgs, EditUserFields};
use crate::Result;
use syre_core::error::{Error as CoreError, Resource as ResourceError};
use syre_core::system::User;
use syre_core::types::UserId;
use syre_local::system::collections::UserManifest;
use syre_local::system::user_manifest;

/// List all users.
pub fn list() -> Result {
    let users = match UserManifest::load() {
        Ok(sets) => sets,
        Err(err) => panic!("Something went wrong: {:?}", err),
    };

    let users = users
        .iter()
        .map(|user: &User| match &user.name {
            None => format!("{} ({})", user.email, user.rid()),
            Some(name) => format!("{} <{}> ({})", user.email, name, user.rid()),
        })
        .collect::<Vec<_>>();

    if users.len() == 0 {
        println!("No users");
        return Ok(());
    }

    println!("{}", users.join("\n"));
    Ok(())
}

pub fn add(user: AddArgs) -> Result {
    let u = if let Some(name) = user.name {
        User::with_name(user.email, name)
    } else {
        User::new(user.email)
    };

    match user_manifest::add_user(u) {
        Ok(_) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

pub fn delete(id: UserId) -> Result {
    let uid = match id {
        UserId::Id(u) => u,
        UserId::Email(email) => {
            let user = match user_manifest::user_by_email(&email) {
                Ok(u) => u,
                Err(err) => return Err(err.into()),
            };

            match user {
                None => return Ok(()),
                Some(u) => u.rid().clone(),
            }
        }
    };

    match user_manifest::delete_user(&uid) {
        Ok(_) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

pub fn edit(id: UserId, edits: EditUserFields) -> Result {
    let mut user: User = match id {
        UserId::Id(uid) => match user_manifest::user_by_id(&uid) {
            Ok(Some(user)) => user,
            Ok(None) => {
                return Err(CoreError::Resource(ResourceError::does_not_exist(
                    "user does not exist",
                ))
                .into());
            }
            Err(err) => return Err(err.into()),
        },
        UserId::Email(email) => match user_manifest::user_by_email(&email) {
            Ok(Some(user)) => user,
            Ok(None) => {
                return Err(CoreError::Resource(ResourceError::DoesNotExist(
                    "user with email `{email}` is not registered".to_string(),
                ))
                .into());
            }
            Err(err) => return Err(err.into()),
        },
    };

    if edits.name.is_some() {
        user.name = edits.name.unwrap();
    }

    if edits.email.is_some() {
        user.email = edits.email.unwrap();
    }

    match user_manifest::update_user(user) {
        Ok(_) => Ok(()),
        Err(err) => Err(err.into()),
    }
}
