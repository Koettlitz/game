use bevy::{asset::io::AssetSourceBuilder, prelude::*};

use crate::overworld::OverworldPlugin;

mod overworld;

fn main() {
    App::new()
        .register_asset_source(
            "game",
            AssetSourceBuilder::platform_default("game/assets", None),
        )
        .add_plugins(OverworldPlugin)
        .run();
}
