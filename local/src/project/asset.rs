//! High level functionality for `Assets` and `Buckets`.
use super::container::path_is_container;
use super::resources::asset::{Asset as LocalAsset, Assets};
use crate::error::AssetError;
use crate::types::FsResourceAction;
use crate::{common, Error, Result};
use std::path::{Path, PathBuf};
use std::{fs, io};
use syre_core::project::Asset as CoreAsset;
use syre_core::types::ResourceId;

pub struct AssetBuilder {
    /// Path to create an [`Asset`](CoreAsset) from.
    path: PathBuf,

    /// Associated [`Container`] or `None` to use nearest found.
    container: Option<PathBuf>,

    /// The [`Asset`](CoreAsset)'s bucket.
    bucket: Option<PathBuf>,

    /// The [`AssetFileAction`] used to perform an operation that dictates all future actions,
    /// or `None` if no such action has been taken.
    action: Option<FsResourceAction>,

    /// Path to the moved file, or `None` if the file was not moved.
    moved_asset_path: Option<PathBuf>,
}

impl AssetBuilder {
    pub fn new(path: PathBuf) -> Self {
        AssetBuilder {
            path,
            container: None,
            bucket: None,
            action: None,
            moved_asset_path: None,
        }
    }

    /// Gets the calculated [`Container`](syre_core::project::Container) path.
    ///
    /// # Errors
    /// + [`AssetError::PathNotAContainer`]: If `container` is provided,
    ///     but is not a valid [`Container`](super::resources::Container).
    pub fn container_path(&self) -> Result<PathBuf> {
        let container = match self.container.clone() {
            Some(p) => {
                if !path_is_container(&p) {
                    return Err(AssetError::PathNotAContainer(self.path.clone()).into());
                }

                p
            }
            None => match self.path.parent() {
                Some(p) => container_from_path_ancestor(&p)?,
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidFilename,
                        "path is not a valid filename",
                    )
                    .into());
                }
            },
        };

        Ok(container)
    }

    pub fn set_container(&mut self, container: PathBuf) -> &mut Self {
        self.container = Some(container);
        self
    }

    pub fn unset_container(&mut self) -> &mut Self {
        self.container = None;
        self
    }

    pub fn set_bucket(&mut self, bucket: PathBuf) -> &mut Self {
        self.bucket = Some(bucket);
        self
    }

    pub fn unset_bucket(&mut self) -> &mut Self {
        self.bucket = None;
        self
    }

    /// Calulates the final path of the [`Asset`](CoreAsset) given the action
    /// and current file system state.
    ///
    /// # Errors
    /// + [`AssetError::InvalidPath`] if the path does not have a valid file name.
    ///
    /// # Note
    /// If the state of the file system changes, the **result of this function may change**.
    pub fn tentative_final_path(&self, action: FsResourceAction) -> Result<PathBuf> {
        // calculate paths
        let mut rel_path = self.bucket.clone().unwrap_or(PathBuf::new());
        let Some(file_name) = self.path.file_name() else {
            return Err(AssetError::InvalidPath(
                self.path.clone(),
                "could not get file name".to_string(),
            )
            .into());
        };

        rel_path.push(file_name);

        let mut abs_path = self.container_path()?;
        abs_path.push(rel_path.clone());

        if self.path == abs_path {
            return Ok(self.path.clone());
        }

        let abs_path = common::unique_file_name(abs_path)?;
        match action {
            FsResourceAction::Reference => Ok(self.path.clone()),
            FsResourceAction::Move => Ok(abs_path),
            FsResourceAction::Copy => Ok(abs_path),
        }
    }

    /// Move the [`Asset`](CoreAsset) file to the correct position given the action.
    fn move_file(&mut self) -> Result {
        let Some(action) = self.action.clone() else {
            return Err(AssetError::BuilderError("action not set".to_string()).into());
        };

        let to_path = self.tentative_final_path(action.clone())?;

        // move file if needed
        match action {
            FsResourceAction::Move => {
                if self.path != to_path {
                    fs::rename(&self.path, &to_path).expect("could not rename `Asset` file");
                    self.moved_asset_path = Some(to_path.clone());
                }
            }
            FsResourceAction::Copy => {
                if self.path != to_path {
                    fs::copy(&self.path, &to_path).expect("could not copy `Asset` file");
                    self.action = Some(action);
                    self.moved_asset_path = Some(to_path.clone());
                }
            }
            FsResourceAction::Reference => {}
        }

        Ok(())
    }

    /// Calculates the [`ResourcePath`] for the [`Asset`](CoreAsset)'s path.
    fn resource_path(&self) -> Result<PathBuf> {
        let Some(action) = self.action.clone() else {
            return Err(AssetError::BuilderError("action not set".to_string()).into());
        };

        let container = self.container_path()?;
        let path = match action {
            FsResourceAction::Move | FsResourceAction::Copy => {
                let path = match self.moved_asset_path.as_ref() {
                    Some(path) => path.clone(),
                    None => self.path.clone(),
                };

                path.strip_prefix(&container)
                    .expect("could not calculate relative path for `Asset` file")
                    .to_path_buf()
            }

            FsResourceAction::Reference => self.path.clone(),
        };

        Ok(path)
    }

    /// Initializes an [`Asset`](CoreAsset).
    /// This **does not** register it with its parent Container.
    /// This **does not** move the file.
    ///
    /// # Arguments
    /// 1. `path` may point to a non-existant file.
    /// 2. If `container` is None, assumes the parent folder is the desired Container.
    ///
    /// # Errors
    /// + [`io::ErrorKind::InvalidFilename`]: If the `path` argument is not a valid path name.
    /// + [`AssetError::PathNotAContainer`]: If the `container` argument is provided,
    ///    but the given path is not initialized as a [`Container`].
    ///
    /// # See also
    /// + [`create`](Self::create)
    /// + [`add`](Self::add)
    pub fn init(self) -> Result<ResourceId> {
        // create asset
        let asset = LocalAsset::new(self.path.clone())?;
        let rid = asset.rid().clone();

        // insert asset
        let container = self.container_path()?;
        let mut assets = Assets::load_from(container)?;
        assets.push(asset);
        assets.save()?;
        Ok(rid)
    }

    /// Creates an [`Asset`](CoreAsset) from a file.
    /// Moves the file if needed, based on the action.
    /// This **does not** add or register the [`Asset`](CoreAsset) with the [`Container`](CoreContainer).
    ///
    /// # Returns
    /// The final [`Asset`](CoreAsset).
    /// Note that the path may be changed, and the file may be moved.
    ///
    /// # See also
    /// + [`init`](Self::init)
    /// + [`add`](Self::add)
    pub fn create(mut self, action: FsResourceAction) -> Result<CoreAsset> {
        // validate bucket + action
        if self.bucket.is_some() && action == FsResourceAction::Reference {
            return Err(AssetError::IncompatibleAction(
                "`Assets` can not have a `Reference` path and a `bucket`".to_string(),
            )
            .into());
        }

        self.action = Some(action);

        // create asset
        self.move_file()?;
        let path = self.resource_path()?;
        let asset = LocalAsset::new(path).expect("could not create `Asset`");

        Ok(asset)
    }

    /// Adds an [`Asset`](CoreAsset) to a [`Container`](CoreContainer).
    /// Moves the file if needed, based on the action.
    /// This **does not** register the [`Asset`](CoreAsset) with the [`Container`](CoreContainer).
    ///
    /// # Returns
    /// The final [`Asset`](CoreAsset).
    ///
    /// # See also
    /// + [`init`](Self::init)
    /// + [`create`](Self::create)
    pub fn add(mut self, action: FsResourceAction) -> Result<CoreAsset> {
        // validate bucket + action
        if self.bucket.is_some() && action == FsResourceAction::Reference {
            return Err(AssetError::IncompatibleAction(
                "`Assets` can not have a `Reference` path and a `bucket`".to_string(),
            )
            .into());
        }

        self.action = Some(action);

        // create asset
        self.move_file()?;
        let path = self.resource_path()?;
        let asset = LocalAsset::new(path).expect("could not create `Asset`");

        // insert asset
        let container = self.container_path()?;
        let mut assets = Assets::load_from(container)?;
        assets.push(asset.clone());
        assets.save()?;

        Ok(asset)
    }
}

