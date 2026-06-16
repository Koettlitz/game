use std::ops::{Deref, DerefMut};

use bevy::prelude::*;
use engine::{
    overworld::tile::{GridSize, TILE_SIZE_VEC2},
    progress::ProgressState,
};
use input::InputPlugin;

pub use input::{PlaceObject, PlaceTile, RemoveTile};

mod camera;
mod input;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputPlugin)
            .add_observer(on_tile_grid_spawn)
            .add_systems(
                Update,
                draw_grid_bounds.run_if(in_state(ProgressState::Finished)),
            );
    }
}

pub fn screen_to_world(
    cursor_pos: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
) -> Vec2 {
    match camera.viewport_to_world_2d(camera_transform, cursor_pos) {
        Ok(world_pos) => world_pos,
        Err(e) => panic!("could not convert screen to world coords - {e}"),
    }
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
                    TILE_SIZE_VEC2,
                    Color::BLACK,
                )
                .outer_edges();
        } else {
            gizmos.rect_2d(
                Isometry2d::IDENTITY,
                grid_size.as_vec2() * TILE_SIZE_VEC2,
                Color::BLACK,
            );
        }
    }
}
