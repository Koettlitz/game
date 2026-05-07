use bevy::{asset::io::AssetSourceBuilder, prelude::*};
use engine::animation::AnimationPlugin;

use crate::overworld::OverworldPlugin;

mod overworld;

fn main() {
    App::new()
        .register_asset_source(
            "game",
            AssetSourceBuilder::platform_default("game/assets", None),
        )
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            OverworldPlugin,
            AnimationPlugin,
        ))
        .add_systems(Startup, init)
        .run();
}

fn init(mut commands: Commands) {
    commands.spawn(Camera2d);
}
