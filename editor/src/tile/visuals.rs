use std::collections::HashSet;

use bevy::prelude::*;
use engine::{
    animation::Animated,
    asset::{
        AssetRef, MissingAssetError, animation::sprite::SpriteAnimationAsset,
        overworld::tile::TileKindSpritesheet,
    },
    overworld::tile::{Grid, GridPosition, GridSize},
    progress::ProgressState,
};

use super::spawn_ground_tile_grid;
use crate::{
    asset::tile::{GroundTileVisual, TileEdgeConfig, TileKindAsset},
    tile::{InvalidGridPosition, Tile, TilesChanged},
};

pub struct TileVisualsPlugin;
impl Plugin for TileVisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(ProgressState::Finished),
            init_sprite_grid.after(spawn_ground_tile_grid),
        )
        .add_observer(on_ground_tile_changed)
        .add_observer(update_sprites);
    }
}

#[derive(Event)]
struct UpdateTileSprites(UVec2);

fn init_sprite_grid(mut commands: Commands, grid_size: Single<&GridSize>) {
    for pos in grid_size.iter() {
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
        for neighbor in position.around_inclusive().into_iter().filter_map(|p| p) {
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
    ground_tile_grid: Single<(&mut Grid<Tile>, &GridSize)>,
    tile_kinds: Res<Assets<TileKindAsset>>,
    edge_configs: Res<Assets<TileEdgeConfig>>,
) -> Result<()> {
    let (mut tile_grid, grid_size) = ground_tile_grid.into_inner();
    let position =
        GridPosition::new(event.0, &grid_size).ok_or_else(|| InvalidGridPosition(event.0))?;
    let tile_kind_asset = tile_kinds
        .get(tile_grid[position].kind.handle().id())
        .expect("missing visual for ground tile");
    for old_sprite in tile_grid[position].sprite_stack.drain(..) {
        commands.entity(old_sprite).despawn();
    }
    let layers = &edge_configs
        .get(tile_kind_asset.edge_config.id())
        .ok_or_else(|| MissingAssetError::new(tile_kind_asset.edge_config.id()))?
        .edge_cases
        .iter()
        .find(|(req, _)| req.matches(&tile_grid.cursor_at(position)))
        .expect("no adjacent requirement matched the current surroundings")
        .1;
    for (z, layer) in layers.iter() {
        let sprite_entity = spawn_tile_sprite(
            &position,
            layer,
            &tile_kind_asset.spritesheet,
            z,
            &mut commands,
            &tile_kinds,
            &edge_configs,
            &grid_size,
            &tile_grid,
        )?;
        if let Some(entity) = sprite_entity {
            tile_grid[position].sprite_stack.push(entity);
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
    position: &GridPosition,
    layer: &GroundTileVisual,
    spritesheet: &TileKindSpritesheet,
    z: f32,
    commands: &mut Commands,
    tile_kinds: &Assets<TileKindAsset>,
    edge_configs: &Assets<TileEdgeConfig>,
    grid_size: &GridSize,
    tile_grid: &Grid<Tile>,
) -> Result<Option<Entity>> {
    let transform = Transform::from_translation(grid_size.to_world_pos(**position).extend(z));
    match layer {
        GroundTileVisual::Static(idx) => {
            let atlas = TextureAtlas {
                layout: spritesheet.layout()?.clone(),
                index: *idx,
            };
            let entity = commands
                .spawn((
                    TileSprite(spritesheet.id().to_string()),
                    Sprite::from_atlas_image(spritesheet.image().clone(), atlas),
                    transform,
                ))
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
                    TileSprite(spritesheet.id().to_string()),
                    sprite,
                    Animated::by(animation_asset.handle().clone()),
                    AnimationId(animation_asset.id().to_string()),
                    transform,
                ))
                .id();
            Ok(Some(entity))
        }
        GroundTileVisual::Neighbor(neighbor) => {
            let Some(neighbor_position) = position.neighbor(&neighbor) else {
                return Ok(None);
            };
            let neighbor = &tile_grid[neighbor_position];
            let neighbor = tile_kinds
                .get(neighbor.kind.handle().id())
                .ok_or_else(|| MissingAssetError::new(neighbor.kind.handle().id()))?;
            let layer = edge_configs
                .get(neighbor.edge_config.id())
                .ok_or_else(|| MissingAssetError::new(neighbor.edge_config.id()))?
                .get_default()
                .base();
            return spawn_tile_sprite(
                &position,
                layer,
                &neighbor.spritesheet,
                z,
                commands,
                tile_kinds,
                edge_configs,
                grid_size,
                tile_grid,
            );
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
            Sprite {
                image: spritesheet.image().clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: spritesheet.layout()?.clone(),
                    index: *idx,
                }),
                ..Default::default()
            },
            None,
        ),
        GroundTileVisual::Animated(animation_asset) => (
            Sprite {
                image: spritesheet.image().clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: spritesheet.layout()?.clone(),
                    index: 0,
                }),
                ..Default::default()
            },
            Some(animation_asset.clone()),
        ),
        GroundTileVisual::Neighbor(_) => {
            panic!("GroundTileVisual cannot have neighbor sprite as default")
        }
    })
}

#[derive(Component)]
pub struct AnimationId(pub String);
