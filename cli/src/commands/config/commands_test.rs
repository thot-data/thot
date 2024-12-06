use super::*;
use fake::faker::internet::raw::FreeEmail;
use fake::locales::EN;
use fake::Fake;
use syre_core::system::User;
use syre_local::system::config::Config;

#[test]
fn set_user_should_work_with_email() {
    let email: String = FreeEmail(EN).fake();
    let e_id = UserId::Email(email.clone());
    let user = User::new(email.clone());
    let uid = user.rid().clone();
    user_manifest::add_user(user).unwrap();
    set_active_user(&e_id).unwrap();

    let config = Config::load().expect("user settings should load");
    assert_eq!(Some(uid), config.user, "user should be set as active");
}

#[test]
fn set_user_should_work_with_id() {
    let email: String = FreeEmail(EN).fake();
    let user = User::new(email.clone());
    let uid = UserId::Id(user.rid().clone());
    let uuid = user.rid().clone();

    user_manifest::add_user(user).unwrap();
    set_active_user(&uid).unwrap();

    let config = Config::load().expect("user settings should load");
    assert_eq!(Some(uuid), config.user, "user should be set as active");
}

#[test]
#[should_panic(expected = "DoesNotExist")]
fn set_user_with_id_should_error_if_the_user_does_not_exist() {
    let email: String = FreeEmail(EN).fake();
    let user = User::new(email);
    let uid = UserId::Id(user.rid().clone());
    match set_active_user(&uid) {
        Err(err) => panic!("{:?}", err),
        _ => (),
    };
}

#[test]
#[should_panic(expected = "DoesNotExist")]
fn set_user_with_email_should_error_if_the_user_does_not_exist() {
    let email: String = FreeEmail(EN).fake();
    let e_id = UserId::Email(email.clone());
    match set_active_user(&e_id) {
        Err(err) => panic!("{:?}", err),
        _ => (),
    };
}

#[test]
fn set_user_should_error_if_the_user_is_invalid() {
    todo!();
}
