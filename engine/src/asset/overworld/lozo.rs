use bevy::prelude::*;
use macros::{FromDef, asset_spec};

use crate::asset::{AssetRef, overworld::{object::GameObjectSpriteAsset, tile::TileAsset}};

#[derive(FromDef, Asset, TypePath)]
#[asset_spec(base_path = "game://lozo", extension = "lozo.ron")]
pub struct LozoAsset {
    pub width: u32,
    pub height: u32,
    pub tile_grid: Vec<Option<TileAsset>>,

    #[expose_resolver]
    pub objects: Vec<AssetRef<GameObjectSpriteAsset>>,
}

impl LozoAsset {
    pub fn grid_size(&self) -> UVec2 {
        UVec2::new(self.width, self.height)
    }
}
