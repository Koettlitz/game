use std::collections::HashSet;

use bevy::prelude::*;
use engine::{
    animation::Animated,
    asset::{AssetsExt, animation::sprite::SpriteAnimationAsset},
    overworld::tile::{Grid, GridPosition, GridSize},
    progress::ProgressState,
};
use bevy_elf::AssetRef;

use super::spawn_tile_grid;
use crate::{
    asset::tile::{GroundTileVisual, TileEdgeConfig, TileKindAsset, TileKindSpritesheet},
    tile::{InvalidGridPosition, Tile, TilesChanged},
};

pub struct TileVisualsPlugin;
impl Plugin for TileVisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(ProgressState::Finished),
            init_sprite_grid.after(spawn_tile_grid),
        )
        .add_observer(on_ground_tile_changed)
        .add_observer(update_sprites);
    }
}

#[derive(Event)]
struct UpdateTileSprites(UVec2);

fn init_sprite_grid(mut commands: Commands, grid_size: Single<&GridSize>) {
    for pos in grid_size.iter_all() {
        commands.trigger(UpdateTileSprites(*pos));
    }
}

fn on_ground_tile_changed(
    event: On<TilesChanged>,
    mut commands: Commands,
    grid_size: Single<&GridSize>,
) {
    let mut sprites_to_update: HashSet<UVec2> = HashSet::new();
    for position in &event.0 {
        let Some(position) = GridPosition::new(*position, &grid_size) else {
            error!("invalid position in GroundTilesChangedEvent: {position}");
            continue;
        };
        for neighbor in position.around_inclusive().into_iter().flatten() {
            sprites_to_update.insert(*neighbor);
        }
    }
    for sprite_to_update in sprites_to_update {
        commands.trigger(UpdateTileSprites(sprite_to_update));
    }
}

fn update_sprites(
    event: On<UpdateTileSprites>,
    mut commands: Commands,
    tile_grid: Single<(&mut Grid<Option<Tile>>, &GridSize)>,
    tile_kinds: Res<Assets<TileKindAsset>>,
    edge_configs: Res<Assets<TileEdgeConfig>>,
) -> Result<()> {
    let (mut tile_grid, grid_size) = tile_grid.into_inner();
    let position =
        GridPosition::new(event.0, grid_size).ok_or_else(|| InvalidGridPosition(event.0))?;
    let (id, tile_kind_asset) = if let Some(tile) = &mut tile_grid[position] {
        for old_sprite in tile.sprite_stack.drain(..) {
            commands.entity(old_sprite).despawn();
        }
        (
            tile.kind.id().to_string(),
            tile_kinds.require_handle(tile.kind.handle())?,
        )
    } else {
        return Ok(());
    };
    let layers = &edge_configs.require_handle(&tile_kind_asset.edge_config)?
        .edge_cases
        .iter()
        .find(|(req, _)| 
            req.matches(&tile_grid.cursor_at(position))
        )
        .expect("no adjacent requirement matched the current surroundings - this should never happen because there needs to be a default")
        .1;
    for (z, layer) in layers.iter() {
        let sprite_entity = spawn_tile_sprite(
            &id,
            &position,
            layer,
            &tile_kind_asset.spritesheet,
            z,
            &mut commands,
            &tile_kinds,
            &edge_configs,
            grid_size,
            &tile_grid,
        )?;
        if let Some(entity) = sprite_entity {
            tile_grid[position]
                .as_mut()
                .unwrap()
                .sprite_stack
                .push(entity);
        }
    }
    Ok(())
}

#[derive(Component)]
pub struct TileSprite(String);

impl TileSprite {
    pub fn id(&self) -> &str {
        &self.0
    }
}

fn spawn_tile_sprite(
    id: &str,
    position: &GridPosition,
    layer: &GroundTileVisual,
    spritesheet: &TileKindSpritesheet,
    z: f32,
    commands: &mut Commands,
    tile_kinds: &Assets<TileKindAsset>,
    edge_configs: &Assets<TileEdgeConfig>,
    grid_size: &GridSize,
    tile_grid: &Grid<Option<Tile>>,
) -> Result<Option<Entity>> {
    let transform =
        Transform::from_translation(grid_size.grid_to_world(position.as_vec2()).extend(z));
    match layer {
        GroundTileVisual::Static(idx) => {
            let atlas = TextureAtlas {
                layout: spritesheet.layout()?.clone(),
                index: *idx,
            };
            let sprite = Sprite::from_atlas_image(spritesheet.image().clone(), atlas);
            let entity = commands
                .spawn((TileSprite(id.to_string()), transform, sprite))
                .id();
            Ok(Some(entity))
        }
        GroundTileVisual::Animated(animation_asset) => {
            let atlas = TextureAtlas {
                layout: spritesheet.layout()?.clone(),
                index: 0,
            };
            let sprite = Sprite::from_atlas_image(spritesheet.image().clone(), atlas);
            let entity = commands
                .spawn((
                    TileSprite(id.to_string()),
                    transform,
                    sprite,
                    Animated::by(animation_asset.handle().clone()),
                    AnimationId(animation_asset.id().to_string()),
                ))
                .id();
            Ok(Some(entity))
        }
        GroundTileVisual::Neighbor(neighbor) => {
            let Some(neighbor_position) = position.neighbor(neighbor) else {
                return Ok(None);
            };
            let Some(neighbor_tile) = &tile_grid[neighbor_position] else {
                return Ok(None);
            };
            let neighbor_tile_kind = tile_kinds.require_handle(neighbor_tile.kind.handle())?;
            let layer = edge_configs
                .require_handle(&neighbor_tile_kind.edge_config)?
                .get_default()
                .base();
            spawn_tile_sprite(
                neighbor_tile.kind.id(),
                position,
                layer,
                &neighbor_tile_kind.spritesheet,
                z,
                commands,
                tile_kinds,
                edge_configs,
                grid_size,
                tile_grid,
            )
        }
    }
}

pub fn create_tile_sprite(
    spritesheet: &TileKindSpritesheet,
    visuals: &TileEdgeConfig,
) -> Result<(Sprite, Option<AssetRef<SpriteAnimationAsset>>)> {
    let visual = visuals.get_default();
    Ok(match &visual.base() {
        GroundTileVisual::Static(idx) => (
            Sprite::from_atlas_image(
                spritesheet.image().clone(),
                TextureAtlas {
                    layout: spritesheet.layout()?.clone(),
                    index: *idx,
                },
            ),
            None,
        ),
        GroundTileVisual::Animated(animation_asset) => (
            Sprite::from_atlas_image(
                spritesheet.image().clone(),
                TextureAtlas {
                    layout: spritesheet.layout()?.clone(),
                    index: 0,
                },
            ),
            Some(animation_asset.clone()),
        ),
        GroundTileVisual::Neighbor(_) => {
            panic!("GroundTileVisual cannot have neighbor sprite as default")
        }
    })
}

#[derive(Component)]
pub struct AnimationId(pub String);