// *****************
// *** Functions ***
// *****************

/// Initializes an [`Asset`](CoreAsset).
/// This **does not** register it with its parent Container.
/// This **does not** move the file.
///
/// # Arguments
/// 1. `path` may point to a non-existant file.
/// 2. If `container` is None, assumes the parent folder is the desired Container.
///
/// # Errors
/// + [`io::ErrorKind::InvalidFilename`]: If the `path` argument is not a valid path name.
/// + [`AssetError::PathNotAContainer`]: If the `container` argument is provided,
///    but the given path is not initialized as a [`Container`].
///
/// # See also
/// + [`create`]
#[deprecated(note = "Use `AssetBuilder`")]
pub fn init(path: &Path, container: Option<&Path>) -> Result<ResourceId> {
    // validate container
    let container = match container {
        Some(p) => {
            if !path_is_container(p) {
                return Err(AssetError::PathNotAContainer(PathBuf::from(path)).into());
            }

            p.to_path_buf()
        }
        None => match path.parent() {
            Some(p) => container_from_path_ancestor(&p)?,
            None => {
                // @unreachable
                return Err(io::Error::new(
                    io::ErrorKind::InvalidFilename,
                    "container could not be determined from path",
                )
                .into());
            }
        },
    };

    // create asset
    let asset = LocalAsset::new(path)?;
    let rid = asset.rid().clone();

    // insert asset
    let mut assets = Assets::load_from(container)?;
    assets.push(asset);
    assets.save()?;
    Ok(rid)
}

