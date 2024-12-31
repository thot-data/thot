mod autofocus;
mod detail_popout;
pub mod drawer;
pub mod form;
mod logo;
pub mod modal;
mod toggle_expand;
mod truncate_left;

pub use autofocus::Autofocus;
pub use detail_popout::DetailPopout;
pub use drawer::Drawer;
pub use logo::Logo;
pub use modal::ModalDialog;
pub use toggle_expand::ToggleExpand;
pub use truncate_left::TruncateLeft;

pub mod icon {
    use std::{fmt::Display, path::Path};

    pub use {
        icondata::AiCloseOutlined as Close, icondata::AiMinusOutlined as Remove,
        icondata::AiPlusOutlined as Add, icondata::AiSyncOutlined as Refresh,
        icondata::AiUserOutlined as User, icondata::FaFlagRegular as Flag,
        icondata::FaPenSolid as Edit, icondata::IoSettingsSharp as Settings,
        icondata::TbEye as Eye, icondata::TbEyeClosed as EyeClosed,
        icondata::VsChevronDown as ChevronDown, icondata::VsChevronRight as ChevronRight,
    };

    /// For Tailwind to include classes
    /// they must appear as string literals in at least one place.
    /// This array is used to include them when needed.
    static _TAILWIND_CLASSES: &'static [&'static str] = &[
        "dark:text-primary-100",
        "dark:text-primary-200",
        "dark:text-primary-400",
        "dark:text-primary-500",
        "dark:text-secondary-50",
        "dark:text-syre-green-200",
        "dark:text-syre-green-400",
        "dark:text-syre-red-400",
        "dark:text-syre-red-500",
        "dark:text-syre-yellow-400",
        "dark:text-syre-yellow-500",
        "dark:text-syre-yellow-600",
        "text-primary-700",
        "text-primary-800",
        "text-primary-900",
        "text-secondary-900",
        "text-syre-green-700",
        "text-syre-green-900",
        "text-syre-red-700",
        "text-syre-red-800",
        "text-syre-yellow-700",
        "text-syre-yellow-800",
        "text-syre-yellow-900",
    ];

    type Color = &'static str;
    pub struct ThemedColor {
        light: Color,
        dark: Color,
    }

    impl Display for ThemedColor {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "text-{} dark:text-{}", self.light, self.dark)
        }
    }

    /// Get the icon id to display for file type.
    pub fn file_type_icon(path: impl AsRef<Path>) -> icondata::Icon {
        let Some(extension) = path.as_ref().extension() else {
            return icondata::FaFileRegular;
        };

        let Some(extension) = extension.to_str() else {
            return icondata::FaFileRegular;
        };

        match extension {
            "mp3" | "m4a" | "flac" | "wav" => icondata::FaFileAudioRegular,
            "py" => icondata::FaPythonBrands,
            "r" => icondata::FaRProjectBrands,
            "m" | "js" | "ts" | "cpp" | "c" | "rs" => icondata::FaFileCodeRegular,
            "csv" | "xlsx" | "xlsm" | "xml" | "odf" => icondata::FaFileExcelRegular,
            "png" | "svg" | "jpg" | "jpeg" | "tiff" | "bmp" => icondata::FaFileImageRegular,
            "txt" => icondata::FaFileLinesRegular,
            "pdf" => icondata::FaFilePdfRegular,
            "pptx" | "pptm" | "ppt" => icondata::FaFilePowerpointRegular,
            "doc" | "docm" | "docx" | "dot" => icondata::FaFileWordRegular,
            "mp4" | "mov" | "wmv" | "avi" => icondata::FaFileVideoRegular,
            "zip" | "zipx" | "rar" | "7z" | "gz" => icondata::FaFileZipperRegular,
            "dat" | "pkl" | "bin" | "exe" => icondata::OcFileBinaryLg,
            _ => icondata::FaFileRegular,
        }
    }

    /// Get the icon color to display for file type.
    ///
    /// # Returns
    /// Color class for the file type icon.
    pub fn file_type_icon_color(path: impl AsRef<Path>) -> ThemedColor {
        let Some(extension) = path.as_ref().extension() else {
            return ThemedColor {
                light: "secondary-900",
                dark: "secondary-50",
            };
        };

        let Some(extension) = extension.to_str() else {
            return ThemedColor {
                light: "secondary-900",
                dark: "secondary-50",
            };
        };

        let (light, dark) = match extension {
            "mp3" | "m4a" | "flac" | "wav" => ("syre-yellow-800", "syre-yellow-500"),
            "py" | "r" | "m" | "js" | "ts" | "cpp" | "c" | "rs" => ("primary-700", "primary-400"),
            "csv" | "xlsx" | "xlsm" | "xml" | "odf" => ("syre-green-700", "syre-green-400"),
            "png" | "svg" | "jpg" | "jpeg" | "tiff" | "bmp" => {
                ("syre-yellow-700", "syre-yellow-400")
            }
            "txt" => ("primary-800", "primary-100"),
            "pdf" => ("syre-red-800", "syre-red-500"),
            "pptx" | "pptm" | "ppt" => ("syre-red-700", "syre-red-400"),
            "doc" | "docm" | "docx" | "dot" => ("primary-800", "primary-500"),
            "mp4" | "mov" | "wmv" | "avi" => ("syre-yellow-900", "syre-yellow-600"),
            "zip" | "zipx" | "rar" | "7z" | "gz" => ("primary-900", "primary-200"),
            "dat" | "pkl" | "bin" | "exe" => ("syre-green-900", "syre-green-200"),
            _ => ("secondary-900", "secondary-50"),
        };

        ThemedColor { light, dark }
    }
}
