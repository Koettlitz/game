use std::collections::HashMap;

use bevy::{input::mouse::MouseMotion, log, prelude::*, window::PrimaryWindow};
use bevy_elf::AssetRef;
use engine::{
    overworld::{
        character::Orientation,
        tile::{GridSize, TILE_SIZE},
    },
    progress::ProgressState,
};

use crate::{
    character::asset::{CharacterKindAsset, CharacterKindMap},
    io::export::ExportLozo,
    object::asset::{GameObjectKindAsset, GameObjectKindMap},
    tile::asset::{TileKindAsset, TileKindMap},
    ui::{
        ShowGridLines,
        camera::{CameraMovement, WorldCamera},
        screen_to_world,
    },
};

const CURSOR_SPRITE_ALPHA: f32 = 0.5;
const CURSOR_Z: f32 = 500.0;

pub struct InputPlugin;
impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlaceTile>()
            .add_message::<PlaceObject>()
            .add_message::<RemoveTile>()
            .add_message::<PlaceCharacter>()
            .init_resource::<TileKindKeyMap>()
            .init_resource::<GameObjectKindKeyMap>()
            .init_resource::<CharacterKindKeyMap>()
            .add_systems(Startup, init_cursor)
            .add_systems(
                OnEnter(ProgressState::Finished),
                (
                    init_tile_kind_keymap,
                    init_object_kind_keymap,
                    init_character_kind_keymap,
                ),
            )
            .add_systems(
                PreUpdate,
                (
                    move_camera,
                    (
                        (
                            (update_cursor_position, switch_cursor),
                            (place_tiles, place_object),
                        )
                            .chain(),
                        toggle_grid_lines,
                    )
                        .run_if(in_state(ProgressState::Finished)),
                ),
            )
            .add_systems(
                PostUpdate,
                (update_cursor_visuals, save_lozo).run_if(in_state(ProgressState::Finished)),
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
    pub world_position: Vec2,
    pub object_kind: AssetRef<GameObjectKindAsset>,
}

#[derive(Message)]
pub struct PlaceCharacter {
    pub world_position: Vec2,
    pub character_kind: AssetRef<CharacterKindAsset>,
    pub orientation: Orientation,
}

#[derive(Component, Default)]
#[require(Visibility, Transform)]
enum Cursor {
    #[default]
    Default,
    GroundTile(AssetRef<TileKindAsset>),
    Object(AssetRef<GameObjectKindAsset>),
    Character {
        asset: AssetRef<CharacterKindAsset>,
        orientation: Orientation,
    },
}

fn init_cursor(mut commands: Commands) {
    commands.spawn(Cursor::default());
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

fn init_character_kind_keymap(
    character_kind_map: Res<CharacterKindMap>,
    mut keymap: ResMut<CharacterKindKeyMap>,
) {
    for (id, handle) in character_kind_map.iter() {
        let keycode = match id.as_str() {
            "brendan" => KeyCode::KeyB,
            "pink_kid" => KeyCode::KeyP,
            other => {
                log::warn!("no hard coded key binding for character {other}");
                continue;
            }
        };
        keymap
            .0
            .insert(keycode, AssetRef::new(id.clone(), handle.clone()));
    }
}

#[derive(Resource, Default)]
struct CharacterKindKeyMap(HashMap<KeyCode, AssetRef<CharacterKindAsset>>);

#[derive(Resource, Default)]
struct TileKindKeyMap(HashMap<KeyCode, AssetRef<TileKindAsset>>);

#[derive(Resource, Default)]
struct GameObjectKindKeyMap(HashMap<KeyCode, AssetRef<GameObjectKindAsset>>);

fn update_cursor_position(
    mut cursor: Single<&mut Transform, With<Cursor>>,
    grid_size: Single<&GridSize>,
    camera: Single<(&Camera, &GlobalTransform), With<WorldCamera>>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    if let Some(cursor_position) = window.cursor_position() {
        let world_position = screen_to_world(cursor_position, camera.0, camera.1);
        cursor.translation = grid_size.snap_to_tile(world_position).extend(CURSOR_Z);
    }
}

fn switch_cursor(
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor: Single<&mut Cursor>,
    character_kind_keymap: Res<CharacterKindKeyMap>,
    tilekind_keymap: Res<TileKindKeyMap>,
    objectkind_keymap: Res<GameObjectKindKeyMap>,
) {
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        for key in keys.get_just_pressed() {
            if let Some(handle) = objectkind_keymap.0.get(key) {
                **cursor = Cursor::Object(handle.clone());
            }
        }
    } else if keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight) {
        for key in keys.get_just_pressed() {
            if let Some(handle) = character_kind_keymap.0.get(key) {
                **cursor = Cursor::Character {
                    asset: handle.clone(),
                    orientation: Orientation::default(),
                };
            }
        }
    } else {
        for key in keys.get_just_pressed() {
            if let Some(handle) = tilekind_keymap.0.get(key) {
                **cursor = Cursor::GroundTile(handle.clone());
            }
        }
    }
}

