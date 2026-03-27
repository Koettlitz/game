use std::collections::HashMap;

use bevy::prelude::*;
use engine::assets::AssetMap;
use engine::assets::GameAssetLoader;
use engine::assets::overworld::object;
use engine::assets::sprite_sheet::SpriteSheet;
use engine::assets::sprite_sheet::SpriteSheetMap;
use engine::assets::{AssetSetPlugin, LoadState};
use macros::FromDef;
use macros::asset_set;
use serde::{Deserialize, Serialize};

pub struct ObjectAssetPlugin;
impl Plugin for ObjectAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<GameObjectAsset>()
            .init_asset_loader::<GameAssetLoader<GameObjectAsset, GameObjectAsset>>()
            .init_resource::<ObjectSpriteSheetMap>()
            .add_plugins((
                AssetSetPlugin::<GameObjectAsset>::default(),
                AssetSetPlugin::<ObjectSprites>::default(),
            ))
            .add_systems(
                OnEnter(LoadState::<ObjectSprites>::finished()),
                object::derive_texture_atlas_layouts::<ObjectSprites, ObjectSpriteSheetMap>,
            )
            .add_systems(
                OnEnter(LoadState::<ObjectSprites>::finished()),
                cleanup.after(
                    object::derive_texture_atlas_layouts::<ObjectSprites, ObjectSpriteSheetMap>,
                ),
            );
    }
}

#[derive(FromDef, Asset, TypePath, Serialize, Deserialize)]
#[asset_set(
    name = "GameObjects",
    base_path = "editor://objects",
    extension = "obj.ron",
    asset_registry(crate::asset_registry),
    asset_type(Self)
)]
pub struct GameObjectAsset {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collision_box: Option<IRect>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub lozo_transitions: HashMap<IVec2, String>,
    pub sprite_sheet_id: String,
}

#[asset_set(
    name = "object_sprite_folder",
    base_path = "objects/spritesheets",
    extension = "png",
    asset_registry(crate::asset_registry),
    asset_type(Image)
)]
pub struct ObjectSprites;

#[derive(Resource, Default)]
pub struct ObjectSpriteSheetMap(HashMap<String, SpriteSheet>);
impl SpriteSheetMap for ObjectSpriteSheetMap {
    fn insert(&mut self, id: String, value: SpriteSheet) {
        self.0.insert(id, value);
    }

    fn get(&self, id: &str) -> Option<&SpriteSheet> {
        self.0.get(id)
    }

    fn remove(&mut self, id: &str) -> Option<SpriteSheet> {
        self.0.remove(id)
    }
}

fn cleanup(mut commands: Commands) {
    commands.remove_resource::<AssetMap<ObjectSprites>>();
}
