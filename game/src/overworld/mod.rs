use bevy::prelude::*;

use bevy_elf::{AssetResolver, HasResolver};
use engine::{
    asset::overworld::{CHARACTER_LAYER, character::CharacterAsset},
    overworld::{
        character::{Character, CharacterPlugin, LoadingCharacter},
        lozo::{LozoPlugin, NextLozo},
        object::GameObjectPlugin,
        tile::{GridSize, TileGridSpawned, TilePlugin},
    },
};
use input::InputPlugin;

mod input;

pub struct OverworldPlugin;
impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins((
            LozoPlugin,
            CharacterPlugin,
            InputPlugin,
            TilePlugin,
            GameObjectPlugin,
        ))
        .add_systems(Startup, (init_lozo, load_character_asset))
        .add_observer(on_tile_grid_spawned);
    }
}

fn init_lozo(mut next_lozo: ResMut<NextLozo>) {
    next_lozo.set("world".to_string());
    next_lozo.auto_activate = true;
}

fn load_character_asset(mut commands: Commands, asset_server: ResMut<AssetServer>) -> Result {
    let handle = asset_server.load(CharacterAsset::resolver().resolve("brendan")?);
    commands.insert_resource(LoadingCharacter(handle));
    Ok(())
}

fn on_tile_grid_spawned(
    event: On<TileGridSpawned>,
    grid_size: Query<&GridSize>,
    character: Option<Single<&mut Transform, With<Character>>>,
) -> Result {
    if let Some(mut character) = character {
        character.translation = grid_size
            .get(event.entity())?
            .snap_to_tile(Vec2::new(0.0, 0.0))
            .extend(CHARACTER_LAYER);
    }

    Ok(())
}
