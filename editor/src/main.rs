use bevy::prelude::*;
use engine::{
    animation::AnimationPlugin,
    assets::{SpriteSheetId, SpriteSheetMap},
};
use strum::IntoEnumIterator;

use crate::{tile::TilePlugin, ui::UIPlugin};

mod tile;
mod ui;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, UIPlugin, TilePlugin, AnimationPlugin))
        .insert_state(State::LoadingAssets)
        .add_systems(Startup, init)
        .add_systems(
            PostUpdate,
            transition_state_on_assets_loaded.run_if(in_state(State::LoadingAssets)),
        )
        .run();
}

#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum State {
    LoadingAssets,
    Initialized,
}

fn init(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn transition_state_on_assets_loaded(
    sprite_sheets: Res<SpriteSheetMap>,
    mut next_state: ResMut<NextState<State>>,
) {
    if sprite_sheets.len() == SpriteSheetId::iter().count() {
        next_state.set(State::Initialized);
    }
}
