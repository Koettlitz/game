use std::ops::Deref;

use bevy::{asset::io::AssetSourceBuilder, prelude::*};
use bevy_elf::RonAssetLoader;
use engine::animation::{AnimationTimersAsset, SpriteAnimationPlugin};

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
            SpriteAnimationPlugin,
        ))
        .init_asset_loader::<RonAssetLoader<TextureAtlasLayout>>()
        .add_systems(Startup, init)
        .run();
}

#[derive(Resource)]
pub struct AnimationTimers(Handle<AnimationTimersAsset>);

impl Deref for AnimationTimers {
    type Target = Handle<AnimationTimersAsset>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load("animation_timers.ron");
    commands.insert_resource(AnimationTimers(handle));
}
