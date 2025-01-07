//! Common use functions.
use crate::constants::*;
use rayon::prelude::*;
use regex::Regex;
use std::{
    ffi::OsString,
    io,
    path::{Component, Path, PathBuf, Prefix, MAIN_SEPARATOR},
    sync::{Arc, Mutex},
};

/// Creates a unique file name.
pub fn unique_file_name(path: impl AsRef<Path>) -> Result<PathBuf, io::ErrorKind> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(path.to_path_buf());
    }

    // get file name
    let Some(file_prefix) = path.file_prefix() else {
        return Err(io::ErrorKind::InvalidFilename);
    };

    let Some(file_prefix) = file_prefix.to_str() else {
        return Err(io::ErrorKind::InvalidFilename);
    };

    // get extension
    let Some(ext) = path.file_name() else {
        return Err(io::ErrorKind::InvalidFilename);
    };
    let Some(ext) = ext.to_str() else {
        return Err(io::ErrorKind::InvalidFilename);
    };
    let ext = &ext[file_prefix.len()..];

    let Some(parent) = path.parent() else {
        return Err(io::ErrorKind::InvalidFilename);
    };

    // get highest counter
    let name_pattern = Regex::new(&format!(r"{file_prefix} \((\d+)\){ext}$")).unwrap();
    let mut highest = None;
    for entry in std::fs::read_dir(parent).map_err(|err| err.kind())? {
        let entry_path = entry.map(|entry| entry.path()).map_err(|err| err.kind())?;
        let Some(entry_file_name) = entry_path
            .file_name()
            .map(|filename| filename.to_str())
            .flatten()
        else {
            continue;
        };

        let Some(captures) = name_pattern.captures(entry_file_name) else {
            continue;
        };

        if let Some(n) = captures.get(1) {
            let Ok(n) = n.as_str().parse::<u32>() else {
                continue;
            };

            match highest {
                None => {
                    let n = std::cmp::max(n, 1);
                    let _ = highest.insert(n);
                }
                Some(m) if n > m => {
                    let _ = highest.insert(n);
                }
                _ => {}
            }
        }
    }

    // set unique file name
    let mut file_name = file_prefix.to_string();
    match highest {
        None => file_name.push_str(" (1)"),
        Some(n) => {
            file_name.push_str(&format!(" ({})", n + 1));
        }
    };
    file_name.push_str(ext);

    let mut unique_path = path.to_path_buf();
    unique_path.set_file_name(file_name);
    Ok(unique_path)
}

/// Replaces any non-alphanumeric or standard characters with underscore (_).
pub fn sanitize_file_path(path: impl Into<String>) -> String {
    let path: String = path.into();
    let char_whitelist = vec!['-', '_', '.', ' ', '(', ')', '[', ']'];
    path.chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() || char_whitelist.contains(&char) {
                char
            } else {
                '_'
            }
        })
        .collect()
}

/// Normalizes path separators to the current systems.
///
/// On Windows this is `\\`.
/// On all other systems this is `/`.
pub fn normalize_path_separators(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref()
        .components()
        .fold(PathBuf::new(), |path, component| match component {
            Component::RootDir => path.join(MAIN_SEPARATOR.to_string()),
            Component::Prefix(prefix) => path.join(prefix.as_os_str()),
            Component::Normal(segment) => path.join(segment),
            _ => {
                panic!("invalid path component");
            }
        })
}

/// Root path for the current system.
///
/// On Windows this is `\\`.
/// On all other systems this is `/`.
pub fn root_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    return PathBuf::from("\\");
    #[cfg(not(target_os = "windows"))]
    return PathBuf::from("/");
}

/// Prefixes the path with the [Windows UNC](https://learn.microsoft.com/en-us/dotnet/standard/io/file-path-formats#unc-paths) path if it is not already there.
pub fn ensure_windows_unc(path: impl Into<PathBuf>) -> PathBuf {
    let path: PathBuf = path.into();
    if path.to_str().unwrap().starts_with(WINDOWS_UNC_PREFIX) {
        path
    } else {
        // Must prefix UNC path as `str` because using `Path`s strips it.
        let mut p = WINDOWS_UNC_PREFIX.to_string();
        p.push_str(path.to_str().unwrap());
        PathBuf::from(p)
    }
}

/// Strip the UNC prefix from a Windows path.
/// If the UNC prefix is not present, the path is returned as is.
pub fn strip_windows_unc(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref()
        .components()
        .filter(|component| match component {
            Component::Prefix(prefix) => match prefix.kind() {
                Prefix::Disk(_) => true,
                _ => false,
            },
            _ => true,
        })
        .fold(PathBuf::new(), |path, component| path.join(component))
}

