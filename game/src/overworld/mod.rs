use bevy::prelude::Plugin;

use crate::overworld::{asset::AssetPlugin, lozo::LozoPlugin};

pub mod asset;
pub mod lozo;
mod tile;

pub struct OverworldPlugin;
impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins((AssetPlugin, LozoPlugin));
    }
}
