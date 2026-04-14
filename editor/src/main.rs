use bevy::{asset::io::AssetSourceBuilder, prelude::*};
use engine::{
    animation::AnimationPlugin,
    progress::{ProgressPlugin, ProgressScreen, ProgressState},
};

use crate::{object::GameObjectPlugin, tile::TilePlugin, ui::UIPlugin};

mod asset;
mod io;
mod object;
mod tile;
mod ui;

fn main() {
    App::new()
        .register_asset_source(
            "editor",
            AssetSourceBuilder::platform_default("editor/assets", None),
        )
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            asset::AssetPlugin,
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
