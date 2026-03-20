use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::overworld::tile::TileAsset;

#[derive(Asset, TypePath, Serialize, Deserialize)]
pub struct LozoAsset {
    pub width: usize,
    pub height: usize,
    pub tile_grid: Vec<Option<TileAsset>>,
    pub object_sprites: Vec<String>,
}
