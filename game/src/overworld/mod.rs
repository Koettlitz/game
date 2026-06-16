use bevy::prelude::*;

use asset::AssetPlugin;
use engine::overworld::{
    character::CharacterPlugin,
    lozo::{LozoPlugin, NextLozo},
    tile::TilePlugin,
};
use input::InputPlugin;

mod asset;
mod input;

pub struct OverworldPlugin;
impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins((
            AssetPlugin,
            LozoPlugin,
            CharacterPlugin,
            InputPlugin,
            TilePlugin,
        ))
        .add_systems(Startup, init_lozo);
    }
}

fn init_lozo(mut next_lozo: ResMut<NextLozo>) {
    next_lozo.set("world".to_string());
    next_lozo.auto_activate = true;
}
