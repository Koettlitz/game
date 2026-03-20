use bevy::prelude::*;
use engine::overworld::tile::TileGrid;

pub struct TilePlugin;
impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_tile_grid);
    }
}

fn init_tile_grid(mut commands: Commands, asset_server: Res<AssetServer>) {}
