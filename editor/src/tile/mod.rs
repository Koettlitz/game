use std::{collections::HashMap, fmt::Display};

use crate::{
    tile::{asset::TileAssetPlugin, edge::TileVisualsPlugin},
    ui::{PlaceTile, RemoveTile, SpawnCursorSprite},
};
use asset::{GroundTileVisual, TileEdgeConfig, TileKindAsset};
use bevy::prelude::*;
use bevy_elf::AssetRef;
use engine::{
    animation::Animated,
    asset::AssetsExt,
    overworld::tile::{Grid, GridSize},
    progress::ProgressState,
};
use thiserror::Error;

pub mod asset;
pub mod edge;
pub mod io;

pub const DEFAULT_TILE_KIND: &str = "grass";
pub const DEFAULT_TILE_GRID_SIZE: UVec2 = UVec2::new(32, 20);

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlaceTiles>()
            .add_plugins((TileVisualsPlugin, TileAssetPlugin, io::IoPlugin))
            .add_observer(spawn_cursor_sprite)
            .add_systems(
                Update,
                ((group_place_tile_events, place_tiles).chain(), remove_tile)
                    .run_if(in_state(ProgressState::Finished)),
            )
            .add_systems(
                Update,
                (hot_reload_tile_kinds, hot_reload_edge_configs)
                    .run_if(in_state(ProgressState::Finished)),
            );
    }
}

#[derive(Event)]
struct TilesChanged {
    lozo_entity: Entity,
    positions: Vec<UVec2>,
}

pub struct Tile {
    pub kind: AssetRef<TileKindAsset>,
    pub sprite_stack: Vec<Entity>,
    pub group: Option<String>,
}

#[derive(Message)]
struct PlaceTiles {
    lozo_entity: Entity,
    placements: Vec<(Vec2, AssetRef<TileKindAsset>)>,
}

fn group_place_tile_events(
    mut event_reader: MessageReader<PlaceTile>,
    mut event_writer: MessageWriter<PlaceTiles>,
) {
    let mut by_lozo: HashMap<Entity, Vec<(Vec2, AssetRef<TileKindAsset>)>> = HashMap::new();

    for msg in event_reader.read() {
        by_lozo
            .entry(msg.lozo_entity)
            .or_default()
            .push((msg.world_position, msg.tile_kind.clone()));
    }

    for (lozo_entity, placements) in by_lozo {
        event_writer.write(PlaceTiles {
            lozo_entity,
            placements,
        });
    }
}

fn place_tiles(
    mut event_reader: MessageReader<PlaceTiles>,
    mut tile_grid: Query<(&mut Grid<Option<Tile>>, &mut GridSize)>,
    tile_kinds: Res<Assets<TileKindAsset>>,
    edge_configs: Res<Assets<TileEdgeConfig>>,
    mut commands: Commands,
) -> Result {
    for PlaceTiles {
        lozo_entity,
        placements,
    } in event_reader.read()
    {
        let (mut grid, mut grid_size) = tile_grid.get_mut(*lozo_entity)?;
        grid.grow_to_fit(&mut grid_size, placements.iter().map(|(p, _)| *p));

        let mut changed = Vec::new();

        for (world_position, tile_kind_ref) in placements {
            let Some(pos) = grid_size.world_to_grid(*world_position) else {
                error!(
                    "world position {} in PlaceTile message out of bounds",
                    world_position
                );
                continue;
            };

            let tile_kind = tile_kinds.require_handle(tile_kind_ref.handle())?;
            let edge_config = edge_configs.require_handle(&tile_kind.edge_config)?;

            if let Some(tile) = &mut grid[pos] {
                if tile.kind.handle().id() != tile_kind_ref.handle().id() {
                    tile.kind = tile_kind_ref.clone();
                    tile.group = edge_config.group.clone();
                    changed.push(*pos);
                }
            } else {
                grid[pos] = Some(Tile {
                    kind: tile_kind_ref.clone(),
                    group: edge_config.group.clone(),
                    sprite_stack: Vec::default(),
                });
                changed.push(*pos);
            }
        }

        if !changed.is_empty() {
            commands.trigger(TilesChanged {
                lozo_entity: *lozo_entity,
                positions: changed,
            });
        }
    }

    Ok(())
}

