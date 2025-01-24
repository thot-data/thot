use std::{
    io,
    path::{Path, PathBuf},
};
use syre_core::{
    project::{Asset, AssetProperties},
    types::ResourceId,
};
use syre_desktop_lib::{
    self as lib,
    command::asset::{bulk, error},
};
use syre_local as local;
use syre_local_database as db;

/// Update a container's properties.
#[tauri::command]
pub fn asset_properties_update(
    db: tauri::State<db::Client>,
    project: ResourceId,
    container: PathBuf,
    asset: PathBuf,
    // properties: AssetProperties,
    properties: String, // TODO: Issue with serializing enum with Option. perform manually.
                        // See: https://github.com/tauri-apps/tauri/issues/5993
) -> Result<(), error::Update> {
    let properties = serde_json::from_str::<AssetProperties>(&properties).unwrap();
    let Some((project_path, project_data)) = db.project().get_by_id(project.clone()).unwrap()
    else {
        return Err(error::Update::ProjectNotFound);
    };

    let db::state::DataResource::Ok(project_properties) = project_data.properties() else {
        panic!("invalid state");
    };
    assert_eq!(project_properties.rid(), &project);

    let data_root = project_path.join(&project_properties.data_root);
    let path = db::common::container_system_path(data_root, container);
    let mut assets = local::project::resources::Assets::load_from(path)
        .map_err(|err| error::Update::Load(err))?;
    let asset = assets
        .iter_mut()
        .find(|asset_state| asset_state.path == asset)
        .unwrap();
    asset.properties = properties;
    if let Err(err) = assets.save() {
        return Err(error::Update::Save(err.kind()));
    }

    Ok(())
}

/// Update multiple asset's properties.
#[tauri::command]
pub fn asset_properties_update_bulk(
    db: tauri::State<db::Client>,
    project: ResourceId,
    assets: Vec<bulk::ContainerAssets>,
    // update: bulk::PropertiesUpdate,
    update: String, // TODO: Issue with serializing enum with Option. perform manually.
                    // See: https://github.com/tauri-apps/tauri/issues/5993
) -> Result<Vec<Result<(), bulk::error::Update>>, lib::command::error::ProjectNotFound> {
    let update = serde_json::from_str::<bulk::PropertiesUpdate>(&update).unwrap();
    let Some((project_path, project_data)) = db.project().get_by_id(project.clone()).unwrap()
    else {
        return Err(lib::command::error::ProjectNotFound);
    };

    let db::state::DataResource::Ok(project_properties) = project_data.properties() else {
        panic!("invalid state");
    };
    assert_eq!(project_properties.rid(), &project);

    let data_root = project_path.join(&project_properties.data_root);
    Ok(assets
        .iter()
        .map(|ids| {
            let container = db::common::container_system_path(&data_root, &ids.container);
            asset_properties_update_bulk_perform(&container, &ids.assets, &update)
        })
        .collect())
}

/// Updates the assets within a container.
/// If an asset is not found, none of the assets are updated.
/// This indicates the state of the app and the file system are out of sync.
fn asset_properties_update_bulk_perform(
    container: impl AsRef<Path>,
    assets: &Vec<ResourceId>,
    update: &bulk::PropertiesUpdate,
) -> Result<(), bulk::error::Update> {
    let base_path = container.as_ref();
    let mut container_assets =
        match local::loader::container::Loader::load_from_only_assets(base_path) {
            Ok(assets) => assets,
            Err(err) => return Err(bulk::error::Update::Load(err)),
        };

    let not_found = assets
        .iter()
        .filter_map(|asset| {
            let Some(asset) = container_assets
                .iter_mut()
                .find(|container_asset| container_asset.rid() == asset)
            else {
                return Some(asset.clone());
            };

            update_asset(asset, update);
            None
        })
        .collect::<Vec<_>>();

    if !not_found.is_empty() {
        return Err(bulk::error::Update::NotFound(not_found));
    }

    if let Err(err) = container_assets.save(base_path) {
        return Err(bulk::error::Update::Save(err.kind()));
    }
    Ok(())
}

fn update_asset(asset: &mut Asset, update: &bulk::PropertiesUpdate) {
    if let Some(name) = &update.name {
        asset.properties.name = name.clone();
    }

    if let Some(kind) = &update.kind {
        asset.properties.kind = kind.clone();
    }

    if let Some(description) = &update.description {
        asset.properties.description = description.clone();
    }

    asset
        .properties
        .tags
        .retain(|tag| !update.tags.remove.contains(tag));

    let new = update
        .tags
        .insert
        .iter()
        .filter(|tag| !asset.properties.tags.contains(tag))
        .cloned()
        .collect::<Vec<_>>();
    asset.properties.tags.extend(new);

    asset
        .properties
        .metadata
        .retain(|key, _| !update.metadata.remove.contains(key));

    update
        .metadata
        .update
        .iter()
        .for_each(|(update_key, update_value)| {
            if let Some(value) = asset.properties.metadata.get_mut(update_key) {
                *value = update_value.clone();
            }
        });

    let new = update
        .metadata
        .add
        .iter()
        .filter(|(key, _)| !asset.properties.metadata.contains_key(key))
        .cloned()
        .collect::<Vec<_>>();
    asset.properties.metadata.extend(new);
}

/// Remove an asset's file.
///
/// # Arguments
/// `project`: Project id.
/// `container`: Container path.
/// `asset`: Asset path.
#[tauri::command]
pub fn asset_remove_file(
    db: tauri::State<db::Client>,
    project: ResourceId,
    container: PathBuf,
    asset: PathBuf,
) -> Result<(), lib::command::error::IoErrorKind> {
    let (project_path, project_data) = db.project().get_by_id(project).unwrap().unwrap();
    let data_root = &project_data.properties().as_ref().unwrap().data_root;
    let data_root = project_path.join(data_root);
    let container_path = lib::utils::join_path_absolute(data_root, container);
    let asset_path = container_path.join(&asset);

    if asset_path.exists() {
        trash::delete(asset_path).map_err(|err| {
            tracing::debug!(?err);
            let err = match err {
                trash::Error::Unknown { description } => todo!(),
                trash::Error::Os { code, description } => todo!(),
                trash::Error::CouldNotAccess { .. } => io::ErrorKind::PermissionDenied,
                trash::Error::CanonicalizePath { .. } => io::ErrorKind::NotFound,
                trash::Error::ConvertOsString { .. } => io::ErrorKind::InvalidFilename,
                trash::Error::TargetedRoot
                | trash::Error::RestoreCollision { .. }
                | trash::Error::RestoreTwins { .. } => unreachable!(),
            };

            err.into()
        })
    } else {
        let mut assets =
            local::loader::container::Loader::load_from_only_assets(&container_path).unwrap();
        assert!(assets
            .iter()
            .find(|stored_asset| stored_asset.path == asset)
            .is_some());

        assets.retain(|stored_asset| stored_asset.path != asset);
        assets.save(&container_path).map_err(|err| err.into())
    }
}
