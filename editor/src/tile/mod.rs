use std::fmt::Display;

use bevy::prelude::*;
use engine::{
    asset::{AssetMap, AssetRef, LoadState, MissingAssetError},
    overworld::tile::{Grid, GridCommands, GridPosition, GridSize},
    progress::{Progress, ProgressPanel, ProgressState},
};
use thiserror::Error;

use crate::{asset::tile::TileKindAsset, tile::visuals::TileVisualsPlugin, ui::PlaceTile};

pub mod visuals;

const DEFAULT_TILE_GRID_SIZE: UVec2 = UVec2::new(32, 20);

type TileKindMap = AssetMap<crate::asset::tile::Tile, TileKindAsset>;

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TileVisualsPlugin)
            .add_systems(Startup, init_tile_grid_progress)
            .add_systems(
                OnEnter(LoadState::<crate::asset::tile::Tile>::finished()),
                spawn_ground_tile_grid.in_set(GroundTileGridInit),
            )
            .add_systems(Update, place_tile.run_if(in_state(ProgressState::Finished)));
    }
}

#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Debug, Hash)]
struct GroundTileGridInit;

#[derive(Component)]
struct TileGridProgress;

#[derive(Event)]
struct GroundTilesChanged(Vec<UVec2>);

#[derive(Component)]
pub struct Tile {
    pub kind: AssetRef<TileKindAsset>,
    pub sprite_stack: Vec<Entity>,
    pub group: Option<String>,
}

fn init_tile_grid_progress(mut commands: Commands) {
    commands.spawn((
        TileGridProgress,
        Progress::new(0, 1),
        ProgressPanel::new("tile grid".to_string()),
    ));
}

fn spawn_ground_tile_grid(
    mut commands: GridCommands,
    tile_kind_map: Res<TileKindMap>,
    tile_kinds: Res<Assets<TileKindAsset>>,
    mut progress: Single<&mut Progress, With<TileGridProgress>>,
) -> Result<()> {
    let (id, tile_kind_handle) = tile_kind_map
        .iter()
        .filter(|(id, _)| *id == "grass")
        .next()
        .expect("missing ground tile kind \"grass\"");
    commands.spawn_from_fn_result(DEFAULT_TILE_GRID_SIZE, |_| {
        Ok(Tile {
            kind: AssetRef::new(id.clone(), tile_kind_handle.clone()),
            sprite_stack: Vec::default(),
            group: tile_kinds
                .get(tile_kind_handle.id())
                .ok_or_else(|| MissingAssetError::new(tile_kind_handle.id()))?
                .group
                .clone(),
        })
    })?;
    progress.add(1);
    Ok(())
}

fn place_tile(
    mut event_reader: MessageReader<PlaceTile>,
    mut commands: Commands,
    ground_tile_grid: Single<(&mut Grid<Tile>, &GridSize)>,
    tile_kinds: Res<Assets<TileKindAsset>>,
) -> Result<()> {
    let (mut grid, grid_size) = ground_tile_grid.into_inner();
    let mut changed = Vec::new();
    for m in event_reader.read() {
        let Some(pos) = GridPosition::new(m.pos, &grid_size) else {
            error!("invalid position in PlaceTile message");
            continue;
        };
        let tile = &grid[pos];
        if tile.kind.id() != m.tile_kind.id() {
            grid[pos].kind = m.tile_kind.clone();
            grid[pos].group = tile_kinds
                .get(m.tile_kind.handle().id())
                .ok_or_else(|| MissingAssetError::new(m.tile_kind.handle().id()))?
                .group
                .clone();
            changed.push(m.pos);
        }
    }
    commands.trigger(GroundTilesChanged(changed));
    Ok(())
}

#[derive(Error, Debug)]
pub struct InvalidGridPosition(pub UVec2);
impl From<UVec2> for InvalidGridPosition {
    fn from(position: UVec2) -> Self {
        Self(position)
    }
}

impl Display for InvalidGridPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid grid position: {}", self.0)
    }
}
