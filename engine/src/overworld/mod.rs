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

pub const TILE_LAYER: f32 = 20.0;
pub const CHARACTER_LAYER: f32 = 100.0;
pub const OBJECT_LAYER_TOP: f32 = 128.0;
pub const OBJECT_LAYER_BOTTOM: f32 = 99.9;

pub struct OverworldPlugin;

impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            CameraPlugin,
            CharacterPlugin,
            GameObjectPlugin,
            LozoPlugin,
            TilePlugin,
        ));
    }
}
