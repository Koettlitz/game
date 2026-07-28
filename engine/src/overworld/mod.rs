use bevy::prelude::*;

use camera::CameraPlugin;
use character::CharacterPlugin;
use lozo::LozoPlugin;
use object::GameObjectPlugin;
use tile::TilePlugin;

pub mod camera;
pub mod character;
pub mod input;
pub mod lozo;
pub mod object;
pub mod tile;

pub struct OverworldPlugin;

impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            LozoPlugin,
            TilePlugin,
            GameObjectPlugin,
            CharacterPlugin,
            CameraPlugin,
        ));
    }
}
