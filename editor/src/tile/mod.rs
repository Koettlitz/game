use std::fmt::Display;

use bevy::prelude::*;
use bevy_elf::AssetRef;
use engine::{
    asset::{AssetMap, AssetsExt, LoadState},
    overworld::tile::{Grid, GridCommands, GridSize},
    progress::{Progress, ProgressPanel, ProgressState},
};
use thiserror::Error;

use crate::{
    asset::tile::{TileEdgeConfig, TileKindAsset, TileResolverSet},
    tile::edge::TileVisualsPlugin,
    ui::{PlaceTile, RemoveTile},
};

pub mod edge;

pub const DEFAULT_TILE_KIND: &str = "grass";
const DEFAULT_TILE_GRID_SIZE: UVec2 = UVec2::new(32, 20);

type TileKindMap = AssetMap<TileResolverSet, TileKindAsset>;

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TileVisualsPlugin)
            .add_systems(Startup, init_tile_grid_progress)
            .add_systems(
                OnEnter(LoadState::<TileResolverSet>::finished()),
                spawn_tile_grid,
            )
            .add_systems(
                Update,
                ((grow_grid_to_fit_tiles, place_tile).chain(), remove_tile)
                    .run_if(in_state(ProgressState::Finished)),
            )
            .add_systems(
                Update,
                (hot_reload_tile_kinds, hot_reload_edge_configs)
                    .run_if(in_state(ProgressState::Finished)),
            );
    }
}

#[derive(Component)]
struct TileGridProgress;

#[derive(Event)]
struct TilesChanged(Vec<UVec2>);

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

fn spawn_tile_grid(
    mut commands: GridCommands,
    tile_kind_map: Res<TileKindMap>,
    tile_kinds: Res<Assets<TileKindAsset>>,
    edge_configs: Res<Assets<TileEdgeConfig>>,
    mut progress: Single<&mut Progress, With<TileGridProgress>>,
) -> Result<()> {
    let (id, tile_kind_handle) = tile_kind_map
        .iter()
        .find(|(id, _)| *id == DEFAULT_TILE_KIND)
        .ok_or_else(|| format!("missing tile kind \"{DEFAULT_TILE_KIND}\""))?;
    commands.spawn_from_fn_result(DEFAULT_TILE_GRID_SIZE, |_| {
        let tile_kind = tile_kinds.require_handle(tile_kind_handle)?;
        let edge_config = edge_configs.require_handle(&tile_kind.edge_config)?;
        Ok(Some(Tile {
            kind: AssetRef::new(id.clone(), tile_kind_handle.clone()),
            sprite_stack: Vec::default(),
            group: edge_config.group.clone(),
        }))
    })?;
    progress.add(1);
    Ok(())
}

fn grow_grid_to_fit_tiles(
    mut event_reader: MessageReader<PlaceTile>,
    tile_grid: Single<(&mut Grid<Option<Tile>>, &mut GridSize)>,
) {
    if event_reader.is_empty() {
        return;
    }
    let (mut grid, mut grid_size) = tile_grid.into_inner();
    grid.grow_to_fit(
        &mut grid_size,
        event_reader.read().map(|msg| msg.world_position),
    );
}