pub fn mv() -> Result {
    todo!();
}

pub fn delete() -> Result {
    todo!();
}

pub fn update() -> Result {
    todo!();
}

/// Make a new bucket directory.
pub fn add_bucket(name: &Path) -> Result {
    todo!();
}

/// Make an existing directory a bucket.
pub fn make_bucket(path: &Path) -> Result {
    todo!();
}

/// Returns whether the path is an Asset registered with the given Container.
pub fn path_is_asset(path: &Path, container: &Path) -> bool {
    todo!();
}

// TODO Return `Option` instead of `Result`.
/// Moves up the path until the first `Container` is reached.
pub fn container_from_path_ancestor(path: &Path) -> Result<PathBuf> {
    match path.ancestors().find(|p| path_is_container(p)) {
        None => {
            return Err(Error::AssetError(AssetError::ContainerNotFound(
                path.to_path_buf(),
            )))
        }
        Some(p) => Ok(p.to_path_buf()),
    }
}

/// Calculates the path of an [`Asset`](syre_core::project::Asset)'s file
/// if it were to be moved into the given [`Container`](syre_core::project::Container) and `bucket`
/// with the given [action](AssetFileAction).
pub fn unique_asset_file_path(
    path: &Path,
    container_path: &Path,
    action: &FsResourceAction,
    bucket: Option<&Path>,
) -> PathBuf {
    let c_path = || {
        let rel_path = path.file_name().expect("could not get `Asset` file_name");
        let rel_path = PathBuf::from(rel_path);
        let rel_path = if let Some(bucket) = bucket {
            let mut path = bucket.to_path_buf();
            path.push(rel_path);
            path
        } else {
            rel_path
        };

        let mut container_path = container_path.to_path_buf();
        container_path.push(rel_path);
        container_path
    };

    match action {
        FsResourceAction::Reference => path.to_path_buf(),
        FsResourceAction::Move => c_path(),
        FsResourceAction::Copy => c_path(),
    }
}

#[cfg(test)]
#[path = "./asset_test.rs"]
mod asset_test;
