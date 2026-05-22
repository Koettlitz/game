use bevy::prelude::*;
use engine::asset::AssetMap;
use engine::asset::AssetRef;
use engine::asset::AssetSetPlugin;
use engine::asset::RonAssetPlugin;
use macros::FromDef;
use macros::asset_set;
use serde::{Deserialize, Serialize};

pub type GameObjectKindMap = AssetMap<ObjectResolverSet, GameObjectKindAsset>;

pub struct GameObjectAssetPlugin;
impl Plugin for GameObjectAssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RonAssetPlugin::<GameObjectKindAsset>::default(),
            AssetSetPlugin::<GameObjectKindAsset>::default(),
        ));
    }
}

#[derive(FromDef, Asset, TypePath)]
#[def_type(GameObjectDef)]
#[asset_set(base_path = "objects")]
pub struct GameObjectKindAsset {
    pub collision_box: Option<IRect>,
    #[from_def(with_spec(base_path = "objects/spritesheets"))]
    pub sprite_sheet: AssetRef<Image>,
}

#[derive(Serialize, Deserialize)]
pub struct GameObjectDef {
    #[serde(skip_serializing_if = "Option::is_none")]
    collision_box: Option<IRect>,
    sprite_sheet: String,
}
