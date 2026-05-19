use std::collections::HashMap;

use bevy::{input::mouse::MouseMotion, prelude::*, window::PrimaryWindow};
use engine::{
    animation::Animated,
    asset::{AssetRef, AssetsExt},
    overworld::tile::{GridSize, TILE_SIZE},
    progress::ProgressState,
};

use crate::{
    asset::{
        object::{GameObjectKindAsset, GameObjectKindMap},
        tile::{TileEdgeConfig, TileKindAsset, TileKindMap},
    },
    io::export::ExportLozo,
    tile::edge::create_tile_sprite,
    ui::camera::{CameraMovement, CameraPlugin},
};

const CURSOR_SPRITE_ALPHA: f32 = 0.5;

pub struct InputPlugin;
impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CameraPlugin)
            .add_message::<PlaceTile>()
            .add_message::<PlaceObject>()
            .add_message::<RemoveTile>()
            .init_resource::<Cursor>()
            .init_resource::<TileKindKeyMap>()
            .init_resource::<GameObjectKindKeyMap>()
            .add_systems(OnEnter(ProgressState::Finished), init_tile_kind_keymap)
            .add_systems(OnEnter(ProgressState::Finished), init_object_kind_keymap)
            .add_systems(
                PreUpdate,
                (place_tiles, place_object, switch_cursor)
                    .run_if(in_state(ProgressState::Finished)),
            )
            .add_systems(Update, on_cursor_changed.run_if(resource_changed::<Cursor>))
            .add_systems(Update, update_cursor_sprite)
            .add_systems(Update, move_camera)
            .add_systems(
                PostUpdate,
                save_lozo.run_if(in_state(ProgressState::Finished)),
            );
    }
}

#[derive(Resource, Default)]
pub enum Cursor {
    #[default]
    Default,
    GroundTile(Option<AssetRef<TileKindAsset>>),
    Object(AssetRef<GameObjectKindAsset>),
}

#[derive(Component)]
struct CursorSprite;

#[derive(Message)]
pub struct PlaceTile {
    pub world_position: Vec2,
    pub tile_kind: AssetRef<TileKindAsset>,
}

#[derive(Message)]
pub struct RemoveTile {
    pub world_position: Vec2,
}

#[derive(Message)]
pub struct PlaceObject {
    pub pos: UVec2,
    pub object_kind: AssetRef<GameObjectKindAsset>,
}

fn init_tile_kind_keymap(tile_kind_map: Res<TileKindMap>, mut keymap: ResMut<TileKindKeyMap>) {
    for (id, handle) in tile_kind_map.iter() {
        let keycode = match id.as_str() {
            "grass" => KeyCode::KeyG,
            "water_calm" => KeyCode::KeyC,
            "water_wild" => KeyCode::KeyW,
            "sand" => KeyCode::KeyS,
            _ => {
                warn!("no hard coded key for tile kind {id:?}");
                return;
            }
        };

        keymap
            .0
            .insert(keycode, AssetRef::new(id.clone(), handle.clone()));
    }
}

fn init_object_kind_keymap(
    object_kind_map: Res<GameObjectKindMap>,
    mut keymap: ResMut<GameObjectKindKeyMap>,
) {
    for (id, handle) in object_kind_map.iter() {
        let keycode = match id.as_str() {
            "pokecenter" => KeyCode::KeyC,
            _ => {
                warn!("no hard coded key for game object kind {id:?}");
                continue;
            }
        };
        keymap
            .0
            .insert(keycode, AssetRef::new(id.clone(), handle.clone()));
    }
}

#[derive(Resource, Default)]
struct TileKindKeyMap(HashMap<KeyCode, AssetRef<TileKindAsset>>);
#[derive(Resource, Default)]
struct GameObjectKindKeyMap(HashMap<KeyCode, AssetRef<GameObjectKindAsset>>);

