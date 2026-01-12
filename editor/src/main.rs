use bevy::{DefaultPlugins, app::App};

use crate::{tile::TilePlugin, ui::UIPlugin};

mod tile;
mod ui;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TilePlugin, UIPlugin))
        .run();
}
