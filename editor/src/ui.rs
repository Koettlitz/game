use bevy::{prelude::*, window::PrimaryWindow};
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
    }
}

fn write_place_tile_messages(
    window: Single<&Window, With<PrimaryWindow>>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    cursor: Res<Cursor>,
    grid: Res<GroundTileGrid>,
    mut message_writer: MessageWriter<PlaceTile>,
) {
    if mouse_btn.pressed(MouseButton::Left) {
        if let Cursor::GroundTile(tile_kind) = *cursor {
            if let Some(pos) = window.cursor_position() {
                let pos = pos - window.size() / 2.0;
                let pos = window_pos_to_grid_pos(pos, &grid);
                message_writer.write(PlaceTile { pos, tile_kind });
            }
        }
    }
}

fn window_pos_to_grid_pos(window_pos: Vec2, grid: &GroundTileGrid) -> UVec2 {
    let half_grid_size = Vec2::new(grid.width() as f32 / 2.0, grid.height() as f32 / 2.0);
    let half_tile_size = TILE_SIZE.as_vec2() / 2.0;
    ((window_pos + half_tile_size) / TILE_SIZE.as_vec2() + half_grid_size).as_uvec2()
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
