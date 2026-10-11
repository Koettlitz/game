use bevy::prelude::*;
use engine::{
    asset::AssetsExt,
    overworld::{
        lozo::{InitLozo, LozoSource},
        tile::create_grid_bundle,
    },
};

use crate::{
    lozo::{SavedLozo, SavedLozoSource},
    tile::{
        Tile, TilesChanged,
        asset::{TileEdgeConfig, TileKindAsset},
    },
};

pub struct TileGridLoadPlugin;

impl Plugin for TileGridLoadPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_tile_grid);
    }
}

#[derive(EntityEvent)]
pub struct TileGridSpawned {
    #[event_target]
    lozo_entity: Entity,
}

fn spawn_tile_grid(
    event: On<InitLozo>,
    lozos: Query<&SavedLozoSource>,
    lozo_assets: Res<Assets<SavedLozo>>,
    tile_kind_assets: Res<Assets<TileKindAsset>>,
    edge_configs: Res<Assets<TileEdgeConfig>>,
    mut commands: Commands,
) -> Result {
    let lozo = lozos.get(event.event_target())?;
    let asset = lozo_assets.require_handle(lozo.handle())?;
    let saved_tile_grid = &asset.tile_grid;

    let (grid, grid_size) = create_grid_bundle(
        UVec2::new(saved_tile_grid.width, saved_tile_grid.height),
        |pos| {
            Ok(
                if let Some(tile_kind_ref) = &saved_tile_grid.grid[*pos.as_index()] {
                    let tile_kind = tile_kind_assets.require_handle(tile_kind_ref.handle())?;
                    let edge_config = edge_configs.require_handle(&tile_kind.edge_config)?;
                    Some(Tile {
                        kind: tile_kind_ref.clone(),
                        group: edge_config.group.clone(),
                        sprite_stack: Vec::new(),
                    })
                } else {
                    None
                },
            )
        },
    )?;

    let positions = grid_size.iter_all().map(|p| *p).collect();

    commands
        .entity(event.event_target())
        .insert((grid, grid_size));

    commands.trigger(TilesChanged {
        lozo_entity: event.event_target(),
        positions,
    });

    commands.trigger(TileGridSpawned {
        lozo_entity: event.event_target(),
    });

    Ok(())
}
