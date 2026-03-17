use std::{collections::HashMap, fmt::Display};

use bevy::log::tracing::instrument;
use bevy::prelude::*;
use engine::{
    Id,
    animation::SpriteAnimation,
    assets::{AssetMap, EntityLookupMap, LoadState, SpriteSheet, folder::FolderProgress},
    overworld::tile::{GridPosition, GridSize, TileGrid},
    progress::{Progress, ProgressPanel, ProgressState},
};

use crate::{
    assets::{
        animation::AnimationFolder,
        tile::{GroundTileAsset, GroundTileAssetFolder, TileSpriteAssets, TileSpriteSheetMap},
    },
    tile::visuals::{GroundTileVisuals, TileVisualsPlugin},
    ui::PlaceTile,
};

pub mod visuals;

const DEFAULT_TILE_GRID_SIZE: UVec2 = UVec2::new(32, 20);

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GridSize::from(DEFAULT_TILE_GRID_SIZE))
            .init_resource::<UnlinkedGroundTileKinds>()
            .add_plugins(TileVisualsPlugin)
            .add_systems(Startup, init_tile_grid_progress)
            .add_observer(init_link_progress)
            .add_systems(
                PostUpdate,
                spawn_tile_kinds.run_if(in_state(LoadState::<GroundTileAssetFolder>::loading())),
            )
            .add_systems(
                Update,
                link_tile_kinds
                    .run_if(resource_exists::<UnlinkedGroundTileKinds>)
                    .run_if(ready_to_link_tiles),
            )
            .add_systems(
                Update,
                spawn_ground_tile_grid
                    .in_set(GroundTileGridInit)
                    .after(link_tile_kinds)
                    .run_if(not(resource_exists::<GroundTileGrid>))
                    .run_if(ready_to_link_tiles),
            )
            .add_systems(Update, place_tile.run_if(in_state(ProgressState::Finished)));
    }
}

#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Debug, Hash)]
struct GroundTileGridInit;

fn init_tile_grid_progress(mut commands: Commands) {
    commands.spawn((
        TileGridProgress,
        Progress::new(0, 1),
        ProgressPanel::new("Spawning tiles".to_string()),
    ));
}

#[derive(Component)]
struct TileGridProgress;

fn init_link_progress(
    event: On<Add, FolderProgress<GroundTileAssetFolder>>,
    query: Query<&Progress, With<FolderProgress<GroundTileAssetFolder>>>,
    mut commands: Commands,
) {
    let progress = query
        .get(event.entity)
        .expect("on add event fired, but entity missing");
    commands.spawn((
        LinkProgress,
        Progress::new(0, progress.max()),
        ProgressPanel::new("Linking tiles".to_string()),
    ));
}

#[derive(Component)]
struct LinkProgress;

fn spawn_tile_kinds(
    mut commands: Commands,
    mut assets: ResMut<Assets<GroundTileAsset>>,
    mut asset_map: ResMut<AssetMap<GroundTileAssetFolder>>,
    mut tile_kinds: ResMut<UnlinkedGroundTileKinds>,
) {
    let mut spawned_ids = Vec::new();
    for (id, handle) in asset_map.0.iter() {
        let Some(asset) = assets.remove(handle.id()) else {
            continue;
        };
        let entity = commands.spawn((GroundTileKind, Id(id.clone()))).id();
        tile_kinds.0.insert(id.clone(), (asset, entity));
        spawned_ids.push(id.clone());
    }
    for spawned_id in spawned_ids {
        asset_map.0.remove(&spawned_id);
    }
}

#[derive(Resource, Default)]
struct UnlinkedGroundTileKinds(HashMap<String, (GroundTileAsset, Entity)>);

