use bevy::prelude::*;
use bevy_elf::AppExt;
use bevy_event_cascade::{EventCascadeAppExt, InStage};
use engine::{
    asset::AssetsExt,
    overworld::{
        lozo::{Lozo, LozoSource, SpawnOverworldObjects},
        tile::GridSize,
    },
};

use crate::{
    character::io::save::SavedCharacter,
    lozo::{SavedLozo, SavedLozoSource},
    tile::io::TileGridSpawned,
    ui::PlaceCharacter,
};

pub struct CharacterLoadPlugin;

impl Plugin for CharacterLoadPlugin {
    fn build(&self, app: &mut App) {
        app.register_event::<CharactersSpawned>()
            .init_ron_asset::<SavedCharacter>()
            .add_observer(spawn_characters);
    }
}

#[derive(EntityEvent)]
struct CharactersSpawned {
    #[event_target]
    lozo_entity: Entity,
}

impl InStage for CharactersSpawned {
    type Stage = SpawnOverworldObjects;
}

fn spawn_characters(
    event: On<TileGridSpawned>,
    lozo: Query<(&SavedLozoSource, &GridSize), With<Lozo>>,
    lozo_assets: Res<Assets<SavedLozo>>,
    character_assets: Res<Assets<SavedCharacter>>,
    mut message_writer: MessageWriter<PlaceCharacter>,
    mut commands: Commands,
) -> Result {
    let (lozo, grid_size) = lozo.get(event.event_target())?;

    let saved_lozo = lozo_assets.require_handle(lozo.handle())?;
    for character_handle in &saved_lozo.characters {
        let saved_character = character_assets.require_handle(character_handle)?;

        let world_position = grid_size.grid_to_world(saved_character.position.as_vec2());
        message_writer.write(PlaceCharacter {
            lozo_entity: event.event_target(),
            world_position,
            character_kind: saved_character.kind.clone(),
            orientation: saved_character.orientation,
        });
    }

    commands.trigger(CharactersSpawned {
        lozo_entity: event.event_target(),
    });

    Ok(())
}
