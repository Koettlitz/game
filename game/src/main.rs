use bevy::{asset::io::AssetSourceBuilder, prelude::*};

use crate::overworld::lozo::LozoPlugin;

mod overworld;

fn main() {
    App::new()
        .register_asset_source(
            "game",
            AssetSourceBuilder::platform_default("game/assets", None),
        )
        .add_plugins(LozoPlugin)
        .run();
}
