use bevy::prelude::*;

use asset::AssetPlugin;
use lozo::{LozoPlugin, LozoState};

use lozo::NextLozo;

use crate::overworld::character::CharacterPlugin;
use crate::overworld::input::InputPlugin;

pub mod asset;
mod character;
mod input;
pub mod lozo;
mod tile;

pub struct OverworldPlugin;
impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins((AssetPlugin, LozoPlugin, CharacterPlugin, InputPlugin))
            .configure_sets(
                FixedUpdate,
                OverworldLogic.run_if(in_state(LozoState::Default)),
            )
            .add_systems(Startup, init_lozo);
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OverworldLogic;

fn init_lozo(mut next_lozo: ResMut<NextLozo>) {
    next_lozo.set("world".to_string());
}
