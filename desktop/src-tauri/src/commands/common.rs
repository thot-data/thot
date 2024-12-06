use std::{
    fs, io,
    path::{Path, PathBuf},
};
use syre_desktop_lib::command::error::IoErrorKind;

#[tauri::command]
pub fn open_file(path: PathBuf) -> Result<(), IoErrorKind> {
    let path = syre_local::common::normalize_path_separators(path);
    let path = path
        .canonicalize()
        .map_err(|err| <std::io::Error as Into<IoErrorKind>>::into(err))?;
    open::that(path).map_err(|err| err.into())
}

/// Returns the target os string for which the app was built.
#[tauri::command]
pub fn target_os() -> &'static str {
    std::env::consts::OS
}

/// Gets the file of files or directories
#[tauri::command]
pub async fn file_size(paths: Vec<PathBuf>) -> Result<Vec<u64>, Vec<(PathBuf, IoErrorKind)>> {
    let sizes = paths
        .into_iter()
        .map(|path| {
            if path.is_file() {
                fs::metadata(&path)
                    .map(|metadata| metadata.len())
                    .map_err(|err| (path, err.kind().into()))
            } else if path.is_dir() {
                dir_size(&path).map_err(|err| (path, err.into()))
            } else {
                tracing::debug!(?path);
                todo!();
            }
        })
        .collect::<Vec<_>>();

    if sizes.iter().any(|size| size.is_err()) {
        let errors = sizes.into_iter().filter_map(|size| size.err()).collect();
        Err(errors)
    } else {
        Ok(sizes.into_iter().map(|size| size.unwrap()).collect())
    }
}

/// Gets the size of folder's contents.
fn dir_size(path: impl AsRef<Path>) -> Result<u64, io::ErrorKind> {
    assert!(path.as_ref().is_dir());
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            if entry.file_type().is_file() {
                fs::metadata(entry.path())
                    .ok()
                    .map(|metadata| metadata.len())
            } else {
                None
            }
        })
        .reduce(|total, size| total + size)
        .ok_or(io::ErrorKind::NotFound)
}
