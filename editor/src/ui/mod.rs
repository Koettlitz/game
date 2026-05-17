use bevy::prelude::*;
use input::InputPlugin;

pub use input::{PlaceObject, PlaceTile, RemoveTile};
mod input;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputPlugin);
    }
}
