//! Handle `Asset` related functionality.
use std::path::PathBuf;

use super::super::Database;
use crate::command::asset::{AssetPropertiesUpdate, BulkUpdateAssetPropertiesArgs};
use crate::command::AssetCommand;
use crate::Result;
use serde_json::Value as JsValue;
use thot_core::error::{Error as CoreError, ResourceError};
use thot_core::project::{Asset as CoreAsset, AssetProperties, Container as CoreContainer};
use thot_core::types::ResourceId;

impl Database {
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn handle_command_asset(&mut self, cmd: AssetCommand) -> JsValue {
        match cmd {
            AssetCommand::Get(rid) => {
                let asset: Option<CoreAsset> = {
                    if let Some(container) = self.store.get_asset_container(&rid) {
                        container.assets.get(&rid).cloned().into()
                    } else {
                        None
                    }
                };

                serde_json::to_value(asset).expect("could not convert `Asset` to JSON")
            }

            AssetCommand::GetMany(rids) => {
                let assets = rids
                    .iter()
                    .filter_map(|rid| {
                        let Some(container) = self.store.get_asset_container(&rid) else {
                            return None;
                        };

                        let Some(asset) = container.assets.get(&rid) else {
                            return None;
                        };

                        Some(asset.clone())
                    })
                    .collect::<Vec<CoreAsset>>();

                serde_json::to_value(assets).expect("could not convert `Vec<Asset>` to JSON")
            }

            AssetCommand::Parent(asset) => {
                // TODO Convert to result for homogeneity with `ContainerCommand::Parent`.
                let container: Option<CoreContainer> = self
                    .store
                    .get_asset_container(&asset)
                    .map(|container| (*container).clone().into());

                serde_json::to_value(container).expect("could not convert `Container` to JSON")
            }

            AssetCommand::Add(asset, container) => {
                let res = self.store.add_asset(asset, container);
                serde_json::to_value(res).expect("could not convert result to JSON")
            }

            AssetCommand::Remove(rid) => {
                let res = self.remove_asset(&rid);
                serde_json::to_value(res).expect("could not convert result to JSON")
            }

            AssetCommand::UpdateProperties(rid, properties) => {
                let res = self.update_asset_properties(&rid, properties);
                serde_json::to_value(res).expect("could not convert result to JSON")
            }

            AssetCommand::UpdatePath(from, to) => {
                let res = self.store.update_asset_path(&from, to);
                serde_json::to_value(res).expect("could not convert result to JSON")
            }

            AssetCommand::Find(root, filter) => {
                let assets = self.store.find_assets(&root, filter);
                serde_json::to_value(assets).expect("could not convert result to JSON")
            }

            AssetCommand::FindWithMetadata(root, filter) => {
                let assets = self.store.find_assets_with_metadata(&root, filter);
                serde_json::to_value(assets).expect("could not convert result to JSON")
            }

            AssetCommand::BulkUpdateProperties(BulkUpdateAssetPropertiesArgs { rids, update }) => {
                let res = self.bulk_update_asset_properties(&rids, &update);
                serde_json::to_value(res).unwrap()
            }
        }
    }

    fn update_asset_properties(&mut self, rid: &ResourceId, properties: AssetProperties) -> Result {
        let Some(container) = self.store.get_asset_container_id(&rid).cloned() else {
            return Err(CoreError::ResourceError(ResourceError::does_not_exist(
                "`Asset` does not exist",
            ))
            .into());
        };

        let Some(container) = self.store.get_container_mut(&container) else {
            return Err(CoreError::ResourceError(ResourceError::does_not_exist(
                "`Container` does not exist",
            ))
            .into());
        };

        let Some(asset) = container.assets.get_mut(&rid) else {
            return Err(CoreError::ResourceError(ResourceError::does_not_exist(
                "`Asset` does not exist",
            ))
            .into());
        };

        asset.properties = properties;
        container.save()?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    fn remove_asset(&mut self, rid: &ResourceId) -> Result {
        let Some((asset, path)) = self.store.remove_asset(rid)? else {
            return Ok(());
        };

        trash::delete(&path)?;
        Ok(())
    }

    /// Bulk update `Asset` properties.
    #[tracing::instrument(skip(self))]
    fn bulk_update_asset_properties(
        &mut self,
        assets: &Vec<ResourceId>,
        update: &AssetPropertiesUpdate,
    ) -> Result {
        for asset in assets {
            self.update_asset_properties_from_update(asset, update)?;
        }

        Ok(())
    }

    /// Update a `Asset`'s properties.
    #[tracing::instrument(skip(self))]
    fn update_asset_properties_from_update(
        &mut self,
        rid: &ResourceId,
        update: &AssetPropertiesUpdate,
    ) -> Result {
        let Some(container) = self.store.get_asset_container_id(&rid).cloned() else {
            return Err(CoreError::ResourceError(ResourceError::does_not_exist(
                "`Asset` does not exist",
            ))
            .into());
        };

        let Some(container) = self.store.get_container_mut(&container) else {
            return Err(CoreError::ResourceError(ResourceError::does_not_exist(
                "`Container` does not exist",
            ))
            .into());
        };

        let Some(asset) = container.assets.get_mut(&rid) else {
            return Err(CoreError::ResourceError(ResourceError::does_not_exist(
                "`Asset` does not exist",
            ))
            .into());
        };

        // basic properties
        if let Some(name) = update.name.as_ref() {
            asset.properties.name = name.clone();
        }

        if let Some(kind) = update.kind.as_ref() {
            asset.properties.kind = kind.clone();
        }

        if let Some(description) = update.description.as_ref() {
            asset.properties.description = description.clone();
        }

        // tags
        asset
            .properties
            .tags
            .append(&mut update.tags.insert.clone());

        asset.properties.tags.sort();
        asset.properties.tags.dedup();
        asset
            .properties
            .tags
            .retain(|tag| !update.tags.remove.contains(tag));

        // metadata
        asset
            .properties
            .metadata
            .extend(update.metadata.insert.clone());

        for key in update.metadata.remove.iter() {
            asset.properties.metadata.remove(key);
        }

        container.save()?;
        Ok(())
    }
}
