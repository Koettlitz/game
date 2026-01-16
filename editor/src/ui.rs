use bevy::{input::mouse::MouseMotion, prelude::*, window::PrimaryWindow};
use engine::assets::TILE_SIZE;

use crate::tile::{GroundTileGrid, GroundTileKind};

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlaceTile>()
            .init_resource::<Cursor>()
            .add_systems(PreUpdate, (write_place_tile_messages, switch_cursor));
    }
}

fn switch_cursor(keys: Res<ButtonInput<KeyCode>>, mut cursor: ResMut<Cursor>) {
    if keys.just_pressed(KeyCode::KeyG) {
        *cursor = Cursor::GroundTile(GroundTileKind::Gras);
    } else if keys.just_pressed(KeyCode::KeyW) {
        *cursor = Cursor::GroundTile(GroundTileKind::WaterCalm);
    } else if keys.just_pressed(KeyCode::KeyD) {
        *cursor = Cursor::GroundTile(GroundTileKind::WaterDeep);
    }
}

fn write_place_tile_messages(
    mut mouse_motion: MessageReader<MouseMotion>,
    window: Single<&Window, With<PrimaryWindow>>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    cursor: Res<Cursor>,
    grid: Res<GroundTileGrid>,
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
                        if let Some(pos) = window_pos_to_grid_pos(starting_pos, &grid) {
                            message_writer.write(PlaceTile { pos, tile_kind });
                        }
                        starting_pos += tile_step;
                    }
                } else if mouse_btn.just_pressed(MouseButton::Left) {
                    if let Some(pos) = window_pos_to_grid_pos(pos, &grid) {
                        message_writer.write(PlaceTile { pos, tile_kind });
                    }
                }
            }
        }
    }
}

fn window_pos_to_grid_pos(window_pos: Vec2, grid: &GroundTileGrid) -> Option<UVec2> {
    let half_grid_size = grid.size().as_vec2() / 2.0;
    let grid_pos = window_pos / TILE_SIZE.as_vec2() + half_grid_size;
    if grid.contains(grid_pos) {
        Some(grid_pos.as_uvec2())
    } else {
        None
    }
}

#[derive(Resource, Default)]
pub enum Cursor {
    #[default]
    Default,
    GroundTile(GroundTileKind),
}

#[derive(Message)]
pub struct PlaceTile {
    pub pos: UVec2,
    pub tile_kind: GroundTileKind,
}
