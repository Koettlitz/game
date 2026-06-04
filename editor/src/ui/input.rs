use std::collections::HashMap;

use bevy::{input::mouse::MouseMotion, prelude::*, window::PrimaryWindow};
use engine::{
    asset::AssetRef,
    overworld::tile::{GridSize, TILE_SIZE},
    progress::ProgressState,
};

use crate::{
    asset::{
        object::{GameObjectKindAsset, GameObjectKindMap},
        tile::{TileKindAsset, TileKindMap},
    },
    io::export::ExportLozo,
    ui::{
        Cursor, ShowGridLines,
        camera::{CameraMovement, CameraPlugin},
        screen_to_world,
    },
};

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
            .add_systems(
                OnEnter(ProgressState::Finished),
                (init_tile_kind_keymap, init_object_kind_keymap),
            )
            .add_systems(
                PreUpdate,
                (
                    move_camera,
                    (
                        (switch_cursor, (place_tiles, place_object)).chain(),
                        toggle_grid_lines,
                    )
                        .run_if(in_state(ProgressState::Finished)),
                ),
            )
            .add_systems(
                PostUpdate,
                save_lozo.run_if(in_state(ProgressState::Finished)),
            );
    }
}

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
                *cursor = Cursor::GroundTile(handle.clone());
            }
        }
    }
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
    let left_pressed = mouse_btn.pressed(MouseButton::Left);
    let right_pressed = mouse_btn.pressed(MouseButton::Right);
    if !left_pressed && !right_pressed {
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
            let world_position = screen_to_world(starting_pos, camera.0, camera.1).truncate();
            if left_pressed {
                place_tile_writer.write(PlaceTile {
                    world_position,
                    tile_kind: tile_kind.clone(),
                });
            } else if right_pressed {
                remove_tile_writer.write(RemoveTile { world_position });
            }
            starting_pos += tile_step;
        }
    } else {
        let world_position = screen_to_world(cursor_position, camera.0, camera.1).truncate();

        if mouse_btn.just_pressed(MouseButton::Left) {
            place_tile_writer.write(PlaceTile {
                world_position,
                tile_kind: tile_kind.clone(),
            });
        } else if mouse_btn.just_pressed(MouseButton::Right) {
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
    let Some(grid_position) =
        grid_size.world_to_grid(screen_to_world(cursor_position, camera.0, camera.1).truncate())
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
    movement.up = keys.pressed(KeyCode::ArrowUp);
    movement.left = keys.pressed(KeyCode::ArrowLeft);
    movement.right = keys.pressed(KeyCode::ArrowRight);
    movement.down = keys.pressed(KeyCode::ArrowDown);
}

fn toggle_grid_lines(keys: Res<ButtonInput<KeyCode>>, mut grid_lines: Single<&mut ShowGridLines>) {
    if keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight) {
        if keys.just_pressed(KeyCode::KeyG) {
            grid_lines.toggle();
        }
    }
}

fn save_lozo(keys: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && keys.just_pressed(KeyCode::KeyS)
    {
        commands.trigger(ExportLozo);
    }
}
