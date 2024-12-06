//! Excel template.
use crate::db::StandardSearchFilter;
use crate::project::AssetProperties;
use crate::types::ResourceId;
pub use coordinates::*;
use has_id::HasId;
use std::path::PathBuf;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "serde")]
use has_id::HasIdSerde;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, HasIdSerde))]
#[derive(PartialEq, HasId, Clone, Debug)]
pub struct ExcelTemplate {
    #[id]
    rid: ResourceId,
    pub name: Option<String>,
    pub description: Option<String>,
    pub template: TemplateParameters,
    pub input: InputParameters,
    pub output: OutputParameters,

    /// Python executable.
    pub python_exe: PathBuf,
}

impl ExcelTemplate {
    pub fn rid(&self) -> &ResourceId {
        &self.rid
    }

    /// Returns a list of supported extensions.
    pub fn supported_extensions() -> Vec<&'static str> {
        vec!["xlsx"]
    }
}

/// Describes the shape of the template and manipulations to take.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, PartialEq, Debug)]
pub struct TemplateParameters {
    /// Path to the template file.
    pub path: PathBuf,

    /// Range for new data to be copied into.
    /// Existing data in this range will be removed.
    pub replace_range: WorkbookRange,

    /// How new data should labeled.
    pub data_label_action: DataLabelAction,
}

/// Describes the shape of the input data.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone, Debug)]
pub struct InputParameters {
    /// Asset filter for input data.
    pub asset_filter: StandardSearchFilter,

    /// Where data sits in each Asset.
    pub data_selection: DataSelection,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone, Debug)]
pub struct OutputParameters {
    pub path: PathBuf,
    pub properties: AssetProperties,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone, Debug)]
pub enum DataLabelAction {
    /// No modification to input data.
    None,

    /// Insert the input sources path as an additional header above the data.
    Insert,

    /// Replace data headers with input source's path.
    Replace,
}

#[cfg(feature = "runner")]
mod runner {
    use super::*;

    impl crate::runner::Runnable for ExcelTemplate {
        fn command(&self) -> std::process::Command {
            let &ExcelTemplate {
                rid: _,
                name: _,
                description: _,
                template,
                input,
                output,
                python_exe,
            } = &self;

            // template params
            let TemplateParameters {
                path: template_path,
                replace_range,
                data_label_action,
            } = template;

            let worksheet = match &replace_range.sheet {
                WorksheetId::Name(name) => name,
                WorksheetId::Index(idx) => todo!(),
            };

            let template_path = template_path.to_str().unwrap().to_string();

            let Range {
                start: replace_start,
                end: replace_end,
            } = replace_range.range;

            let header_action = match data_label_action {
                DataLabelAction::None => "none",
                DataLabelAction::Replace => "replace",
                DataLabelAction::Insert => "insert",
            };

            // input data params
            let InputParameters {
                asset_filter,
                data_selection,
            } = input;

            let StandardSearchFilter {
                rid: _,
                name: filter_name,
                kind: filter_type,
                tags: filter_tags,
                metadata: filter_metadata,
            } = asset_filter;

            // output parameters
            let OutputParameters {
                path: output_path,
                properties: output_properties,
            } = output;

            let output_path = output_path.to_string_lossy().to_string();
            let AssetProperties {
                creator: _,
                name: output_name,
                kind: output_kind,
                description: output_description,
                tags: output_tags,
                metadata: output_metadata,
                ..
            } = output_properties;

            // command
            let mut cmd = std::process::Command::new(python_exe);
            cmd.args(vec![
                "-m".into(),
                "syre_excel_template_runner".into(),
                template_path.clone(),
                worksheet.clone(),
                format!("--replace-start={replace_start}"),
                format!("--replace-end={replace_end}"),
                format!("--output={output_path}"),
                format!("--header-action={header_action}"),
            ]);

            let (data_columns, skip_rows) = match data_selection {
                DataSelection::Spreadsheet {
                    columns,
                    skip_rows,
                    comment,
                } => {
                    cmd.arg("--data-format-type=spreadsheet");

                    if let Some(comment) = comment.as_ref() {
                        cmd.arg(format!("--comment-character={comment}"));
                    }

                    let data_columns = spreadsheet_columns_to_args(&columns);
                    (data_columns, skip_rows)
                }

                DataSelection::ExcelWorkbook {
                    sheet,
                    columns,
                    skip_rows,
                } => {
                    cmd.arg("--data-format-type=excel");

                    match sheet {
                        WorksheetId::Index(sheet) => cmd.arg(format!("--excel-sheet={sheet}")),
                        WorksheetId::Name(sheet) => cmd.arg(format!("--excel-sheet={sheet}")),
                    };

                    let data_columns = spreadsheet_columns_to_args(&columns);
                    (data_columns, skip_rows)
                }
            };

            cmd.arg(format!("--skip-rows={skip_rows}"));
            cmd.args(std::iter::once("--data-columns".to_string()).chain(data_columns.into_iter()));

            if let Some(filter_name) = filter_name {
                let filter_name = filter_name.clone().unwrap_or("''".into());
                cmd.arg(format!("--filter-name={filter_name}"));
            }

            if let Some(filter_type) = filter_type {
                let filter_type = filter_type.clone().unwrap_or("''".into());
                cmd.arg(format!("--filter-type={filter_type}"));
            }

            if let Some(filter_tags) = filter_tags {
                todo!();
            }

            if let Some(output_name) = output_name {
                cmd.arg(format!("--output-name={output_name}"));
            }

            if let Some(output_description) = output_description {
                cmd.arg(format!("--output-description={output_description}"));
            }

            if let Some(output_kind) = output_kind {
                cmd.arg(format!("--output-type={output_kind}"));
            }

            if output_tags.len() > 0 {
                todo!();
            }

            if output_metadata.len() > 0 {
                todo!();
            }

            cmd
        }
    }

