use bevy::prelude::*;
use engine::{
    asset::{AssetMap, AssetRef, LoadState},
    overworld::tile::{GridPosition, GridSize, TileGrid},
    progress::{Progress, ProgressPanel, ProgressState},
};

use crate::{
    asset::tile::{Tile, TileKindAsset},
    tile::visuals::TileVisualsPlugin,
    ui::PlaceTile,
};

pub mod visuals;

const DEFAULT_TILE_GRID_SIZE: UVec2 = UVec2::new(32, 20);

type TileKindMap = AssetMap<Tile, TileKindAsset>;

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GridSize::from(DEFAULT_TILE_GRID_SIZE))
            .add_plugins(TileVisualsPlugin)
            .add_systems(Startup, init_tile_grid_progress)
            .add_systems(
                Update,
                spawn_ground_tile_grid
                    .in_set(GroundTileGridInit)
                    .run_if(in_state(LoadState::<Tile>::finished()))
                    .run_if(not(resource_exists::<GroundTileGrid>)),
            )
            .add_systems(Update, place_tile.run_if(in_state(ProgressState::Finished)));
    }
}

#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Debug, Hash)]
struct GroundTileGridInit;

#[derive(Component)]
struct TileGridProgress;

#[derive(Event)]
struct GroundTilesChanged(Vec<GridPosition>);

#[derive(Resource)]
pub struct GroundTileGrid(pub TileGrid<AssetRef<TileKindAsset>>);

fn init_tile_grid_progress(mut commands: Commands) {
    commands.spawn((
        TileGridProgress,
        Progress::new(0, 1),
        ProgressPanel::new("tile grid".to_string()),
    ));
}

fn spawn_ground_tile_grid(
    mut commands: Commands,
    grid_size: Res<GridSize>,
    tile_kind_map: Res<TileKindMap>,
    mut progress: Query<&mut Progress, With<TileGridProgress>>,
) {
    let (id, tile_kind_handle) = tile_kind_map
        .iter()
        .filter(|(id, _)| *id == "grass")
        .next()
        .expect("missing ground tile kind \"grass\"");
    let grid = TileGrid::from_fn(&grid_size, |_| {
        AssetRef::new(id.clone(), tile_kind_handle.clone())
    });
    commands.insert_resource(GroundTileGrid(grid));
    let mut progress = progress.single_mut().expect("missing tile grid progress");
    progress.add(1);
}

fn place_tile(
    mut event_reader: MessageReader<PlaceTile>,
    mut commands: Commands,
    mut ground_tile_grid: ResMut<GroundTileGrid>,
    grid_size: Res<GridSize>,
) {
    let mut changed = Vec::new();
    for m in event_reader.read() {
        let tile = &ground_tile_grid.0[&m.pos.as_index(&grid_size)];

        if tile.id() != m.tile_kind.id() {
            ground_tile_grid.0[&m.pos.as_index(&grid_size)] = m.tile_kind.clone();
            changed.push(m.pos);
        }
    }
    commands.trigger(GroundTilesChanged(changed));
}
