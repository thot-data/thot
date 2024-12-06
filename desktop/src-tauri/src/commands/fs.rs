use std::path::PathBuf;
use tauri_plugin_dialog::{DialogExt, FilePath};

#[tauri::command]
pub async fn pick_folder(app: tauri::AppHandle, title: String) -> Option<PathBuf> {
    app.dialog()
        .file()
        .set_title(title)
        .blocking_pick_folder()
        .map(|path| {
            let FilePath::Path(path) = path else {
                panic!("invalid path kind");
            };
            path
        })
}

#[tauri::command]
pub async fn pick_folder_with_location(
    app: tauri::AppHandle,
    title: String,
    dir: PathBuf,
) -> Option<PathBuf> {
    app.dialog()
        .file()
        .set_title(title)
        .set_directory(dir)
        .blocking_pick_folder()
        .map(|path| {
            let FilePath::Path(path) = path else {
                panic!("invalid path kind");
            };
            path
        })
}

#[tauri::command]
pub async fn pick_file_with_location(
    app: tauri::AppHandle,
    title: String,
    dir: PathBuf,
) -> Option<PathBuf> {
    let h = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title(title)
            .set_directory(dir)
            .blocking_pick_file()
            .map(|path| {
                let FilePath::Path(path) = path else {
                    panic!("invalid path kind");
                };
                path
            })
    });

    h.await.ok().flatten()
}