#[derive(Event)]
pub struct SpawnCursorSprite<A: Asset> {
    pub cursor: Entity,
    pub asset: Handle<A>,
    pub alpha: f32,
}

impl<A: Asset> SpawnCursorSprite<A> {
    fn new(cursor: Entity, asset: Handle<A>) -> Self {
        Self {
            cursor,
            asset,
            alpha: CURSOR_SPRITE_ALPHA,
        }
    }
}

fn update_cursor_visuals(
    cursor: Single<(Entity, &Cursor, Option<&Children>), Changed<Cursor>>,
    mut commands: Commands,
) {
    let (entity, cursor, children) = cursor.into_inner();

    if let Some(children) = children {
        for child in children {
            commands.entity(*child).despawn();
        }
    }

    match *cursor {
        Cursor::GroundTile(ref tile_kind) => {
            commands.trigger(SpawnCursorSprite::new(entity, tile_kind.handle().clone()));
        }
        Cursor::Object(ref object_handle) => {
            commands.trigger(SpawnCursorSprite::new(
                entity,
                object_handle.handle().clone(),
            ));
        }
        Cursor::Character { ref asset, .. } => {
            commands.trigger(SpawnCursorSprite::new(entity, asset.handle().clone()));
        }
        Cursor::Default => {}
    }
}

fn place_tiles(
    mut mouse_motion: MessageReader<MouseMotion>,
    camera: Single<(&Camera, &GlobalTransform), With<WorldCamera>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    cursor: Single<&Cursor>,
    mut place_tile_writer: MessageWriter<PlaceTile>,
    mut remove_tile_writer: MessageWriter<RemoveTile>,
) {
    let left_pressed = mouse_btn.pressed(MouseButton::Left);
    let right_pressed = mouse_btn.pressed(MouseButton::Right);
    if !left_pressed && !right_pressed {
        return;
    }
    let Cursor::GroundTile(tile_kind) = *cursor else {
        return;
    };
    // TODO: use cursor Transform component instead
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };
    if let Some(mouse_motion) = mouse_motion.read().next() {
        let delta = mouse_motion.delta * window.scale_factor();
        let mut starting_pos = cursor_position - delta;
        let tile_step_size = TILE_SIZE as f32;
        let tile_step = mouse_motion
            .delta
            .clamp_length(tile_step_size, tile_step_size);
        let step_count = (delta.length() / tile_step.length()).ceil() as usize;
        for _ in 0..step_count {
            let world_position = screen_to_world(starting_pos, camera.0, camera.1);
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
        let world_position = screen_to_world(cursor_position, camera.0, camera.1);

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
    cursor: Single<&Cursor>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    camera: Single<(&Camera, &GlobalTransform), With<WorldCamera>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut place_object_message_writer: MessageWriter<PlaceObject>,
    mut place_character_message_writer: MessageWriter<PlaceCharacter>,
) {
    if !mouse_btn.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };
    let world_position = screen_to_world(cursor_position, camera.0, camera.1);
    match *cursor {
        Cursor::Object(object_kind) => {
            place_object_message_writer.write(PlaceObject {
                world_position,
                object_kind: object_kind.clone(),
            });
        }
        Cursor::Character { asset, orientation } => {
            place_character_message_writer.write(PlaceCharacter {
                character_kind: asset.clone(),
                world_position,
                orientation: *orientation,
            });
        }
        _ => {}
    }
}

fn move_camera(
    keys: Res<ButtonInput<KeyCode>>,
    camera: Single<&mut CameraMovement, (With<Camera2d>, With<WorldCamera>)>,
) {
    let mut movement = camera.into_inner();
    movement.up = keys.pressed(KeyCode::ArrowUp);
    movement.left = keys.pressed(KeyCode::ArrowLeft);
    movement.right = keys.pressed(KeyCode::ArrowRight);
    movement.down = keys.pressed(KeyCode::ArrowDown);
}

fn toggle_grid_lines(keys: Res<ButtonInput<KeyCode>>, mut grid_lines: Single<&mut ShowGridLines>) {
    if (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && keys.just_pressed(KeyCode::KeyG)
    {
        grid_lines.toggle();
    }
}

fn save_lozo(keys: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && keys.just_pressed(KeyCode::KeyS)
    {
        commands.trigger(ExportLozo);
    }
}
