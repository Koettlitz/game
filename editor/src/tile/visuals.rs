use std::{collections::HashSet, ops::Deref};

use bevy::prelude::*;
use engine::{
    animation::Animated,
    asset::{
        AssetRef, MissingAssetError, animations::sprite::SpriteAnimationAsset,
        overworld::tile::TileSpriteSheet,
    },
    overworld::tile::{GridPosition, GridSize, TileGrid},
    progress::ProgressState,
};

use super::spawn_ground_tile_grid;
use crate::{
    asset::tile::{GroundTileVisual, GroundTileVisuals, TileKindAsset},
    tile::{GroundTileGrid, GroundTilesChanged},
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

#[derive(Resource)]
pub struct TileSpriteGrid(TileGrid<Vec<Entity>>);
impl Deref for TileSpriteGrid {
    type Target = TileGrid<Vec<Entity>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Event)]
struct UpdateTileSprites(GridPosition);

fn init_sprite_grid(mut commands: Commands, grid_size: Res<GridSize>) {
    commands.insert_resource(TileSpriteGrid(TileGrid::with_size(&grid_size)));
    for pos in grid_size.iter() {
        commands.trigger(UpdateTileSprites(pos));
    }
}

fn on_ground_tile_changed(
    event: On<GroundTilesChanged>,
    mut commands: Commands,
    grid_size: Res<GridSize>,
) {
    let mut sprites_to_update: HashSet<GridPosition> = HashSet::new();
    for position in &event.0 {
        for neighbor in position
            .adjacent(&grid_size)
            .iter_inclusive()
            .into_iter()
            .filter_map(|p| p)
        {
            sprites_to_update.insert(neighbor);
        }
    }
    for sprite_to_update in sprites_to_update {
        commands.trigger(UpdateTileSprites(sprite_to_update));
    }
}

fn update_sprites(
    event: On<UpdateTileSprites>,
    mut commands: Commands,
    ground_tile_grid: Res<GroundTileGrid>,
    grid_size: Res<GridSize>,
    tile_kinds: Res<Assets<TileKindAsset>>,
    mut sprites_grid: ResMut<TileSpriteGrid>,
) -> Result<()> {
    let position = event.0;
    let surroundings = ground_tile_grid.0.view_of(position.adjacent(&grid_size));
    let tile_kind_asset = tile_kinds
        .get(surroundings.center().handle().id())
        .expect("missing visual for ground tile");
    let layers = &tile_kind_asset
        .visuals
        .config
        .iter()
        .find(|(req, _)| req.matches(&surroundings))
        .expect("no adjacent requirement matched the current surroundings")
        .1;
    let old_sprites = &sprites_grid.0[&position.as_index(&grid_size)];
    for old_sprite in old_sprites {
        commands.entity(*old_sprite).despawn();
    }
    let mut new_sprites = Vec::new();
    for (z, layer) in layers.iter() {
        let sprite_entity = spawn_tile_sprite(
            &position,
            layer,
            &tile_kind_asset.visuals.spritesheet,
            z,
            &mut commands,
            &tile_kinds,
            &grid_size,
            &ground_tile_grid.0,
        )?;
        if let Some(entity) = sprite_entity {
            new_sprites.push(entity);
        }
    }
    sprites_grid.0[&position.as_index(&grid_size)] = new_sprites;
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
    spritesheet: &TileSpriteSheet,
    z: f32,
    commands: &mut Commands,
    tile_kinds: &Assets<TileKindAsset>,
    grid_size: &GridSize,
    tile_grid: &TileGrid<AssetRef<TileKindAsset>>,
) -> Result<Option<Entity>> {
    let transform = Transform::from_translation(grid_size.to_world_pos(position).extend(z));
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
                    Animated::by(animation_asset.clone()),
                    transform,
                ))
                .id();
            Ok(Some(entity))
        }
        GroundTileVisual::Neighbor(neighbor) => {
            let Some(neighbor_position) = position.neighbor(&neighbor, &grid_size) else {
                return Ok(None);
            };
            let neighbor = &tile_grid[&neighbor_position.as_index(&grid_size)];
            let neighbor = tile_kinds
                .get(neighbor.handle().id())
                .ok_or_else(|| MissingAssetError::new(neighbor.handle().id()))?;
            return spawn_tile_sprite(
                &position,
                &neighbor.visuals.default_config().base(),
                &neighbor.visuals.spritesheet,
                z,
                commands,
                tile_kinds,
                grid_size,
                tile_grid,
            );
        }
    }
}

pub fn create_tile_sprite(
    visuals: &GroundTileVisuals,
) -> Result<(Sprite, Option<AssetRef<SpriteAnimationAsset>>)> {
    let visual = visuals.default_config();
    Ok(match &visual.base() {
        GroundTileVisual::Static(idx) => (
            Sprite {
                image: visuals.spritesheet.image().clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: visuals.spritesheet.layout()?.clone(),
                    index: *idx,
                }),
                ..Default::default()
            },
            None,
        ),
        GroundTileVisual::Animated(animation_asset) => (
            Sprite {
                image: visuals.spritesheet.image().clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: visuals.spritesheet.layout()?.clone(),
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
