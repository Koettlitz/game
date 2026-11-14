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
    lozo::{ExportLozo, SaveLozo},
    object::asset::{GameObjectKindAsset, GameObjectKindMap},
    tile::asset::{TileKindAsset, TileKindMap},
    ui::{
        CurrentLozo, ShowGridLines,
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
                            (
                                update_cursor_position.run_if(resource_exists::<CurrentLozo>),
                                switch_cursor,
                            ),
                            (place_tiles, place_object).run_if(resource_exists::<CurrentLozo>),
                        )
                            .chain(),
                        toggle_grid_lines,
                    )
                        .run_if(in_state(ProgressState::Finished)),
                ),
            )
            .add_systems(
                PostUpdate,
                (
                    update_cursor_visuals,
                    trigger_commands.run_if(resource_exists::<CurrentLozo>),
                )
                    .run_if(in_state(ProgressState::Finished)),
            );
    }
}

#[derive(Message)]
pub struct PlaceTile {
    pub lozo_entity: Entity,
    pub world_position: Vec2,
    pub tile_kind: AssetRef<TileKindAsset>,
}

#[derive(Message)]
pub struct RemoveTile {
    pub lozo_entity: Entity,
    pub world_position: Vec2,
}

#[derive(Message)]
pub struct PlaceObject {
    pub lozo_entity: Entity,
    pub world_position: Vec2,
    pub object_kind: AssetRef<GameObjectKindAsset>,
}

#[derive(Message)]
pub struct PlaceCharacter {
    pub lozo_entity: Entity,
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
    mut cursor: Query<&mut Transform, With<Cursor>>,
    current_lozo: Res<CurrentLozo>,
    grid_size: Query<&GridSize>,
    camera: Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    window: Query<&Window, With<PrimaryWindow>>,
) -> Result {
    if let Some(cursor_position) = window.single()?.cursor_position() {
        let (camera, camera_transform) = camera.single()?;
        let world_position = screen_to_world(cursor_position, camera, camera_transform);
        cursor.single_mut()?.translation = grid_size
            .get(current_lozo.entity())?
            .snap_to_tile(world_position)
            .extend(CURSOR_Z);
    }

    Ok(())
}

fn switch_cursor(
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor: Query<&mut Cursor>,
    character_kind_keymap: Res<CharacterKindKeyMap>,
    tilekind_keymap: Res<TileKindKeyMap>,
    objectkind_keymap: Res<GameObjectKindKeyMap>,
) -> Result {
    if keys.just_pressed(KeyCode::Escape) {
        *cursor.single_mut()? = Cursor::Default;
    }

    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        for key in keys.get_just_pressed() {
            if let Some(handle) = objectkind_keymap.0.get(key) {
                *cursor.single_mut()? = Cursor::Object(handle.clone());
            }
        }
    } else if keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight) {
        for key in keys.get_just_pressed() {
            if let Some(handle) = character_kind_keymap.0.get(key) {
                *cursor.single_mut()? = Cursor::Character {
                    asset: handle.clone(),
                    orientation: Orientation::default(),
                };
            }
        }
    } else {
        for key in keys.get_just_pressed() {
            if let Some(handle) = tilekind_keymap.0.get(key) {
                *cursor.single_mut()? = Cursor::GroundTile(handle.clone());
            }
        }
    }

    Ok(())
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
    cursor: Query<(Entity, &Cursor, Option<&Children>), Changed<Cursor>>,
    mut commands: Commands,
) -> Result {
    for (entity, cursor, children) in cursor {
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

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn place_tiles(
    mut mouse_motion: MessageReader<MouseMotion>,
    camera: Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    cursor: Query<&Cursor>,
    current_lozo: Res<CurrentLozo>,
    mut place_tile_writer: MessageWriter<PlaceTile>,
    mut remove_tile_writer: MessageWriter<RemoveTile>,
) -> Result {
    let left_pressed = mouse_btn.pressed(MouseButton::Left);
    let right_pressed = mouse_btn.pressed(MouseButton::Right);
    if !left_pressed && !right_pressed {
        return Ok(());
    }

    let Cursor::GroundTile(tile_kind) = cursor.single()? else {
        return Ok(());
    };

    // TODO: use cursor Transform component instead
    let window = window.single()?;
    let Some(cursor_position) = window.cursor_position() else {
        return Ok(());
    };

    let (camera, camera_transform) = camera.single()?;

    if let Some(mouse_motion) = mouse_motion.read().next() {
        let delta = mouse_motion.delta * window.scale_factor();
        let mut starting_pos = cursor_position - delta;
        let tile_step_size = TILE_SIZE as f32;
        let tile_step = mouse_motion
            .delta
            .clamp_length(tile_step_size, tile_step_size);
        let step_count = (delta.length() / tile_step.length()).ceil() as usize;
        for _ in 0..step_count {
            let world_position = screen_to_world(starting_pos, camera, camera_transform);
            if left_pressed {
                place_tile_writer.write(PlaceTile {
                    lozo_entity: current_lozo.entity(),
                    world_position,
                    tile_kind: tile_kind.clone(),
                });
            } else if right_pressed {
                remove_tile_writer.write(RemoveTile {
                    lozo_entity: current_lozo.entity(),
                    world_position,
                });
            }
            starting_pos += tile_step;
        }
    } else {
        let world_position = screen_to_world(cursor_position, camera, camera_transform);

        if left_pressed {
            place_tile_writer.write(PlaceTile {
                lozo_entity: current_lozo.entity(),
                world_position,
                tile_kind: tile_kind.clone(),
            });
        } else if right_pressed {
            remove_tile_writer.write(RemoveTile {
                lozo_entity: current_lozo.entity(),
                world_position,
            });
        }
    }

    Ok(())
}

fn place_object(
    cursor: Query<&Cursor>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    camera: Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    window: Query<&Window, With<PrimaryWindow>>,
    current_lozo: Res<CurrentLozo>,
    mut place_object_message_writer: MessageWriter<PlaceObject>,
    mut place_character_message_writer: MessageWriter<PlaceCharacter>,
) -> Result {
    if !mouse_btn.just_pressed(MouseButton::Left) {
        return Ok(());
    }
    let Some(cursor_position) = window.single()?.cursor_position() else {
        return Ok(());
    };

    let (camera, camera_transform) = camera.single()?;
    let world_position = screen_to_world(cursor_position, camera, camera_transform);

    match cursor.single()? {
        Cursor::Object(object_kind) => {
            place_object_message_writer.write(PlaceObject {
                lozo_entity: current_lozo.entity(),
                world_position,
                object_kind: object_kind.clone(),
            });
        }
        Cursor::Character { asset, orientation } => {
            place_character_message_writer.write(PlaceCharacter {
                lozo_entity: current_lozo.entity(),
                character_kind: asset.clone(),
                world_position,
                orientation: *orientation,
            });
        }
        _ => {}
    }

    Ok(())
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

fn trigger_commands(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    current_lozo: Res<CurrentLozo>,
) {
    if keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight) {
        if keys.just_pressed(KeyCode::KeyS) {
            commands.trigger(SaveLozo {
                lozo_entity: current_lozo.entity(),
            });
        }

        if keys.just_pressed(KeyCode::KeyE) {
            commands.trigger(ExportLozo {
                lozo_entity: current_lozo.entity(),
            });
        }
    }
}
