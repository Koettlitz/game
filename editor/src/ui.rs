use bevy::{
    input::mouse::MouseMotion, platform::collections::HashMap, prelude::*, window::PrimaryWindow,
};
use engine::{
    Id,
    assets::tile::TILE_SIZE,
    overworld::tile::{GridPosition, GridSize},
    progress::ProgressState,
};

use crate::tile::GroundTileKind;

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlaceTile>()
            .init_resource::<Cursor>()
            .init_resource::<TileKindKeyMap>()
            .add_observer(init_tile_kind_keymap)
            .add_systems(
                PreUpdate,
                (write_place_tile_messages, switch_cursor)
                    .run_if(in_state(ProgressState::Finished)),
            );
    }
}

fn init_tile_kind_keymap(
    event: On<Add, (GroundTileKind, Id)>,
    tile_kinds: Query<&Id, With<GroundTileKind>>,
    mut keymap: ResMut<TileKindKeyMap>,
) {
    // TODO remove hardcoded shit
    let id = tile_kinds
        .get(event.entity)
        .expect("missing id for added groundtilekind");
    let keycode = match id.0.as_str() {
        "grass" => KeyCode::KeyG,
        "water_calm" => KeyCode::KeyC,
        "water_wild" => KeyCode::KeyW,
        "sand" => KeyCode::KeyS,
        _ => {
            warn!("no hard coded key for tile kind {id:?}");
            return;
        }
    };

    keymap.0.insert(keycode, event.entity);
}

#[derive(Resource, Default)]
struct TileKindKeyMap(HashMap<KeyCode, Entity>);

fn switch_cursor(
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor: ResMut<Cursor>,
    keymap: Res<TileKindKeyMap>,
) {
    for key in keys.get_just_pressed() {
        if let Some(entity) = keymap.0.get(key) {
            *cursor = Cursor::GroundTile(*entity);
        }
    }
}

fn write_place_tile_messages(
    mut mouse_motion: MessageReader<MouseMotion>,
    window: Single<&Window, With<PrimaryWindow>>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    cursor: Res<Cursor>,
    grid_size: Res<GridSize>,
    mut message_writer: MessageWriter<PlaceTile>,
) {
    if mouse_btn.pressed(MouseButton::Left) {
        if let Cursor::GroundTile(tile_kind) = *cursor {
            if let Some(pos) = window.cursor_position() {
                let pos = pos - window.size() / 2.0 + TILE_SIZE.as_vec2() / 2.0;
                if let Some(mouse_motion) = mouse_motion.read().next() {
                    let delta = mouse_motion.delta * window.scale_factor();
                    let mut starting_pos = pos - delta;
                    let tile_step_size = (TILE_SIZE.x as f32 + TILE_SIZE.y as f32) / 2.0;
                    let tile_step = mouse_motion
                        .delta
                        .clamp_length(tile_step_size, tile_step_size);
                    let step_count = (delta.length() / tile_step.length()).ceil() as usize;
                    for _ in 0..step_count {
                        if let Some(pos) = window_pos_to_grid_pos(starting_pos, &grid_size) {
                            message_writer.write(PlaceTile { pos, tile_kind });
                        }
                        starting_pos += tile_step;
                    }
                } else if mouse_btn.just_pressed(MouseButton::Left) {
                    if let Some(pos) = window_pos_to_grid_pos(pos, &grid_size) {
                        message_writer.write(PlaceTile { pos, tile_kind });
                    }
                }
            }
        }
    }
}

fn window_pos_to_grid_pos(window_pos: Vec2, grid_size: &GridSize) -> Option<GridPosition> {
    let half_grid_size = grid_size.as_vec2() / 2.0;
    let grid_pos = window_pos / TILE_SIZE.as_vec2() + half_grid_size;
    GridPosition::new(grid_pos, grid_size)
}

#[derive(Resource, Default)]
pub enum Cursor {
    #[default]
    Default,
    GroundTile(Entity),
}

#[derive(Message)]
pub struct PlaceTile {
    pub pos: GridPosition,
    pub tile_kind: Entity,
}
