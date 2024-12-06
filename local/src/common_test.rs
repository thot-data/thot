use super::*;
use fake::faker::filesystem::raw::{FileName, FilePath};
use fake::locales::EN;
use fake::Fake;
use std::{ffi::OsStr, fs, io, result::Result as StdResult};

#[test]
fn unique_file_name_should_work() {
    let mut _dir = tempfile::tempdir().unwrap();
    let base_path = _dir.path().to_path_buf();

    // already unique
    let mut p = base_path.clone();
    let f_name: String = FileName(EN).fake();
    p.push(f_name.clone());
    let q = unique_file_name(p.clone()).unwrap();

    assert_eq!(p, q, "file name should not change");

    // basic
    let p = create_named_file_in("test.txt", _dir.path()).unwrap();
    let q = unique_file_name(p.path()).unwrap();
    create_named_file_in("test (1).txt", _dir.path()).unwrap();
    let s = unique_file_name(p.path()).unwrap();

    let r = postfix_file_name(p.path().to_path_buf(), "1");
    let t = postfix_file_name(p.path().to_path_buf(), "2");

    assert_ne!(p.path(), q, "file name should change");
    assert_eq!(r, q, "unexpected file name");

    assert_ne!(p.path(), s, "file name should change");
    assert_eq!(t, s, "unexpected file name");

    // no extension
    let p = create_named_file_in("test", _dir.path()).unwrap();
    create_named_file_in("test (1)", _dir.path()).unwrap();
    let q = unique_file_name(p.path().to_path_buf()).unwrap();
    let r = p.path().parent().unwrap().join("test (2)");

    assert_ne!(p.path(), q, "file name should change");
    assert_eq!(r, q, "unexpected file name");

    // multiple extensions
    let p = create_file_with_extension_in("gz.txt", _dir.path()).unwrap();
    let q = unique_file_name(p.path().clone()).unwrap();
    let r = postfix_file_name(p.path().to_path_buf(), "1");

    assert_ne!(p.path(), q, "file name should change");
    assert_eq!(r, q, "unexpected file name");

    // beginning with `.`
    let p = tempfile::NamedTempFile::new_in(_dir.path()).unwrap();
    let p0 = p.path().file_name().unwrap().to_str().unwrap();

    let p0 = PathBuf::from(format!(".{p0}"));
    fs::rename(p, &p0).unwrap();
    let p = p0;

    let q = unique_file_name(p.clone()).unwrap();
    let r = postfix_file_name(p.clone(), "1");

    assert_ne!(p, q, "file name should change");
    assert_eq!(r, q, "unexpected file name");

    // changing digit count
    let o = create_named_file_in("digits.txt", _dir.path()).unwrap();
    let p = create_named_file_in("digits (9).txt", _dir.path()).unwrap();
    let q = unique_file_name(o.path().to_path_buf()).unwrap();
    let r = p.path().parent().unwrap().join("digits (10).txt");

    assert_ne!(o.path(), q, "file name should change");
    assert_eq!(r, q, "unexpected file name");
}

#[cfg(target_os = "windows")]
#[test]
fn ensure_windows_unc_should_work() {
    let dir = tempfile::TempDir::new().unwrap();
    let base_path = dir.path().to_path_buf();
    let canon_path = fs::canonicalize(&base_path).unwrap();

    let unc_base = ensure_windows_unc(&base_path);
    assert_eq!(canon_path, unc_base);

    let unc_canon = ensure_windows_unc(&canon_path);
    assert_eq!(canon_path, unc_canon);
}

// ******************
// *** file paths ***
// ******************

#[test]
fn app_dir_of_should_work() {
    // setup
    let base_path = FilePath(EN).fake::<String>();
    let base_path = PathBuf::from(base_path);
    let expected = base_path.join(APP_DIR);

    // test
    let path = app_dir_of(&base_path);
    assert_eq!(expected, path, "path should be correct");
}

