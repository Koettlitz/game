use std::{collections::HashMap, io};

use bevy::{asset::AssetLoader, prelude::*};
use engine::assets::AssetMap;
use engine::assets::tile::derive_texture_atlas_layouts;
use engine::assets::{
    AssetSetPlugin, LoadState,
    tile::{SpriteSheet, SpriteSheetMap},
};
use macros::{FileAsset, asset_set};
use ron::de::SpannedError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub struct ObjectAssetPlugin;
impl Plugin for ObjectAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<ObjectAssetLoader>()
            .init_resource::<ObjectSpriteSheetMap>()
            .add_plugins((
                AssetSetPlugin::<ObjectAssets>::default(),
                AssetSetPlugin::<ObjectSprites>::default(),
            ))
            .add_systems(
                OnEnter(LoadState::<ObjectSprites>::finished()),
                derive_texture_atlas_layouts::<ObjectSprites, ObjectSpriteSheetMap>,
            )
            .add_systems(
                OnEnter(LoadState::<ObjectSprites>::finished()),
                cleanup.after(derive_texture_atlas_layouts::<ObjectSprites, ObjectSpriteSheetMap>),
            );
    }
}

#[asset_set(
    name = "Objects",
    extension = "obj.ron",
    folder = "editor://objects",
    asset_type(ObjectAsset)
)]
struct ObjectAssets;

#[derive(Asset, FileAsset, TypePath, Serialize, Deserialize)]
struct ObjectAsset {
    #[serde(skip_serializing_if = "Option::is_none")]
    collision_box: Option<URect>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    lozo_transitions: HashMap<IVec2, String>,
    sprite_sheet_id: String,
}

#[asset_set(
    name = "object_sprite_folder",
    folder = "objects/spritesheets",
    asset_type(Image)
)]
pub struct ObjectSprites;

#[derive(Resource, Default)]
struct ObjectSpriteSheetMap(HashMap<String, SpriteSheet>);
impl SpriteSheetMap for ObjectSpriteSheetMap {
    fn insert(&mut self, id: String, value: SpriteSheet) {
        self.0.insert(id, value);
    }
}

#[derive(Default, TypePath)]
struct ObjectAssetLoader;
impl AssetLoader for ObjectAssetLoader {
    type Asset = ObjectAsset;
    type Settings = ();
    type Error = ObjectAssetLoadError;
    async fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _: &Self::Settings,
        _: &mut bevy::asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let asset: ObjectAsset = ron::de::from_bytes(&mut bytes)?;
        Ok(asset)
    }
}

#[derive(Debug, Error)]
enum ObjectAssetLoadError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Ron(#[from] SpannedError),
}

fn cleanup(mut commands: Commands) {
    commands.remove_resource::<AssetMap<ObjectSprites>>();
}
