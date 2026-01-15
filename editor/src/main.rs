use bevy::prelude::*;

use crate::{tile::TilePlugin, ui::UIPlugin};

mod tile;
mod ui;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, UIPlugin, TilePlugin))
        .add_systems(Startup, init)
        .run();
}

fn init(mut commands: Commands) {
    commands.spawn(Camera2d);
}
