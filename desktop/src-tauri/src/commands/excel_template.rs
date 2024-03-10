//! Excel template functions.
use calamine::{open_workbook, Xlsx};
use std::{fs, path::PathBuf};
use syre_desktop_lib::excel_template;

#[tauri::command]
pub fn load_excel(path: PathBuf) -> Result<excel_template::Workbook, String> {
    let path = syre_local::common::normalize_path_separators(path);
    let path = match fs::canonicalize(path) {
        Ok(path) => path,
        Err(err) => {
            tracing::error!(?err);
            return Err(format!("{err:?}"));
        }
    };

    match open_workbook::<Xlsx<_>, _>(path) {
        Ok(workbook) => Ok(excel_template::Workbook::from(workbook)),
        Err(err) => Err(format!("{err:?}")),
    }
}

#[tauri::command]
pub fn load_csv(path: PathBuf) -> Result<excel_template::Spreadsheet, String> {
    match csv::Reader::from_path(path) {
        Ok(reader) => Ok(excel_template::Spreadsheet::from(reader)),
        Err(err) => Err(format!("{err:?}")),
    }
}