fn remove_tile(
    mut message_reader: MessageReader<RemoveTile>,
    mut tile_grid: Query<(&mut Grid<Option<Tile>>, &GridSize)>,
    mut commands: Commands,
) -> Result {
    for msg in message_reader.read() {
        let (mut grid, grid_size) = tile_grid.get_mut(msg.lozo_entity)?;

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

    Ok(())
}

fn spawn_cursor_sprite(
    event: On<SpawnCursorSprite<TileKindAsset>>,
    tile_kinds: Res<Assets<TileKindAsset>>,
    edge_configs: Res<Assets<TileEdgeConfig>>,
    mut commands: Commands,
) -> Result {
    let tile_kind = tile_kinds.require_handle(&event.asset)?;
    let edge_config = edge_configs.require_handle(&tile_kind.edge_config)?;
    let visual = edge_config.get_default();

    let (mut sprite, animation_ref) = match &visual.base() {
        GroundTileVisual::Static(idx) => (
            Sprite::from_atlas_image(
                tile_kind.spritesheet.image().clone(),
                TextureAtlas {
                    layout: tile_kind.spritesheet.layout()?.clone(),
                    index: *idx,
                },
            ),
            None,
        ),
        GroundTileVisual::Animated(animation_asset) => (
            Sprite::from_atlas_image(
                tile_kind.spritesheet.image().clone(),
                TextureAtlas {
                    layout: tile_kind.spritesheet.layout()?.clone(),
                    index: 0,
                },
            ),
            Some(animation_asset.clone()),
        ),
        GroundTileVisual::Neighbor(_) => {
            panic!("GroundTileVisual cannot have neighbor sprite as default")
        }
    };
    sprite.color = sprite.color.with_alpha(event.alpha);

    let mut cursor_commands = commands.entity(event.cursor);
    if let Some(animation_ref) = animation_ref {
        cursor_commands.with_child((sprite, Animated::by(animation_ref.handle().clone())));
    } else {
        cursor_commands.with_child(sprite);
    }

    Ok(())
}

fn hot_reload_tile_kinds(
    mut message_reader: MessageReader<AssetEvent<TileKindAsset>>,
    mut tile_kinds: ResMut<Assets<TileKindAsset>>,
    edge_configs: Res<Assets<TileEdgeConfig>>,
    images: Res<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut ground_tile_grid: Query<(Entity, &mut Grid<Option<Tile>>, &GridSize)>,
    mut commands: Commands,
) -> Result {
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

        for (lozo_entity, mut grid, grid_size) in &mut ground_tile_grid {
            let mut changed = Vec::new();

            for pos in grid_size.iter_all() {
                let Some(tile) = &mut grid[pos] else {
                    continue;
                };
                if tile.kind.handle().id() == *id {
                    tile.group = edge_config.group.clone();
                    changed.push(*pos);
                }
            }

            if !changed.is_empty() {
                commands.trigger(TilesChanged {
                    lozo_entity,
                    positions: changed,
                });
            }
        }
    }
    Ok(())
}

fn hot_reload_edge_configs(
    mut message_reader: MessageReader<AssetEvent<TileEdgeConfig>>,
    mut ground_tile_grid: Query<(Entity, &mut Grid<Option<Tile>>, &GridSize)>,
    tile_kinds: Res<Assets<TileKindAsset>>,
    edge_configs: Res<Assets<TileEdgeConfig>>,
    mut commands: Commands,
) -> Result {
    for msg in message_reader.read() {
        let AssetEvent::LoadedWithDependencies { id } = msg else {
            continue;
        };

        let edge_config = edge_configs.require(*id)?;

        for (lozo_entity, mut grid, grid_size) in &mut ground_tile_grid {
            let mut changed = Vec::new();

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

            if !changed.is_empty() {
                commands.trigger(TilesChanged {
                    lozo_entity,
                    positions: changed,
                });
            }
        }
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