fn switch_cursor(
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor: ResMut<Cursor>,
    tilekind_keymap: Res<TileKindKeyMap>,
    objectkind_keymap: Res<GameObjectKindKeyMap>,
) {
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        for key in keys.get_just_pressed() {
            if let Some(handle) = objectkind_keymap.0.get(key) {
                *cursor = Cursor::Object(handle.clone());
            }
        }
    } else {
        for key in keys.get_just_pressed() {
            if let Some(handle) = tilekind_keymap.0.get(key) {
                *cursor = Cursor::GroundTile(Some(handle.clone()));
            } else if *key == KeyCode::KeyX {
                *cursor = Cursor::GroundTile(None);
            }
        }
    }
}

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
    match &*cursor {
        Cursor::GroundTile(tile_kind_handle) => {
            let (mut sprite, animation_ref) = match tile_kind_handle {
                Some(tile_kind_handle) => {
                    let tile_kind = tile_kinds.require_handle(tile_kind_handle.handle())?;
                    let edge_config = edge_configs.require_handle(&tile_kind.edge_config)?;
                    create_tile_sprite(&tile_kind.spritesheet, edge_config)?
                }
                None => (
                    Sprite {
                        color: Color::BLACK,
                        custom_size: Some(TILE_SIZE.as_vec2()),
                        ..Default::default()
                    },
                    None,
                ),
            };
            sprite.color = sprite.color.with_alpha(CURSOR_SPRITE_ALPHA);
            let mut entity = commands.spawn((CursorSprite, sprite));
            if let Some(animation_ref) = animation_ref {
                entity.insert(Animated::by(animation_ref.handle().clone()));
            }
            if let Some(cursor_position) = window
                .cursor_position()
                .map(|pos| cursor_pos_to_world_pos(pos, camera.0, camera.1))
            {
                let translation = grid_size
                    .snap_to_tile(cursor_position.truncate())
                    .extend(128.0);
                entity.insert(Transform::from_translation(translation));
            }
        }
        Cursor::Object(object_handle) => {
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
                .map(|pos| cursor_pos_to_world_pos(pos, camera.0, camera.1))
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

fn update_cursor_sprite(
    mut query: Query<&mut Transform, With<CursorSprite>>,
    grid_size: Single<&GridSize>,
    camera: Single<(&Camera, &GlobalTransform)>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let Ok(mut sprite_pos) = query.single_mut() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };
    let translation = cursor_pos_to_world_pos(cursor_position, camera.0, camera.1);
    let translation = grid_size
        .snap_to_tile(translation.truncate())
        .extend(translation.z);
    sprite_pos.translation = translation;
}

fn place_tiles(
    mut mouse_motion: MessageReader<MouseMotion>,
    camera: Single<(&Camera, &GlobalTransform)>,
    window: Single<&Window, With<PrimaryWindow>>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    cursor: Res<Cursor>,
    mut place_tile_writer: MessageWriter<PlaceTile>,
    mut remove_tile_writer: MessageWriter<RemoveTile>,
) {
    if !mouse_btn.pressed(MouseButton::Left) {
        return;
    }
    let Cursor::GroundTile(ref tile_kind) = *cursor else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };
    if let Some(mouse_motion) = mouse_motion.read().next() {
        let delta = mouse_motion.delta * window.scale_factor();
        let mut starting_pos = cursor_position - delta;
        let tile_step_size = (TILE_SIZE.x as f32 + TILE_SIZE.y as f32) / 2.0;
        let tile_step = mouse_motion
            .delta
            .clamp_length(tile_step_size, tile_step_size);
        let step_count = (delta.length() / tile_step.length()).ceil() as usize;
        for _ in 0..step_count {
            let world_position =
                cursor_pos_to_world_pos(starting_pos, camera.0, camera.1).truncate();
            if let Some(tile_kind) = tile_kind.as_ref() {
                place_tile_writer.write(PlaceTile {
                    world_position,
                    tile_kind: tile_kind.clone(),
                });
            } else {
                remove_tile_writer.write(RemoveTile { world_position });
            }
            starting_pos += tile_step;
        }
    } else if mouse_btn.just_pressed(MouseButton::Left) {
        let world_position =
            cursor_pos_to_world_pos(cursor_position, camera.0, camera.1).truncate();

        if let Some(tile_kind) = tile_kind.as_ref() {
            place_tile_writer.write(PlaceTile {
                world_position,
                tile_kind: tile_kind.clone(),
            });
        } else {
            remove_tile_writer.write(RemoveTile { world_position });
        }
    }
}

fn place_object(
    cursor: Res<Cursor>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    window: Single<&Window, With<PrimaryWindow>>,
    grid_size: Single<&GridSize>,
    mut message_writer: MessageWriter<PlaceObject>,
) {
    if !mouse_btn.pressed(MouseButton::Left) {
        return;
    }
    let Cursor::Object(ref object_kind_handle) = *cursor else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };
    let Some(grid_position) = grid_size
        .world_to_grid(cursor_pos_to_world_pos(cursor_position, camera.0, camera.1).truncate())
    else {
        return;
    };
    message_writer.write(PlaceObject {
        pos: *grid_position,
        object_kind: object_kind_handle.clone(),
    });
}

fn move_camera(
    keys: Res<ButtonInput<KeyCode>>,
    camera: Single<&mut CameraMovement, With<Camera2d>>,
) {
    let mut movement = camera.into_inner();
    if keys.just_pressed(KeyCode::ArrowUp) {
        movement.up = true;
    } else if keys.just_released(KeyCode::ArrowUp) {
        movement.up = false;
    }
    if keys.just_pressed(KeyCode::ArrowLeft) {
        movement.left = true;
    } else if keys.just_released(KeyCode::ArrowLeft) {
        movement.left = false;
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        movement.right = true;
    } else if keys.just_released(KeyCode::ArrowRight) {
        movement.right = false;
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        movement.down = true;
    } else if keys.just_released(KeyCode::ArrowDown) {
        movement.down = false;
    }
}

fn save_lozo(keys: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && keys.just_pressed(KeyCode::KeyS)
    {
        commands.trigger(ExportLozo);
    }
}

fn cursor_pos_to_world_pos(
    cursor_pos: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
) -> Vec3 {
    match camera.viewport_to_world_2d(camera_transform, cursor_pos) {
        Ok(world_pos) => world_pos.extend(128.0),
        Err(e) => panic!("could not get world coords from mouse coords - {e}"),
    }
}
