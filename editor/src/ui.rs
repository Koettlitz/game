use bevy::prelude::*;

use crate::tile::GroundTileKind;

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Cursor>();
    }
}

#[derive(Resource, Default)]
pub enum Cursor {
    #[default]
    Default,
    GroundTile(GroundTileKind)
}

#[derive(Event)]
pub struct PlaceTileEvent {
    tile_coords: UVec2,
    tile_kind: GroundTileKind,
}

impl PlaceTileEvent {
    pub fn coords(&self) -> UVec2 {
        self.tile_coords
    }

    pub fn tile_kind(&self) -> GroundTileKind {
        self.tile_kind
    }
}