fn link_tile_kinds(
    mut commands: Commands,
    unlinked: Res<UnlinkedGroundTileKinds>,
    animations: Res<EntityLookupMap<SpriteAnimation>>,
    mut sprite_sheets: ResMut<TileSpriteSheetMap>,
    mut progress: Query<&mut Progress, With<LinkProgress>>,
) {
    let mut progress = progress.single_mut().expect("missing link progress");
    for (tile_kind_id, (asset, entity)) in unlinked.0.iter() {
        let Some(sprite_sheet) = sprite_sheets.0.remove(tile_kind_id) else {
            error!("missing sprite sheet for tile kind \"{tile_kind_id}\"");
            continue;
        };
        let Some(bundle) = link_tile_kind(
            tile_kind_id,
            asset,
            |id| unlinked.0.get(id).map(|v| v.1),
            |id| animations.0.get(id).copied(),
            sprite_sheet,
        ) else {
            continue;
        };
        commands.entity(*entity).insert(bundle);
        progress.add(1);
    }
    let max = progress.max();
    progress.add(max);
    commands.remove_resource::<UnlinkedGroundTileKinds>();
    commands.remove_resource::<AssetMap<GroundTileAssetFolder>>();
    commands.remove_resource::<TileSpriteSheetMap>();
}

#[cfg_attr(
    debug_assertions,
    instrument(
        level = "debug",
        skip(asset, tile_kind_lookup, animation_lookup, sprite_sheet)
    )
)]
fn link_tile_kind(
    tile_kind_id: &str,
    asset: &GroundTileAsset,
    tile_kind_lookup: impl Fn(&str) -> Option<Entity>,
    animation_lookup: impl Fn(&str) -> Option<Entity>,
    sprite_sheet: SpriteSheet,
) -> Option<impl Bundle> {
    let tile_kind_lookup = |id: &str| {
        tile_kind_lookup(id).ok_or_else(|| TileKindLoadingError::InvalidTileKindLink {
            identity: tile_kind_id.to_string(),
            link: id.to_string(),
        })
    };
    let animation_lookup = |id: &str| {
        animation_lookup(id).ok_or_else(|| TileKindLoadingError::MissingAnimation {
            tile_kind_id: tile_kind_id.to_string(),
            animation_id: id.to_string(),
        })
    };
    let visuals =
        GroundTileVisuals::from_config(asset.visuals.iter(), tile_kind_lookup, animation_lookup);
    let Some(visuals) = visuals else {
        return None;
    };
    Some((asset.passability, visuals, sprite_sheet))
}

fn ready_to_link_tiles(
    ground_tile_state: Res<State<LoadState<GroundTileAssetFolder>>>,
    sprite_sheet_state: Res<State<LoadState<TileSpriteAssets>>>,
    animation_state: Res<State<LoadState<AnimationFolder>>>,
) -> bool {
    ground_tile_state.get().is_finished()
        && sprite_sheet_state.get().is_finished()
        && animation_state.get().is_finished()
}

fn spawn_ground_tile_grid(
    mut commands: Commands,
    grid_size: Res<GridSize>,
    tile_kinds: Query<(Entity, &Id), With<GroundTileKind>>,
    mut progress: Query<&mut Progress, With<TileGridProgress>>,
) {
    let ground_tile_entity = tile_kinds
        .iter()
        .filter(|(_, id)| id.0 == "grass")
        .next()
        .expect("missing ground tile kind \"grass\"")
        .0;
    let grid = TileGrid::new(&grid_size, || ground_tile_entity);
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
        let tile = ground_tile_grid.0[&m.pos.as_index(&grid_size)];

        if tile != m.tile_kind {
            ground_tile_grid.0[&m.pos.as_index(&grid_size)] = m.tile_kind;
            changed.push(m.pos);
        }
    }
    commands.trigger(GroundTilesChanged(changed));
}

#[derive(Component)]
pub struct GroundTileKind;

#[derive(Event)]
struct GroundTilesChanged(Vec<GridPosition>);

#[derive(Resource)]
pub struct GroundTileGrid(pub TileGrid<Entity>);

#[derive(Debug)]
pub enum TileKindLoadingError {
    MissingAnimation {
        tile_kind_id: String,
        animation_id: String,
    },
    InvalidTileKindLink {
        identity: String,
        link: String,
    },
}
impl Display for TileKindLoadingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingAnimation {
                tile_kind_id,
                animation_id,
            } => {
                write!(
                    f,
                    "missing animation \"{animation_id}\" referenced in tile kind \"{tile_kind_id}\""
                )
            }
            Self::InvalidTileKindLink { identity, link } => {
                write!(
                    f,
                    "Invalid link to other tile kind \"{link}\" in config of \"{identity}\""
                )
            }
        }
    }
}
impl std::error::Error for TileKindLoadingError {}