/// Recursively copy a folder and its contents.
///
/// # Returns
/// `Err` if any path fails to be copied.
///
/// # Notes
/// Spawns threads for copying.
pub fn copy_dir(
    src: impl AsRef<Path>,
    dst: impl AsRef<Path>,
) -> Result<(), Vec<(PathBuf, io::ErrorKind)>> {
    let src: &Path = src.as_ref();
    let dst: &Path = dst.as_ref();

    let mut errors = vec![];
    let mut files = vec![];
    for entry in walkdir::WalkDir::new(src)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let rel_path = entry.path().strip_prefix(src).unwrap();
        let dst = dst.join(rel_path);

        if entry.file_type().is_file() {
            files.push((entry.path().to_path_buf(), dst));
        } else if entry.file_type().is_dir() {
            if let Err(err) = std::fs::create_dir(dst) {
                errors.push((entry.path().to_path_buf(), err.kind()));
            }
        } else {
            todo!();
        };
    }

    let errors = Arc::new(Mutex::new(errors));
    files.into_par_iter().for_each({
        let errors = errors.clone();
        move |(file_src, file_dst)| {
            if let Err(err) = std::fs::copy(&file_src, &file_dst) {
                errors.lock().unwrap().push((file_src, err.kind()));
            }
        }
    });

    let errors = Arc::into_inner(errors).unwrap().into_inner().unwrap();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// ******************
// *** file paths ***
// ******************

/// Returns the relative path to the Syre directory from a base path.
pub fn app_dir() -> &'static Path {
    Path::new(APP_DIR)
}

/// Path to the Syre directory for a given path.
/// \<path\>/\<APP_DIR\>.
pub fn app_dir_of(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref().join(APP_DIR)
}

/// Path to the project file for a given path.
pub fn project_file() -> PathBuf {
    app_dir().join(PROJECT_FILE)
}

/// Path to the project file for a given path.
/// `app_dir_of(path)/\<PROJECT_FILE\>`
pub fn project_file_of(path: impl AsRef<Path>) -> PathBuf {
    app_dir_of(path).join(PROJECT_FILE)
}

/// Path to the project settings file relative to a base path.
pub fn project_settings_file() -> PathBuf {
    app_dir().join(PROJECT_SETTINGS_FILE)
}

/// Path to the project settings file for a given path.
/// `app_dir_of(path)/\<PROJECT_SETTINGS_FILE\>`
pub fn project_settings_file_of(path: impl AsRef<Path>) -> PathBuf {
    app_dir_of(path).join(PROJECT_SETTINGS_FILE)
}

/// Path to the project runner settings file relative to a base path.
pub fn project_runner_settings_file() -> PathBuf {
    app_dir().join(PROJECT_RUNNER_SETTINGS_FILE)
}

/// Path to the project runner settings file for a given path.
/// `app_dir_of(path)/\<PROJECT_RUNNER_SETTINGS_FILE\>`
pub fn project_runner_settings_file_of(path: impl AsRef<Path>) -> PathBuf {
    app_dir_of(path).join(PROJECT_RUNNER_SETTINGS_FILE)
}

/// Path to the Container file from a base path.
pub fn container_file() -> PathBuf {
    app_dir().join(CONTAINER_FILE)
}

/// Path to the Container file for a given path.
/// `app_dir_of(path)/\<CONTAINER_FILE\>`
pub fn container_file_of(path: impl AsRef<Path>) -> PathBuf {
    app_dir_of(path).join(CONTAINER_FILE)
}

/// Path to the Container settings file from a base path.
pub fn container_settings_file() -> PathBuf {
    app_dir().join(CONTAINER_SETTINGS_FILE)
}

/// Path to the Container settings file for a given path.
/// `app_dir_of(path)/\<CONTAINER_SETTINGS_FILE\>`
pub fn container_settings_file_of(path: impl AsRef<Path>) -> PathBuf {
    app_dir_of(path).join(CONTAINER_SETTINGS_FILE)
}

/// Path to the Assets file from a base path.
pub fn assets_file() -> PathBuf {
    app_dir().join(ASSETS_FILE)
}

/// Path to the Assets file for a given path.
/// `app_dir_of(path)/\<ASSETS_FILE\>`
pub fn assets_file_of(path: impl AsRef<Path>) -> PathBuf {
    app_dir_of(path).join(ASSETS_FILE)
}

/// Path to the flags file relative to a base path.
pub fn flags_file() -> PathBuf {
    app_dir().join(FLAGS_FILE)
}