fn place_tile(
    mut event_reader: MessageReader<PlaceTile>,
    mut commands: Commands,
    tile_grid: Single<(&mut Grid<Option<Tile>>, &GridSize)>,
    tile_kinds: Res<Assets<TileKindAsset>>,
    edge_configs: Res<Assets<TileEdgeConfig>>,
) -> Result<()> {
    let (mut grid, grid_size) = tile_grid.into_inner();
    let mut changed = Vec::new();
    for m in event_reader.read() {
        let Some(pos) = grid_size.world_to_grid(m.world_position) else {
            error!(
                "world position {} in PlaceTile message out of bounds",
                m.world_position
            );
            continue;
        };
        if let Some(tile) = &mut grid[pos] {
            if tile.kind.id() != m.tile_kind.id() {
                tile.kind = m.tile_kind.clone();
                let tile_kind = tile_kinds.require_handle(m.tile_kind.handle())?;
                let edge_config = edge_configs.require_handle(&tile_kind.edge_config)?;
                tile.group = edge_config.group.clone();
                changed.push(*pos);
            }
        } else {
            let tile_kind = tile_kinds.require_handle(m.tile_kind.handle())?;
            let edge_config = edge_configs.require_handle(&tile_kind.edge_config)?;
            grid[pos] = Some(Tile {
                kind: m.tile_kind.clone(),
                group: edge_config.group.clone(),
                sprite_stack: Vec::default(),
            });
            changed.push(*pos);
        }
    }
    if !changed.is_empty() {
        commands.trigger(TilesChanged(changed));
    }
    Ok(())
}

fn remove_tile(
    mut message_reader: MessageReader<RemoveTile>,
    tile_grid: Single<(&mut Grid<Option<Tile>>, &GridSize)>,
    mut commands: Commands,
) {
    let (mut grid, grid_size) = tile_grid.into_inner();
    for msg in message_reader.read() {
        let Some(pos) = grid_size.world_to_grid(msg.world_position) else {
            continue;
        };
        let Some(tile) = &grid[pos] else {
            continue;
        };
        for sprite in &tile.sprite_stack {
            commands.entity(*sprite).despawn();
        }
        grid[pos] = None;
    }
}

fn hot_reload_tile_kinds(
    mut message_reader: MessageReader<AssetEvent<TileKindAsset>>,
    mut tile_kinds: ResMut<Assets<TileKindAsset>>,
    edge_configs: Res<Assets<TileEdgeConfig>>,
    images: Res<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    ground_tile_grid: Single<(&mut Grid<Option<Tile>>, &GridSize)>,
    mut commands: Commands,
) -> Result<()> {
    let (mut grid, grid_size) = ground_tile_grid.into_inner();
    let mut changed = Vec::new();
    for msg in message_reader.read() {
        let AssetEvent::LoadedWithDependencies { id } = msg else {
            continue;
        };
        let mut tile_kind = tile_kinds.require_mut(*id)?;
        let edge_config = edge_configs.require_handle(&tile_kind.edge_config)?;
        tile_kind
            .spritesheet
            .derive_layout(&images, &mut layouts)
            .ok();
        for pos in grid_size.iter_all() {
            let Some(tile) = &mut grid[pos] else {
                continue;
            };
            if tile.kind.handle().id() == *id {
                tile.group = edge_config.group.clone();
                changed.push(*pos);
            }
        }
    }
    if !changed.is_empty() {
        commands.trigger(TilesChanged(changed));
    }
    Ok(())
}

fn hot_reload_edge_configs(
    mut message_reader: MessageReader<AssetEvent<TileEdgeConfig>>,
    ground_tile_grid: Single<(&mut Grid<Option<Tile>>, &GridSize)>,
    tile_kinds: Res<Assets<TileKindAsset>>,
    edge_configs: Res<Assets<TileEdgeConfig>>,
    mut commands: Commands,
) -> Result<()> {
    let (mut grid, grid_size) = ground_tile_grid.into_inner();
    let mut changed = Vec::new();
    for msg in message_reader.read() {
        let AssetEvent::LoadedWithDependencies { id } = msg else {
            continue;
        };
        let edge_config = edge_configs.require(*id)?;
        for pos in grid_size.iter_all() {
            let Some(tile) = &mut grid[pos] else {
                continue;
            };
            let tile_kind = tile_kinds.require_handle(tile.kind.handle())?;
            if &tile_kind.edge_config.id() == id {
                tile.group = edge_config.group.clone();
                changed.push(*pos);
            }
        }
    }
    if !changed.is_empty() {
        commands.trigger(TilesChanged(changed));
    }
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
