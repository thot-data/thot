use super::*;
use fake::faker::internet::raw::FreeEmail;
use fake::faker::name::raw::Name;
use fake::locales::EN;
use fake::Fake;

#[test]
fn new_should_work_without_name() {
    let email: String = FreeEmail(EN).fake();
    let user = User::new(email.clone());

    assert_eq!(email, user.email, "email should be set");
}

#[test]
fn new_should_work_with_name() {
    let email: String = FreeEmail(EN).fake();
    let name: String = Name(EN).fake();
    let user = User::with_name(email.clone(), name.clone());

    assert_eq!(email, user.email, "email should be set");
    assert_eq!(Some(name), user.name, "name should be set");
}
