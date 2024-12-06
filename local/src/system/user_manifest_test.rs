//! # Note
//! There are concurrency errors that cause tests to fail.
//! To ensure a test is failing run it singularly.
use super::*;
use fake::faker::internet::raw::FreeEmail;
use fake::faker::name::raw::Name;
use fake::locales::EN;
use fake::Fake;
use std::sync::{Mutex, MutexGuard};
use syre_core::error::{Error as CoreError, Resource as ResourceError};

// *************
// *** tests ***
// *************

#[test]
fn user_by_email_should_work() {
    // setup
    let _m = get_lock(&MTX);

    // --- add new user
    let u0 = create_user();
    add_user(u0.clone()).expect("should work");

    let u1 = create_user();

    // test
    // --- registered
    let found = user_by_email(&u0.email).expect("find user should work");
    assert!(found.is_some(), "user should be found");

    let found = found.unwrap();
    assert_eq!(u0.rid(), found.rid(), "user ids do not match");

    // --- not registered
    let found = user_by_email(&u1.email).expect("find use should work");
    assert!(found.is_none(), "user should not be found");
}

#[test]
//#[settings_path("")]
fn add_user_should_work() {
    let _m = get_lock(&MTX);

    // add new user
    let user = create_user();
    let uid = user.rid().clone();
    add_user(user).expect("should work");

    // get current users
    let mut users = UserManifest::load().expect("users list should load");
    let user = users.get(&uid);

    assert!(user.is_some(), "user was not added");

    // clean up
    users.remove(&uid);
    users
        .save()
        .expect("could not save `Users` during clean up");
}

#[test]
fn add_user_should_error_if_email_exists() {
    // setup
    let _m = get_lock(&MTX);

    let email: String = FreeEmail(EN).fake();
    let email0 = email.clone();
    let email1 = email.clone();

    let name0: String = Name(EN).fake();
    let name1: String = Name(EN).fake();

    let user0 = User::with_name(email0, name0);
    let user1 = User::with_name(email1, name1);

    // add first user
    add_user(user0).expect("should work");

    // add second user
    match add_user(user1) {
        Ok(_) => {
            assert!(false, "should not succeed: {}", email)
        }

        Err(Error::Users(UsersError::DuplicateEmail(_))) => {} // pass

        Err(err) => {
            assert!(false, "unexpected error kind: {:?}", err)
        }
    };
}

#[test]
fn add_user_should_error_if_email_is_invalid() {
    let _m = get_lock(&MTX);

    let name: String = Name(EN).fake();
    let email = String::from("invalid");

    let user = User::with_name(email, name);
    match add_user(user) {
        Ok(_) => {
            assert!(false, "should not succeed")
        }

        Err(Error::Users(UsersError::InvalidEmail(_))) => {} // pass

        Err(err) => {
            assert!(false, "unexpected error kind: {:?}", err)
        }
    };
}

#[test]
fn delete_user_should_remove_an_existing_user() {
    let _m = get_lock(&MTX);

    // add and remove user
    let user = create_user();
    let uid = user.rid().clone();

    add_user(user).expect("add user should work");
    delete_user(&uid).expect("delete user should work");

    // check user is not in settings
    let users = UserManifest::load().expect("users list should load");
    let user = users.get(&uid);
    assert!(user.is_none(), "user was not deleted");
}

#[test]
fn delete_user_should_exit_silently_if_user_did_not_exist() {
    let _m = get_lock(&MTX);

    let user = create_user();
    delete_user(user.rid()).expect("delete user should work");

    assert_eq!(
        true, true,
        "deleting non-existant user did not exit silently"
    );
}

#[test]
fn delete_user_should_unset_as_active_user() {
    let _m = get_lock(&MTX);

    let user = create_user();
    let user_id = user.rid().clone();

    add_user(user).expect("add user should work");
    set_active_user(&user_id).expect("set active user should work");
    delete_user(&user_id).expect("delete user should work");

    let config = Config::load().expect("could not load settings");
    assert_eq!(None, config.user, "active user should not be set");

    drop(config);
    delete_user(&user_id).expect("delete user should work");
}

#[test]
fn delete_user_by_email_should_remove_an_existing_user() {
    let _m = get_lock(&MTX);

    // add and remove user
    let user = create_user();
    let user_email = user.email.clone();
    let uid = user.rid().clone();

    add_user(user).expect("add user should work");
    delete_user_by_email(&user_email).expect("delete user should work");

    // check user is not in settings
    let users = UserManifest::load().expect("users list should load");
    let user = users.get(&uid);
    assert!(user.is_none(), "user was not deleted");
}

#[test]
fn delete_user_by_email_should_exit_silently_if_user_did_not_exist() {
    let _m = get_lock(&MTX);

    let user = create_user();
    delete_user_by_email(&user.email).expect("delete user should work");
}

#[test]
fn delete_user_by_email_should_unset_as_active_user() {
    let _m = get_lock(&MTX);

    let user = create_user();
    let uid = user.rid().clone();
    let email = user.email.clone();

    add_user(user).expect("add user should work");
    set_active_user(&uid).expect("set active user should work");
    delete_user_by_email(&email).expect("delete user should work");

    let config = Config::load().expect("could not load settings");
    assert!(config.user.is_none(), "active user should not be set");

    // clean up
    let mut users = UserManifest::load().expect("could not load users");
    users.remove(&uid);
    users
        .save()
        .expect("could not save `Users` during clean up");
}

