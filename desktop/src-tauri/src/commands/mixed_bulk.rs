//! Bulk Operations on mixed resources.
use std::path::{Path, PathBuf};
use syre_core::{project::Asset, types::ResourceId};
use syre_desktop_lib::command;
use syre_local as local;
use syre_local_database as db;

/// # Returns
/// Results each resources update as (containers, assets).
#[tauri::command]
pub fn properties_update_bulk_mixed(
    db: tauri::State<db::Client>,
    project: ResourceId,
    containers: Vec<PathBuf>,
    assets: Vec<command::asset::bulk::ContainerAssets>,
    // update: bulk::PropertiesUpdate,
    update: String, // TODO: Issue with serializing enum with Option. perform manually.
                    // See: https://github.com/tauri-apps/tauri/issues/5993
) -> Result<
    (
        Vec<Result<(), command::container::bulk::error::Update>>,
        Vec<Result<(), command::asset::bulk::error::Update>>,
    ),
    command::error::ProjectNotFound,
> {
    let update = serde_json::from_str::<command::bulk::PropertiesUpdate>(&update).unwrap();
    let Some((project_path, project_data)) = db.project().get_by_id(project.clone()).unwrap()
    else {
        return Err(command::error::ProjectNotFound);
    };

    let db::state::DataResource::Ok(project_properties) = project_data.properties() else {
        panic!("invalid state");
    };
    assert_eq!(project_properties.rid(), &project);

    let data_root = project_path.join(&project_properties.data_root);
    let container_results = containers
        .iter()
        .map(|container| {
            let path = db::common::container_system_path(&data_root, container);
            properties_update_bulk_perform_container(&path, &update)
        })
        .collect();

    let asset_results = assets
        .iter()
        .map(|ids| {
            let container = db::common::container_system_path(&data_root, &ids.container);
            properties_update_bulk_perform_asset(&container, &ids.assets, &update)
        })
        .collect();

    Ok((container_results, asset_results))
}

fn properties_update_bulk_perform_container(
    path: impl AsRef<Path>,
    update: &command::bulk::PropertiesUpdate,
) -> Result<(), command::container::bulk::error::Update> {
    let mut container =
        match local::loader::container::Loader::load_from_only_properties(path.as_ref()) {
            Ok(container) => container,
            Err(err) => return Err(command::container::bulk::error::Update::Load(err)),
        };

    if let Some(kind) = &update.kind {
        container.properties.kind = kind.clone();
    }

    if let Some(description) = &update.description {
        container.properties.description = description.clone();
    }

    container
        .properties
        .tags
        .retain(|tag| !update.tags.remove.contains(tag));

    let new = update
        .tags
        .insert
        .iter()
        .filter(|tag| !container.properties.tags.contains(tag))
        .cloned()
        .collect::<Vec<_>>();
    container.properties.tags.extend(new);

    container
        .properties
        .metadata
        .retain(|key, _| !update.metadata.remove.contains(key));

    update
        .metadata
        .update
        .iter()
        .for_each(|(update_key, update_value)| {
            if let Some(value) = container.properties.metadata.get_mut(update_key) {
                *value = update_value.clone();
            }
        });

    let new = update
        .metadata
        .add
        .iter()
        .filter(|(key, _)| !container.properties.metadata.contains_key(key))
        .cloned()
        .collect::<Vec<_>>();
    container.properties.metadata.extend(new);
    if let Err(err) = container.save(&path) {
        return Err(command::container::bulk::error::Update::Save(err.kind()));
    }

    Ok(())
}

/// Updates the assets within a container.
/// If an asset is not found, none of the assets are updated.
/// This indicates the state of the app and the file system are out of sync.
fn properties_update_bulk_perform_asset(
    container: impl AsRef<Path>,
    assets: &Vec<ResourceId>,
    update: &command::bulk::PropertiesUpdate,
) -> Result<(), command::asset::bulk::error::Update> {
    let base_path = container.as_ref();
    let mut container_assets =
        match local::loader::container::Loader::load_from_only_assets(base_path) {
            Ok(assets) => assets,
            Err(err) => return Err(command::asset::bulk::error::Update::Load(err)),
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
        return Err(command::asset::bulk::error::Update::NotFound(not_found));
    }

    if let Err(err) = container_assets.save(base_path) {
        return Err(command::asset::bulk::error::Update::Save(err.kind()));
    }
    Ok(())
}

fn update_asset(asset: &mut Asset, update: &command::bulk::PropertiesUpdate) {
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
