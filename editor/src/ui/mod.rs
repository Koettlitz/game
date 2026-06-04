use std::ops::{Deref, DerefMut};

use bevy::{prelude::*, window::PrimaryWindow};
use engine::{
    animation::Animated,
    asset::{AssetRef, AssetsExt},
    overworld::tile::{GridSize, TILE_SIZE},
    progress::ProgressState,
};
use input::InputPlugin;

pub use input::{PlaceObject, PlaceTile, RemoveTile};

use crate::{
    asset::{
        object::GameObjectKindAsset,
        tile::{TileEdgeConfig, TileKindAsset},
    },
    tile::edge::create_tile_sprite,
};
mod camera;
mod input;

const CURSOR_SPRITE_ALPHA: f32 = 0.5;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputPlugin)
            .add_systems(Update, on_cursor_changed.run_if(resource_changed::<Cursor>))
            .add_systems(Update, update_cursor_sprite)
            .add_observer(on_tile_grid_spawn)
            .add_systems(
                Update,
                draw_grid_bounds.run_if(in_state(ProgressState::Finished)),
            );
    }
}

#[derive(Resource, Default)]
enum Cursor {
    #[default]
    Default,
    GroundTile(AssetRef<TileKindAsset>),
    Object(AssetRef<GameObjectKindAsset>),
}

#[derive(Component)]
struct CursorSprite;

fn on_cursor_changed(
    mut commands: Commands,
    cursor_sprite: Query<Entity, With<CursorSprite>>,
    cursor: Res<Cursor>,
    grid_size: Single<&GridSize>,
    tile_kinds: Res<Assets<TileKindAsset>>,
    edge_configs: Res<Assets<TileEdgeConfig>>,
    object_kinds: Res<Assets<GameObjectKindAsset>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform)>,
) -> Result<()> {
    if let Ok(entity) = cursor_sprite.single() {
        commands.entity(entity).despawn();
    }
    match *cursor {
        Cursor::GroundTile(ref tile_kind_handle) => {
            let tile_kind = tile_kinds.require_handle(tile_kind_handle.handle())?;
            let edge_config = edge_configs.require_handle(&tile_kind.edge_config)?;
            let (mut sprite, animation_ref) =
                create_tile_sprite(&tile_kind.spritesheet, edge_config)?;
            sprite.color = sprite.color.with_alpha(CURSOR_SPRITE_ALPHA);
            let mut entity = commands.spawn((CursorSprite, sprite));
            if let Some(animation_ref) = animation_ref {
                entity.insert(Animated::by(animation_ref.handle().clone()));
            }
            if let Some(cursor_position) = window
                .cursor_position()
                .map(|pos| screen_to_world(pos, camera.0, camera.1))
            {
                let translation = grid_size
                    .snap_to_tile(cursor_position.truncate())
                    .extend(128.0);
                entity.insert(Transform::from_translation(translation));
            }
        }
        Cursor::Object(ref object_handle) => {
            let object_kind = object_kinds
                .get(object_handle.handle().id())
                .expect("cursor contained missing game object kind entity");
            let sprite = Sprite {
                image: object_kind.sprite_sheet.handle().clone(),
                ..Default::default()
            };
            let mut entity = commands.spawn((sprite, CursorSprite));
            if let Some(cursor_position) = window
                .cursor_position()
                .map(|pos| screen_to_world(pos, camera.0, camera.1))
            {
                let translation = grid_size
                    .snap_to_tile(cursor_position.truncate())
                    .extend(128.0);
                entity.insert(Transform::from_translation(translation));
            }
        }
        Cursor::Default => {}
    }
    Ok(())
}

fn screen_to_world(cursor_pos: Vec2, camera: &Camera, camera_transform: &GlobalTransform) -> Vec3 {
    match camera.viewport_to_world_2d(camera_transform, cursor_pos) {
        Ok(world_pos) => world_pos.extend(128.0),
        Err(e) => panic!("could not convert screen to world coords - {e}"),
    }
}

fn update_cursor_sprite(
    cursor_sprite: Single<&mut Transform, With<CursorSprite>>,
    grid_size: Single<&GridSize>,
    camera: Single<(&Camera, &GlobalTransform)>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };
    let translation = screen_to_world(cursor_position, camera.0, camera.1);
    let translation = grid_size
        .snap_to_tile(translation.truncate())
        .extend(translation.z);
    cursor_sprite.into_inner().translation = translation;
}

#[derive(Component, Default)]
pub struct ShowGridLines(bool);

impl Deref for ShowGridLines {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ShowGridLines {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ShowGridLines {
    pub fn toggle(&mut self) {
        **self = !**self;
    }
}

fn on_tile_grid_spawn(event: On<Insert, GridSize>, mut commands: Commands) {
    commands
        .entity(event.entity)
        .insert(ShowGridLines::default());
}

fn draw_grid_bounds(mut gizmos: Gizmos, grid: Query<(&GridSize, &ShowGridLines)>) {
    for (grid_size, show_grid_lines) in &grid {
        if **show_grid_lines {
            gizmos
                .grid_2d(
                    Isometry2d::IDENTITY,
                    grid_size.as_uvec2(),
                    TILE_SIZE.as_vec2(),
                    Color::BLACK,
                )
                .outer_edges();
        } else {
            gizmos.rect_2d(
                Isometry2d::IDENTITY,
                grid_size.as_vec2() * TILE_SIZE.as_vec2(),
                Color::BLACK,
            );
        }
    }
}