#[test]
fn update_user_should_work() {
    let _m = get_lock(&MTX);

    // setup
    let mut user = create_user();
    let uid = user.rid().clone();
    add_user(user.clone()).expect("add user should work");

    // test
    let new_user = create_user();
    let new_user_name = new_user.name.clone();
    let new_user_email = new_user.email.clone();
    user.name = new_user_name.clone();
    user.email = new_user_email.clone();

    update_user(user).expect("edit user should work");

    // check user is not in settings
    let mut users = UserManifest::load().expect("users list should load");
    let edited_user = users.get(&uid).expect("user not found");

    assert_eq!(new_user_name, edited_user.name, "name was not edited");
    assert_eq!(new_user_email, edited_user.email, "email was not edited");

    // clean up
    users.remove(&uid);
    users
        .save()
        .expect("could not save `Users` during clean up");
}

#[test]
fn update_user_should_error_if_user_does_not_exist() {
    let _m = get_lock(&MTX);

    let user = create_user();
    match update_user(user) {
        Ok(_) => {
            assert!(false, "should not succeed")
        }

        Err(Error::Core(CoreError::Resource(ResourceError::DoesNotExist(_)))) => {} // pass

        Err(err) => {
            assert!(false, "unexpected error kind: {:?}", err)
        }
    }
}

#[test]
fn update_user_should_error_if_email_is_invalid() {
    let _m = get_lock(&MTX);

    // setup
    let user = create_user();
    let mut edited_user = user.clone();
    add_user(user).expect("add user should work");

    // test
    edited_user.email = String::from("invalid_email");
    match update_user(edited_user) {
        Ok(_) => {
            assert!(false, "should not succeed")
        }
        Err(Error::Users(UsersError::InvalidEmail(_))) => {} // pass
        Err(err) => {
            assert!(false, "unexpected error kind: {:?}", err)
        }
    };
}

#[test]
fn get_active_user_should_work() {
    let _m = get_lock(&MTX);

    // with active user
    // ---setup
    let user = create_user();
    add_user(user.clone()).expect("add user should work");
    set_active_user(&user.rid()).expect("set active user should work");

    // --- test
    let active_user = get_active_user().expect("get active user should work");
    assert!(active_user.is_some(), "active user should be found");

    let active_user = active_user.expect("active user should exist");
    assert_eq!(
        user.rid(),
        active_user.rid(),
        "correct user should be found"
    );

    // no active user
    unset_active_user().expect("unset active user should work");
    let active_user = get_active_user().expect("get active user should work");
    assert_eq!(None, active_user, "active user should be `None`");

    // clean up
    delete_user(&user.rid()).expect("delete user should work");
}

#[test]
fn set_active_user_should_work() {
    let _m = get_lock(&MTX);

    // setup
    let user = create_user();
    add_user(user.clone()).expect("add user should work");
    set_active_user(&user.rid()).expect("set active user should work");
    let config = Config::load().expect("could not load settings");

    // test
    let active_user = config.user.clone();

    assert!(active_user.is_some(), "active user is None");
    let active_user = active_user.unwrap();

    assert_eq!(user.rid(), &active_user, "incorrect user is active");

    // clean up
    drop(config); // free settings so delete_user can run
    delete_user(&user.rid()).expect("delete user should work");
}

#[test]
fn set_active_user_should_error_if_user_does_not_exist() {
    let _m = get_lock(&MTX);

    let user = create_user();
    match set_active_user(&user.rid()) {
        Ok(_) => assert!(false, "should not succeed"),
        Err(Error::Core(CoreError::Resource(ResourceError::DoesNotExist(_)))) => {} // pass
        Err(err) => assert!(false, "unexpected error kind: {:?}", err),
    };
}

#[test]
fn set_active_user_by_email_should_work() {
    let _m = get_lock(&MTX);

    // setup
    let user = create_user();
    add_user(user.clone()).expect("add user should work");
    set_active_user_by_email(&user.email).expect("set active user should work");
    let config = Config::load().expect("could not load settings");

    // test
    let active_user = config.user.clone();
    assert!(active_user.is_some(), "active user is None");

    let active_user = active_user.unwrap();

    assert_eq!(user.rid(), &active_user, "incorrect user is active");

    // clean up
    drop(config); // free settings so delete_user can run
    delete_user(&user.rid()).expect("delete user should work");
}

#[test]
fn set_active_user_by_email_should_error_if_user_does_not_exist() {
    let _m = get_lock(&MTX);

    let user = create_user();
    match set_active_user_by_email(&user.email) {
        Ok(_) => assert!(false, "should not succeed"),
        Err(Error::Core(CoreError::Resource(ResourceError::DoesNotExist(_)))) => {} // pass
        Err(err) => assert!(false, "unexpected error kind: {:?}", err),
    };
}

#[test]
fn unset_active_user_should_work() {
    let _m = get_lock(&MTX);

    // setup
    let user = create_user();
    let user_id = user.rid().clone();
    add_user(user).expect("add user should work");
    set_active_user(&user_id).expect("set active user should work");

    // test
    unset_active_user().expect("unset active user should work");
    let config = Config::load().expect("could not load settings");
    assert!(config.user.is_none(), "active user still set");

    // clean up
    drop(config); // free settings so delete_user can run
    delete_user(&user_id).expect("delete user should work");
}

#[test]
fn unset_active_user_should_end_quietly_if_no_user_is_set() {
    let _m = get_lock(&MTX);

    unset_active_user().expect("unset active user should work");
    unset_active_user().expect("unset active user should work if already unset");

    let config = Config::load().expect("could not load settings");
    assert!(config.user.is_none(), "active user still set");
}

// ************************
// *** helper functions ***
// ************************

fn create_user() -> User {
    let name: String = Name(EN).fake();
    let email: String = FreeEmail(EN).fake();

    User::with_name(email, name)
}

pub fn get_lock(m: &'static Mutex<()>) -> MutexGuard<'static, ()> {
    match m.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

static MTX: Mutex<()> = Mutex::new(());
