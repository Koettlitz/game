use bevy::prelude::*;
use macros::{FromDef, asset_spec};

use crate::{asset::overworld::tile::TileAsset, overworld::tile::GridSize};

#[derive(FromDef, Asset, TypePath)]
#[asset_spec(base_path = "game://lozo", extension = "lozo.ron")]
pub struct LozoAsset {
    pub width: u32,
    pub height: u32,
    pub tile_grid: Vec<Option<TileAsset>>,
    pub game_object_ids: Vec<String>,
}

impl LozoAsset {
    pub fn grid_size(&self) -> GridSize {
        GridSize::from(UVec2::new(self.width, self.height))
    }
}
