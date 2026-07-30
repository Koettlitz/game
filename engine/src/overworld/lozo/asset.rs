use std::collections::HashMap;

use bevy::prelude::*;
use bevy_elf::{AssetRef, FromDef, asset_spec};

use crate::overworld::{
    object::GameObjectSpriteAsset,
    tile::{TileAsset, TileEdge, TileEventAction},
};

#[derive(FromDef, Asset, TypePath)]
#[asset_spec(base_path = "game://lozo", extension = "lozo.ron")]
pub struct LozoAsset {
    pub width: u32,
    pub height: u32,
    pub tile_grid: Vec<Option<TileAsset>>,

    #[elf(on_def(
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    ))]
    pub char_left_events: HashMap<TileEdge, Vec<TileEventAction>>,

    #[elf(on_def(
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    ))]
    pub char_entered_events: HashMap<TileEdge, Vec<TileEventAction>>,

    #[elf(on_def(
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    ))]
    pub char_reached_events: HashMap<TileEdge, Vec<TileEventAction>>,

    #[elf(expose_resolver)]
    pub objects: Vec<AssetRef<GameObjectSpriteAsset>>,
}

impl LozoAsset {
    pub fn grid_size(&self) -> UVec2 {
        UVec2::new(self.width, self.height)
    }
}
