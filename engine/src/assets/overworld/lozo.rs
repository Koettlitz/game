use bevy::prelude::*;
use macros::{FromDef, resolver};

use crate::{assets::overworld::tile::TileAsset, overworld::tile::GridSize};

#[derive(FromDef, Asset, TypePath)]
#[resolver(base_path = "game://lozo", extension = "lozo.ron", asset_type(Self))]
pub struct LozoAsset {
    pub width: u32,
    pub height: u32,
    pub tile_grid: Vec<Option<Handle<TileAsset>>>,
    pub game_object_ids: Vec<String>,
}

impl LozoAsset {
    pub fn grid_size(&self) -> GridSize {
        GridSize::from(UVec2::new(self.width, self.height))
    }
}