/// Path to the flags file for a given path.
/// `app_dir_of(path)/\<FLAGS_FILE\>`
pub fn flags_file_of(path: impl AsRef<Path>) -> PathBuf {
    app_dir_of(path).join(FLAGS_FILE)
}

/// Path to the Assets file from a base path.
pub fn analyses_file() -> PathBuf {
    app_dir().join(ANALYSES_FILE)
}

/// Path to the analyses file for a given path.
/// `app_dir_of(path)/\<ANALYSES_FILE\>`
pub fn analyses_file_of(path: impl AsRef<Path>) -> PathBuf {
    app_dir_of(path).join(ANALYSES_FILE)
}

/// Path to the ignore file for a given path.
pub fn ignore_file() -> OsString {
    OsString::from(IGNORE_FILE)
}

/// Path to the ignore file for a given path.
/// <path>/\<IGNORE_FILE\>
pub fn ignore_file_of(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref().join(IGNORE_FILE)
}

pub mod fs {
    //! Function that modify the file system.
    use crate::constants;
    use std::{ffi::OsString, io, path::Path};

    const TEMPFILE_NAME_LEN: usize = 6;

    /// Temporary directory.
    /// Removes directory on drop.
    pub struct TempDir {
        /// Absolute path of the directory.
        path: Box<Path>,
    }

    impl TempDir {
        pub fn path(&self) -> &Path {
            self.path.as_ref()
        }

        #[cfg(target_os = "windows")]
        /// Creates a temporary directory in a given parent directory.
        /// The directory name is prefixed by [`constants::TEMPFILE_PREFIX`] and hidden.
        ///
        /// # Notes
        /// + No cleanup is performed for the directory.
        pub fn hidden_in(parent: impl AsRef<Path>) -> io::Result<Self> {
            let dir = Self::create_in(&parent)?;
            hide_folder(dir.path()).map(|_| dir)
        }

        #[cfg(not(target_os = "windows"))]
        /// Creates a temporary directory in a given parent directory.
        /// The directory name is prefixed by [`constants::TEMPFILE_PREFIX`].
        ///
        /// # Returns
        /// Name of the temporary directory.
        pub fn hidden_in(parent: impl AsRef<Path>) -> io::Result<Self> {
            Self::create_in(parent)
        }

        /// Creates a temporary directory in a given parent directory.
        /// The directory name is prefixed by [`constants::TEMPFILE_PREFIX`].
        ///
        /// # Returns
        /// Name of the temporary directory.
        fn create_in(parent: impl AsRef<Path>) -> io::Result<Self> {
            let path = parent.as_ref().join(Self::tmpname());
            std::fs::create_dir(&path)?;
            Ok(Self {
                path: path.into_boxed_path(),
            })
        }

        /// Creates a filename for a temporary directory.
        /// The name is prefixed by [`constants::TEMPFILE_PREFIX`].
        fn tmpname() -> OsString {
            let tmpname = std::iter::repeat_with(fastrand::alphanumeric)
                .take(TEMPFILE_NAME_LEN)
                .collect::<String>();

            let mut filename = OsString::from(constants::TEMPFILE_PREFIX);
            filename.push(tmpname);
            filename
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            if let Err(err) = std::fs::remove_dir_all(self.path()) {
                tracing::error!(
                    "could not remove temporary directory {:?}: {err:?}",
                    self.path()
                );
            }
        }
    }

    #[cfg(target_os = "windows")]
    pub fn hide_folder(path: impl AsRef<Path>) -> io::Result<()> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem;

        let filename_win = path
            .as_ref()
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<u16>>();

        let res = unsafe {
            FileSystem::SetFileAttributesW(filename_win.as_ptr(), FileSystem::FILE_ATTRIBUTE_HIDDEN)
        };

        if res == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(target_os = "windows")]
    pub fn unhide_folder(path: impl AsRef<Path>) -> io::Result<()> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem;

        let filename_win = path
            .as_ref()
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<u16>>();

        let res = unsafe {
            FileSystem::SetFileAttributesW(filename_win.as_ptr(), FileSystem::FILE_ATTRIBUTE_NORMAL)
        };

        if res == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

pub mod ignore {
    use std::path::Path;

    pub struct WalkBuilder;
    impl WalkBuilder {
        pub fn new(path: impl AsRef<Path>) -> ignore::WalkBuilder {
            let mut builder = ignore::WalkBuilder::new(path);
            builder.add_custom_ignore_filename(super::ignore_file());
            builder
        }
    }
}

#[cfg(test)]
#[path = "./common_test.rs"]
mod common_test;