    fn spreadsheet_columns_to_args(columns: &SpreadsheetColumns) -> Vec<String> {
        match columns {
            SpreadsheetColumns::Indices(indices) => {
                indices.into_iter().map(|idx| idx.to_string()).collect()
            }

            SpreadsheetColumns::Header(header) => todo!(),
        }
    }
}

pub mod coordinates {
    //! Coordinate types.
    use std::collections::HashMap;
    use std::hash::Hash;
    use std::ops::{Deref, DerefMut};

    #[cfg(feature = "serde")]
    use serde::{Deserialize, Serialize};

    use crate::project::excel_template::utils;

    pub type Index = u32;
    pub type CoordinateIndex = u32;

    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(PartialEq, Clone, Debug)]
    pub enum DataSelection {
        Spreadsheet {
            columns: SpreadsheetColumns,

            /// Number of rows to skip until meaningful data (i.e. header or data rows).
            /// For details see `pandas.read_csv`.
            skip_rows: u32,

            /// Indicates a comment character to be ignored.
            /// For details see `pandas.read_csv`.
            comment: Option<char>,
        },

        ExcelWorkbook {
            sheet: WorksheetId,
            columns: SpreadsheetColumns,

            /// Number of rows to skip until meaningful data (i.e. header or data rows).
            skip_rows: u32,
        },
    }

    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(PartialEq, Clone, Debug)]
    pub struct WorkbookRange {
        pub sheet: WorksheetId,
        pub range: Range,
    }

    impl TryFrom<String> for WorkbookRange {
        type Error = WorkbookParseError;
        fn try_from(value: String) -> Result<Self, Self::Error> {
            let Some((sheet, range)) = value.split_once('!') else {
                return Err(WorkbookParseError::InvalidSheet);
            };

            let Some((start, end)) = range.split_once(':') else {
                return Err(WorkbookParseError::InvalidRange);
            };

            let sheet = match sheet.parse::<Index>() {
                Ok(idx) => WorksheetId::Index(idx),
                Err(_) => WorksheetId::Name(sheet.to_string()),
            };

            let Ok(start) = start.parse::<CoordinateIndex>() else {
                return Err(WorkbookParseError::InvalidRange);
            };

            let Ok(end) = end.parse::<CoordinateIndex>() else {
                return Err(WorkbookParseError::InvalidRange);
            };

            Ok(Self {
                sheet,
                range: Range { start, end },
            })
        }
    }

    impl Into<String> for WorkbookRange {
        fn into(self) -> String {
            let WorkbookRange {
                sheet,
                range: Range { start, end },
            } = self;

            let sheet = match sheet {
                WorksheetId::Index(idx) => format!("[{idx}]"),
                WorksheetId::Name(name) => name,
            };

            let start = utils::index_to_column(start as usize);
            let end = utils::index_to_column(end as usize);
            format!("{}!{}:{}", sheet, start, end)
        }
    }

    pub enum WorkbookParseError {
        InvalidSheet,
        InvalidRange,
    }

    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(Hash, Eq, PartialEq, Clone, Debug)]
    pub struct WorkbookTrack {
        pub worksheet: WorksheetId,
        pub track: Index,
    }

    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(Hash, Eq, PartialEq, Clone, Debug)]
    pub struct WorkbookCoordinate {
        pub worksheet: WorksheetId,
        pub coordinate: Coordinate,
    }

    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(Hash, Eq, PartialEq, Clone, Debug)]
    pub enum WorksheetId {
        Name(String),
        Index(Index),
    }

    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(PartialEq, Clone, Debug)]
    pub enum SpreadsheetColumns {
        /// Columns are identified by data headers.
        /// A header may be multiple levels.
        Header(Vec<Vec<String>>),

        /// Columns are identified by index.
        Indices(Vec<CoordinateIndex>),
    }

    /// A track is a single column or row.
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(PartialEq, Clone, Debug)]
    pub enum TrackId {
        Name(String),
        Index(Index),
    }

    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(PartialEq, Clone, Debug)]
    pub struct Range {
        pub start: CoordinateIndex,
        pub end: CoordinateIndex,
    }

    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(Hash, Eq, PartialEq, Clone, Debug)]
    pub struct Coordinate {
        pub row: CoordinateIndex,
        pub column: CoordinateIndex,
    }

    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[cfg_attr(feature = "serde", serde(transparent))]
    #[derive(PartialEq, Clone, Debug, Default)]
    pub struct CoordinateMap<T>(HashMap<Coordinate, T>);
    impl<T> CoordinateMap<T> {
        pub fn new() -> Self {
            Self(HashMap::new())
        }

        pub fn get_coordinate(
            &self,
            row: &CoordinateIndex,
            column: &CoordinateIndex,
        ) -> Option<&T> {
            self.0.get(&Coordinate {
                row: *row,
                column: *column,
            })
        }

        pub fn insert_for(
            &mut self,
            row: CoordinateIndex,
            column: CoordinateIndex,
            value: T,
        ) -> Option<T> {
            self.0.insert(Coordinate { row, column }, value)
        }
    }

    impl<T> Deref for CoordinateMap<T> {
        type Target = HashMap<Coordinate, T>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T> DerefMut for CoordinateMap<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[cfg_attr(feature = "serde", serde(transparent))]
    #[derive(PartialEq, Clone, Debug, Default)]
    pub struct WorkbookCoordinateMap<T>(HashMap<WorkbookCoordinate, T>);
    impl<T> WorkbookCoordinateMap<T> {
        pub fn new() -> Self {
            Self(HashMap::new())
        }

        pub fn get_coordinate(
            &self,
            worksheet: &WorksheetId,
            row: &CoordinateIndex,
            column: &CoordinateIndex,
        ) -> Option<&T> {
            self.0.get(&WorkbookCoordinate {
                worksheet: worksheet.clone(),
                coordinate: Coordinate {
                    row: *row,
                    column: *column,
                },
            })
        }

        /// Get all values assocated with the given worksheet.
        ///
        /// # Returns
        /// + `None` if no entries for the worksheet exist.
        pub fn get_worksheet(&self, worksheet: &WorksheetId) -> Option<CoordinateMap<&T>> {
            let mut values = CoordinateMap::new();
            for (
                WorkbookCoordinate {
                    worksheet: wid,
                    coordinate,
                },
                value,
            ) in self.iter()
            {
                if wid == worksheet {
                    values.insert(coordinate.clone(), value);
                }
            }

            if values.is_empty() {
                None
            } else {
                Some(values)
            }
        }

        pub fn insert_for(
            &mut self,
            worksheet: WorksheetId,
            row: CoordinateIndex,
            column: CoordinateIndex,
            value: T,
        ) -> Option<T> {
            self.0.insert(
                WorkbookCoordinate {
                    worksheet,
                    coordinate: Coordinate { row, column },
                },
                value,
            )
        }
    }

    impl<T> Deref for WorkbookCoordinateMap<T> {
        type Target = HashMap<WorkbookCoordinate, T>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T> DerefMut for WorkbookCoordinateMap<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[cfg_attr(feature = "serde", serde(transparent))]
    #[derive(PartialEq, Clone, Debug, Default)]
    pub struct WorkbookTrackMap<T>(HashMap<WorkbookTrack, T>);
    impl<T> WorkbookTrackMap<T> {
        pub fn new() -> Self {
            Self(HashMap::new())
        }

        pub fn get_track(&self, worksheet: WorksheetId, track: Index) -> Option<&T> {
            self.0.get(&WorkbookTrack { worksheet, track })
        }

        pub fn insert_for(&mut self, worksheet: WorksheetId, track: Index, value: T) -> Option<T> {
            self.0.insert(WorkbookTrack { worksheet, track }, value)
        }

        /// Get all values assocated with the given worksheet.
        ///
        /// # Returns
        /// + `None` if no entries for the worksheet exist.
        pub fn get_worksheet(&self, worksheet: &WorksheetId) -> Option<HashMap<Index, &T>> {
            let mut values = HashMap::new();
            for (
                WorkbookTrack {
                    worksheet: wid,
                    track,
                },
                value,
            ) in self.iter()
            {
                if wid == worksheet {
                    values.insert(track.clone(), value);
                }
            }

            if values.is_empty() {
                None
            } else {
                Some(values)
            }
        }
    }

    impl<T> Deref for WorkbookTrackMap<T> {
        type Target = HashMap<WorkbookTrack, T>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T> DerefMut for WorkbookTrackMap<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
}

pub mod utils {
    pub const ALPHABET_LENGTH: u32 = 26;
    pub static ALPHABET: [char; ALPHABET_LENGTH as usize] = [
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];

    pub fn index_to_column(index: usize) -> String {
        if index < 26 {
            ALPHABET[index].to_ascii_uppercase().to_string()
        } else {
            let c1 = ALPHABET[index / 26].to_ascii_uppercase();
            let c2 = ALPHABET[index % 26].to_ascii_uppercase();
            format!("{c1}{c2}")
        }
    }
}
