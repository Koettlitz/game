use bevy::{asset::io::AssetSourceBuilder, prelude::*};
use engine::{
    animation::AnimationPlugin,
    progress::{ProgressPlugin, ProgressScreen, ProgressState},
};

use crate::{assets::AssetsPlugin, object::GameObjectPlugin, tile::TilePlugin, ui::UIPlugin};

mod assets;
mod io;
mod object;
mod tile;
mod ui;

mod asset_registry {
    include!(concat!(env!("OUT_DIR"), "/asset_registry.rs"));
}

fn main() {
    App::new()
        .register_asset_source(
            "editor",
            AssetSourceBuilder::platform_default("editor/assets", None),
        )
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            AssetsPlugin,
            ProgressPlugin,
            UIPlugin,
            TilePlugin,
            GameObjectPlugin,
            AnimationPlugin,
        ))
        .add_systems(Startup, init)
        .add_systems(OnEnter(ProgressState::Finished), remove_progress_screen)
        .run();
}

fn init(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn(ProgressScreen);
}

fn remove_progress_screen(mut commands: Commands, query: Query<Entity, With<ProgressScreen>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
