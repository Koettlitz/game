use bevy::prelude::*;
use engine::asset::AssetMap;
use engine::asset::AssetSetPlugin;
use engine::asset::RonAssetPlugin;
use engine::asset::overworld::object::ObjectSpritesheet;
use macros::FromDef;
use macros::asset_set;
use serde::{Deserialize, Serialize};

pub type GameObjectKindMap = AssetMap<Object, GameObjectKindAsset>;

pub struct ObjectKindAssetPlugin;
impl Plugin for ObjectKindAssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RonAssetPlugin::<GameObjectKindAsset>::default(),
            AssetSetPlugin::<Object, GameObjectKindAsset>::default(),
        ));
    }
}

#[derive(FromDef, Asset, TypePath)]
#[def_type(GameObjectDef)]
#[asset_set(base_path = "objects")]
pub struct GameObjectKindAsset {
    pub _collision_box: Option<IRect>,
    pub sprite_sheet: ObjectSpritesheet,
}

#[derive(Serialize, Deserialize)]
pub struct GameObjectDef {
    #[serde(skip_serializing_if = "Option::is_none")]
    _collision_box: Option<IRect>,
    sprite_sheet: String,
}
