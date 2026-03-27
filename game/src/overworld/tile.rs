use bevy::prelude::*;
use engine::overworld::tile::Passability;

pub struct TilePlugin;
impl Plugin for TilePlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Component)]
pub struct Tile {
    pub passability: Passability,
}
