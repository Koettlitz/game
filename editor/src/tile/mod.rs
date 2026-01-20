use bevy::prelude::*;
use engine::{
    assets::{AssetsPlugin, SpriteSheetId},
    overworld::tile::{GridSize, TileGrid},
};
use strum_macros::EnumIter;

use crate::{State, tile::visuals::TileVisualsPlugin, ui::PlaceTile};

mod visuals;

const DEFAULT_TILE_GRID_SIZE: UVec2 = UVec2::new(32, 24);

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AssetsPlugin)
            .insert_resource(GridSize(DEFAULT_TILE_GRID_SIZE))
            .init_resource::<GroundTileGrid>()
            .add_plugins(TileVisualsPlugin)
            .add_systems(Update, place_tile.run_if(in_state(State::Initialized)));
    }
}

fn place_tile(
    mut event_reader: MessageReader<PlaceTile>,
    mut commands: Commands,
    mut ground_tile_grid: ResMut<GroundTileGrid>,
) {
    let mut changed = Vec::new();
    for m in event_reader.read() {
        let tile = ground_tile_grid
            .0
            .get(m.pos)
            .unwrap_or_else(|| panic!("Invalid tile grid coords in PlaceTileEvent: {:?}", m.pos));

        if tile != m.tile_kind {
            ground_tile_grid.0.set(m.pos, m.tile_kind);
            changed.push(m.pos);
        }
    }
    commands.trigger(GroundTilesChanged(changed));
}

#[derive(Event)]
struct GroundTilesChanged(Vec<UVec2>);

#[derive(Resource)]
pub struct GroundTileGrid(pub TileGrid<GroundTile>);

impl Default for GroundTileGrid {
    fn default() -> Self {
        Self(TileGrid::new(DEFAULT_TILE_GRID_SIZE))
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Default, Debug, EnumIter)]
pub enum GroundTile {
    #[default]
    Gras,
    WaterCalm,
    WaterDeep,
}

impl GroundTile {
    fn sprite_sheet(&self) -> SpriteSheetId {
        match self {
            Self::Gras => SpriteSheetId::Outside,
            Self::WaterCalm => SpriteSheetId::WaterCalm,
            Self::WaterDeep => SpriteSheetId::WaterDeep,
        }
    }

    fn is_water(&self) -> bool {
        matches!(self, Self::WaterCalm | Self::WaterDeep)
    }
}