#[test]
fn project_file_of_should_work() {
    // setup
    let base_path = FilePath(EN).fake::<String>();
    let base_path = PathBuf::from(base_path);
    let expected = base_path.join(APP_DIR).join(PROJECT_FILE);

    // test
    let path = project_file_of(&base_path);
    assert_eq!(expected, path, "path should be correct");
}

#[test]
fn project_settings_file_of_should_work() {
    // setup
    let base_path = FilePath(EN).fake::<String>();
    let base_path = PathBuf::from(base_path);
    let expected = base_path.join(APP_DIR).join(PROJECT_SETTINGS_FILE);

    // test
    let path = project_settings_file_of(&base_path);
    assert_eq!(expected, path, "path should be correct");
}

#[test]
fn container_file_of_should_work() {
    // setup
    let base_path = FilePath(EN).fake::<String>();
    let base_path = PathBuf::from(base_path);
    let expected = base_path.join(APP_DIR).join(CONTAINER_FILE);

    // test
    let path = container_file_of(&base_path);
    assert_eq!(expected, path, "path should be correct");
}

#[test]
fn container_settings_file_of_should_work() {
    // setup
    let base_path = FilePath(EN).fake::<String>();
    let base_path = PathBuf::from(base_path);
    let expected = base_path.join(APP_DIR).join(CONTAINER_SETTINGS_FILE);

    // test
    let path = container_settings_file_of(&base_path);
    assert_eq!(expected, path, "path should be correct");
}

#[test]
fn assets_file_of_should_work() {
    // setup
    let base_path = FilePath(EN).fake::<String>();
    let base_path = PathBuf::from(base_path);
    let expected = base_path.join(APP_DIR).join(ASSETS_FILE);

    // test
    let path = assets_file_of(&base_path);
    assert_eq!(expected, path, "path should be correct");
}

#[test]
fn scripts_file_of_should_work() {
    // setup
    let base_path = FilePath(EN).fake::<String>();
    let base_path = PathBuf::from(base_path);
    let expected = base_path.join(APP_DIR).join(ANALYSES_FILE);

    // test
    let path = analyses_file_of(&base_path);
    assert_eq!(expected, path, "path should be correct");
}

// ***************
// *** helpers ***
// ***************

/// Inject a postfix onto a file name.
///
/// # Examples
/// + postfix_file_name("foo.txt", "1") -> "foo (1).txt"
/// + postfix_file_name("/a/foo.txt", "1") -> "/a/foo (1).txt"
/// + postfix_file_name(".foo.txt", "1") -> ".foo (1).txt")
/// + postfix_file_name("foo", "1") -> "foo (1)")
fn postfix_file_name(path: PathBuf, postfix: impl std::fmt::Display) -> PathBuf {
    let prefix = path
        .file_prefix()
        .expect("could not get file prefix")
        .to_str()
        .expect("could not convert file name prefix to string");

    let ext = path
        .file_name()
        .expect("could not get file name")
        .to_str()
        .expect("could not convert file name to string");

    let ext = &ext[prefix.len()..];

    let name = format!("{prefix} ({postfix}){ext}");
    let mut p = path.clone();
    p.set_file_name(name);
    p
}

fn create_named_file_in(
    name: impl AsRef<OsStr>,
    parent: impl AsRef<Path>,
) -> StdResult<tempfile::NamedTempFile, io::Error> {
    let file = tempfile::NamedTempFile::new_in(parent)?;
    let mut dest = file.path().to_path_buf();
    dest.set_file_name(name);

    fs::rename(file.path(), dest)?;
    Ok(file)
}

fn create_file_with_extension_in(
    ext: impl AsRef<OsStr>,
    parent: impl AsRef<Path>,
) -> StdResult<tempfile::NamedTempFile, io::Error> {
    let file = tempfile::NamedTempFile::new_in(parent)?;
    let mut dest = file.path().to_path_buf();
    dest.set_extension(ext);

    fs::rename(file.path(), &dest)?;
    Ok(file)
}
