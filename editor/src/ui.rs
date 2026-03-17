use bevy::{
    input::mouse::MouseMotion, platform::collections::HashMap, prelude::*, window::PrimaryWindow,
};
use engine::{
    Id,
    animation::{Animated, SpriteAnimation},
    assets::{SpriteSheet, tile::TILE_SIZE},
    overworld::tile::{GridPosition, GridSize},
    progress::ProgressState,
};

use crate::{
    object::GameObjectKind,
    tile::{
        GroundTileKind,
        visuals::{GroundTileVisuals, create_tile_sprite},
    },
};

const CURSOR_SPRITE_ALPHA: f32 = 0.5;

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlaceTile>()
            .add_message::<PlaceObject>()
            .init_resource::<Cursor>()
            .init_resource::<TileKindKeyMap>()
            .init_resource::<GameObjectKindKeyMap>()
            .add_systems(Update, init_tile_kind_keymap)
            .add_systems(Update, init_object_kind_keymap)
            .add_systems(
                PreUpdate,
                (
                    write_place_tile_messages,
                    write_place_object_message,
                    switch_cursor,
                )
                    .run_if(in_state(ProgressState::Finished)),
            )
            .add_systems(Update, on_cursor_changed.run_if(resource_changed::<Cursor>))
            .add_systems(Update, update_cursor_sprite);
    }
}

fn init_tile_kind_keymap(
    tile_kinds: Query<(Entity, &Id), Added<GroundTileKind>>,
    mut keymap: ResMut<TileKindKeyMap>,
) {
    for (entity, Id(id)) in tile_kinds {
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

        keymap.0.insert(keycode, entity);
    }
}

fn init_object_kind_keymap(
    object_kinds: Query<(Entity, &Id), Added<GameObjectKind>>,
    mut keymap: ResMut<GameObjectKindKeyMap>,
) {
    for (entity, Id(id)) in object_kinds.iter() {
        let keycode = match id.as_str() {
            "pokecenter" => KeyCode::KeyC,
            _ => {
                warn!("no hard coded key for game object kind {id:?}");
                continue;
            }
        };
        keymap.0.insert(keycode, entity);
    }
}

#[derive(Resource, Default)]
struct TileKindKeyMap(HashMap<KeyCode, Entity>);
#[derive(Resource, Default)]
struct GameObjectKindKeyMap(HashMap<KeyCode, Entity>);

fn switch_cursor(
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor: ResMut<Cursor>,
    tilekind_keymap: Res<TileKindKeyMap>,
    objectkind_keymap: Res<GameObjectKindKeyMap>,
) {
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        for key in keys.get_just_pressed() {
            if let Some(entity) = objectkind_keymap.0.get(key) {
                *cursor = Cursor::Object(*entity);
            }
        }
    } else {
        for key in keys.get_just_pressed() {
            if let Some(entity) = tilekind_keymap.0.get(key) {
                *cursor = Cursor::GroundTile(*entity);
            }
        }
    }
}

fn on_cursor_changed(
    mut commands: Commands,
    cursor_sprite: Query<Entity, With<CursorSprite>>,
    cursor: Res<Cursor>,
    grid_size: Res<GridSize>,
    tile_kind_query: Query<(&GroundTileVisuals, &SpriteSheet), With<GroundTileKind>>,
    object_kind_query: Query<(&GameObjectKind, &SpriteSheet)>,
    animations: Query<&SpriteAnimation>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform)>,
) {
    if let Ok(entity) = cursor_sprite.single() {
        commands.entity(entity).despawn();
    }
    match *cursor {
        Cursor::GroundTile(ground_tile_entity) => {
            let (visuals, sprite_sheet) = tile_kind_query
                .get(ground_tile_entity)
                .expect("cursor contained missing ground tile kind entity");
            let (mut sprite, animation_entity) =
                create_tile_sprite(visuals, sprite_sheet, animations);
            sprite.color = sprite.color.with_alpha(CURSOR_SPRITE_ALPHA);
            let mut entity = commands.spawn((CursorSprite, sprite));
            if let Some(animation_entity) = animation_entity {
                entity.insert(Animated::by(animation_entity));
            }
            if let Some(cursor_position) = window
                .cursor_position()
                .map(|pos| cursor_pos_to_world_pos(pos, camera.0, camera.1))
            {
                let translation = grid_size
                    .center_on_tile(cursor_position.truncate())
                    .extend(128.0);
                entity.insert(Transform::from_translation(translation));
            }
        }
        Cursor::Object(entity) => {
            let (_object_kind, sprite_sheet) = object_kind_query
                .get(entity)
                .expect("cursor contained missing game object kind entity");
            let sprite = Sprite {
                image: sprite_sheet.image.clone(),
                ..Default::default()
            };
            let mut entity = commands.spawn((sprite, CursorSprite));
            if let Some(cursor_position) = window
                .cursor_position()
                .map(|pos| cursor_pos_to_world_pos(pos, camera.0, camera.1))
            {
                let translation = grid_size
                    .center_on_tile(cursor_position.truncate())
                    .extend(128.0);
                entity.insert(Transform::from_translation(translation));
            }
        }
        Cursor::Default => {}
    }
}

fn update_cursor_sprite(
    mut query: Query<&mut Transform, With<CursorSprite>>,
    grid_size: Res<GridSize>,
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
        .center_on_tile(translation.truncate())
        .extend(translation.z);
    sprite_pos.translation = translation;
}

fn write_place_tile_messages(
    mut mouse_motion: MessageReader<MouseMotion>,
    camera: Single<(&Camera, &GlobalTransform)>,
    window: Single<&Window, With<PrimaryWindow>>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    cursor: Res<Cursor>,
    grid_size: Res<GridSize>,
    mut message_writer: MessageWriter<PlaceTile>,
) {
    if !mouse_btn.pressed(MouseButton::Left) {
        return;
    }
    let Cursor::GroundTile(tile_kind) = *cursor else {
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
            let world_position = cursor_pos_to_world_pos(starting_pos, camera.0, camera.1);
            if let Some(pos) = grid_size.to_grid_pos(world_position.truncate()) {
                message_writer.write(PlaceTile { pos, tile_kind });
            }
            starting_pos += tile_step;
        }
    } else if mouse_btn.just_pressed(MouseButton::Left) {
        let world_position = cursor_pos_to_world_pos(cursor_position, camera.0, camera.1);
        if let Some(pos) = grid_size.to_grid_pos(world_position.truncate()) {
            message_writer.write(PlaceTile { pos, tile_kind });
        }
    }
}

fn write_place_object_message(
    cursor: Res<Cursor>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    window: Single<&Window, With<PrimaryWindow>>,
    grid_size: Res<GridSize>,
    mut message_writer: MessageWriter<PlaceObject>,
) {
    if !mouse_btn.pressed(MouseButton::Left) {
        return;
    }
    let Cursor::Object(object_kind_entity) = *cursor else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };
    let Some(grid_position) = grid_size
        .to_grid_pos(cursor_pos_to_world_pos(cursor_position, camera.0, camera.1).truncate())
    else {
        return;
    };
    message_writer.write(PlaceObject {
        pos: grid_position,
        object_kind: object_kind_entity,
    });
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

#[derive(Resource, Default)]
pub enum Cursor {
    #[default]
    Default,
    GroundTile(Entity),
    Object(Entity),
}

#[derive(Component)]
struct CursorSprite;

#[derive(Message)]
pub struct PlaceTile {
    pub pos: GridPosition,
    pub tile_kind: Entity,
}

#[derive(Message)]
pub struct PlaceObject {
    pub pos: GridPosition,
    pub object_kind: Entity,
}
