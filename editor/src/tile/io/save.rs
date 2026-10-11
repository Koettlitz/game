use bevy::prelude::*;
use bevy_elf::{AssetRef, FromDef};
use engine::overworld::tile::{Grid, GridSize};
use serde::{Deserialize, Serialize};

use crate::{
    lozo::LozoSaveInitialized,
    tile::{Tile, asset::TileKindAsset},
};

pub struct TileGridSavePlugin;

impl Plugin for TileGridSavePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(save_tile_grid);
    }
}

#[derive(FromDef, Component)]
#[elf(on_def(
    #[derive(Component, Serialize, Deserialize)]
))]
pub struct SavedTileGrid {
    pub width: u32,
    pub height: u32,
    pub grid: Vec<Option<AssetRef<TileKindAsset>>>,
}

#[derive(EntityEvent)]
pub struct TileGridSaved {
    pub lozo_entity: Entity,

    #[event_target]
    pub save_entity: Entity,
}

fn save_tile_grid(
    event: On<LozoSaveInitialized>,
    tile_grid: Query<(&Grid<Option<Tile>>, &GridSize)>,
    mut commands: Commands,
) -> Result {
    let (grid, grid_size) = tile_grid.get(event.lozo_entity)?;

    commands.entity(event.save_entity).insert(SavedTileGridDef {
        width: grid_size.width(),
        height: grid_size.height(),
        grid: grid_size
            .iter_all()
            .map(|pos| grid[pos].as_ref().map(|tile| tile.kind.id().to_owned()))
            .collect(),
    });

    commands.trigger(TileGridSaved {
        lozo_entity: event.lozo_entity,
        save_entity: event.save_entity,
    });
    Ok(())
}
