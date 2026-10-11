use bevy::prelude::*;
use bevy_elf::{AssetRef, FromDef};
use bevy_entity_lookup::EntityId;
use bevy_event_cascade::{EventCascadeAppExt, InStage};
use engine::overworld::{character::Orientation, tile::GridSize};

use crate::{
    character::{Character, asset::CharacterKindAsset},
    io::resolve_and_flush,
    lozo::{LozoSave, SaveLozoObjects, SavedLozo},
    tile::io::TileGridSaved,
};

pub struct CharacterSavePlugin;

impl Plugin for CharacterSavePlugin {
    fn build(&self, app: &mut App) {
        app.register_event::<CharactersSaved>()
            .add_observer(save_characters);
    }
}

#[derive(Asset, FromDef, TypePath)]
pub struct SavedCharacter {
    pub kind: AssetRef<CharacterKindAsset>,
    pub position: UVec2,
    pub orientation: Orientation,
}

impl InStage for CharactersSaved {
    type Stage = SaveLozoObjects;
}

#[derive(EntityEvent)]
pub struct CharactersSaved {
    #[event_target]
    pub save_entity: Entity,
}

#[derive(Component)]
pub struct CharactersSave {
    pub character_ids: Vec<String>,
}

fn save_characters(
    event: On<TileGridSaved>,
    lozo_save: Query<&LozoSave>,
    grid_size: Query<&GridSize>,
    characters: Query<(Option<&EntityId>, &Character, &Transform)>,
    mut commands: Commands,
) -> Result {
    let mut saved_characters: Vec<(String, SavedCharacterDef)> = Vec::new();

    for (i, (id, character, transform)) in characters.iter().enumerate() {
        let position = grid_size
            .get(event.lozo_entity)?
            .world_to_grid(transform.translation.truncate())
            .ok_or_else(|| format!("characer out of grid bounds: {}", transform.translation))?
            .as_uvec2();

        saved_characters.push((
            id.map(|id| id.value().to_owned())
                .unwrap_or_else(|| format!("characer{i}")),
            SavedCharacterDef {
                kind: character.asset_ref.id().to_owned(),
                position,
                orientation: character.orientation,
            },
        ));
    }

    let character_ids = saved_characters
        .iter()
        .map(|(id, _)| id.to_owned())
        .collect();

    let base_path = lozo_save.get(event.event_target())?.base_path()?;
    resolve_and_flush(
        saved_characters,
        SavedLozo::characters_resolver(&base_path.to_string()),
    );

    commands
        .entity(event.event_target())
        .insert(CharactersSave { character_ids });

    commands.trigger(CharactersSaved {
        save_entity: event.save_entity,
    });

    Ok(())
}
