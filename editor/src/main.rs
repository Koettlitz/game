use bevy::{asset::io::AssetSourceBuilder, prelude::*};
use engine::{
    animation::SpriteAnimationPlugin,
    progress::{ProgressPlugin, ProgressScreen, ProgressState},
};

use crate::{io::IoPlugin, object::GameObjectPlugin, tile::TilePlugin, ui::UiPlugin};

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
        .register_asset_source(
            "game",
            AssetSourceBuilder::platform_default("game/assets", None),
        )
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            asset::AssetPlugin,
            ProgressPlugin,
            UiPlugin,
            TilePlugin,
            GameObjectPlugin,
            SpriteAnimationPlugin,
            IoPlugin,
        ))
        .add_systems(Startup, init)
        .add_systems(OnEnter(ProgressState::Finished), remove_progress_screen)
        .run();
}

fn init(mut commands: Commands) {
    commands.spawn(ProgressScreen);
}

fn remove_progress_screen(mut commands: Commands, query: Query<Entity, With<ProgressScreen>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
