//! Dev utils for database.
use crate::project::{Asset, Container};
use crate::types::data::Value;
use fake::faker::filesystem::raw::FilePath;
use fake::faker::lorem::raw::{Word, Words};
use fake::locales::EN;
use fake::Fake;
use std::collections::HashMap;
use std::path::PathBuf;

/// Create a fake [`ContainerProperties`].
///
/// # Arguments
/// + `kind`: `None` if `kind` is allowed to be `None` or `Some`, chosen randomly.
///     `Some(false)` to force `kind` to be `None` or `Some(true)` to force `kind` to have a value.
pub fn mock_container(kind_none: Option<bool>) -> Container {
    // tags
    let tag_words: Vec<String> = Words(EN, 1..20).fake();

    // metadata
    let mut md = HashMap::new();
    md.insert(
        String::from("str_val"),
        Value::String(Word(EN).fake::<String>()),
    );

    md.insert(
        String::from("int_val"),
        Value::Number(rand::random::<i32>().into()),
    );

    md.insert(
        String::from("bool_val"),
        Value::Bool(rand::random::<bool>()),
    );

    // props
    let mut container = Container::new(Word(EN).fake::<String>());
    container.properties.kind = fake_opt_str(kind_none);
    container.properties.tags = tag_words;
    container.properties.metadata = md;

    container
}

/// Create a fake [`AssetProperties`].
///
/// # Arguments
/// + `name_none`: `None` if `name` is allowed to be `None` or `Some`, chosen randomly.
///     `Some(false)` to force `name` to be `None` or `Some(true)` to force `name` to have a value.
/// + `kind`: `None` if `kind` is allowed to be `None` or `Some`, chosen randomly.
///     `Some(false)` to force `kind` to be `None` or `Some(true)` to force `kind` to have a value.
pub fn mock_asset(name_none: Option<bool>, kind_none: Option<bool>) -> Asset {
    // tags
    let tag_words: Vec<String> = Words(EN, 1..20).fake();

    // metadata
    let mut md = HashMap::new();
    md.insert(
        String::from("str_val"),
        Value::String(Word(EN).fake::<String>()),
    );

    md.insert(
        String::from("int_val"),
        Value::Number(rand::random::<i32>().into()),
    );

    md.insert(
        String::from("bool_val"),
        Value::Bool(rand::random::<bool>()),
    );

    let mut path = PathBuf::from(FilePath(EN).fake::<String>());
    path.set_extension("py");

    // props
    let mut asset = Asset::new(path);
    asset.properties.name = fake_opt_str(name_none);
    asset.properties.kind = fake_opt_str(kind_none);
    asset.properties.tags = tag_words;
    asset.properties.metadata = md;

    asset
}

// ***************
// *** helpers ***
// ***************

/// Create a fake StandardProperties.
///
/// # Arguments
/// + `none`: `None` if allowed to be `None` or `Some`, chosen randomly.
///     `Some(false)` to force to be `None` or `Some(true)` to force to have a value.
fn fake_opt_str(none: Option<bool>) -> Option<String> {
    let opt_val = match none {
        None => rand::random(),
        Some(true) => true,
        Some(false) => false,
    };

    match opt_val {
        true => Some(Word(EN).fake::<String>()),
        false => None,
    }
}
