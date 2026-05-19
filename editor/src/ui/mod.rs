use bevy::prelude::*;
use engine::{overworld::tile::GridSize, progress::ProgressState};
use input::InputPlugin;

pub use input::{PlaceObject, PlaceTile, RemoveTile};
mod camera;
mod input;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputPlugin).add_systems(
            Update,
            draw_grid_bounds.run_if(in_state(ProgressState::Finished)),
        );
    }
}

fn draw_grid_bounds(mut gizmos: Gizmos, grid_size: Single<&GridSize>) {
    gizmos.grid_2d(
        Isometry2d::IDENTITY,
        grid_size.as_uvec2(),
        Vec2::splat(1.0),
        Color::BLACK,
    );
}
