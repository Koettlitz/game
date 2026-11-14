use std::collections::HashMap;

use bevy::prelude::*;
use bevy_event_cascade::{EventCascadeAppExt, InStage};
use engine::{
    asset::AssetsExt,
    overworld::{
        character::{
            Orientation,
            asset::{CharacterBehaviourDef, CharacterDef, Route},
        },
        lozo::LozoAsset,
        tile::GridSize,
    },
};

use crate::{
    character::{Character, asset::CharacterKindAsset},
    io::resolve_and_flush,
    lozo::ExportLozoObjects,
    tile::io::{TileGridExport, TileGridExported},
};

pub struct ExportPlugin;

impl Plugin for ExportPlugin {
    fn build(&self, app: &mut App) {
        app.register_event::<CharactersExported>()
            .add_observer(export_characters);
    }
}

#[derive(EntityEvent)]
pub struct CharactersExported(Entity);

impl InStage for CharactersExported {
    type Stage = ExportLozoObjects;
}

#[derive(Component)]
pub struct CharacterExport {
    pub characters: Vec<String>,
}

fn export_characters(
    event: On<TileGridExported>,
    character_kinds: Res<Assets<CharacterKindAsset>>,
    character_query: Query<(&Character, &Transform)>,
    grid_size: Query<&GridSize>,
    mut grid_export: Query<&mut TileGridExport>,
    mut commands: Commands,
) -> Result {
    let mut characters = HashMap::new();

    for (i, (character, transform)) in character_query.iter().enumerate() {
        let character_kind = character_kinds.require_handle(character.asset_ref.handle())?;

        let Some(position) = grid_size
            .get(event.lozo_entity)?
            .world_to_grid(transform.translation.truncate())
        else {
            return Err(BevyError::from(format!(
                "character position out of grid bounds {}",
                transform.translation
            )));
        };

        if let Some(ref mut tile_def) =
            grid_export.get_mut(event.event_target())?.grid[*position.as_index()]
        {
            tile_def.blocked = true;
        }

        characters.insert(
            format!("{}_{i}", character.asset_ref.id()),
            CharacterDef {
                dialog: Some("Rück mir nich so auf die Pelle!".to_string()),
                spritesheet: character_kind.spritesheet.clone().into_def(),
                animations: character_kind
                    .animations
                    .iter()
                    .map(|(state, visual)| (*state, visual.clone().into_def()))
                    .collect(),
                position: transform.translation,
                orientation: character.orientation,
                behaviour: default_route(*position, &character.orientation)
                    .map(CharacterBehaviourDef::Walking),
            },
        );
    }

    let character_ids = characters.keys().cloned().collect();
    resolve_and_flush(characters, LozoAsset::characters_resolver());

    let entity = commands
        .entity(event.event_target())
        .insert(CharacterExport {
            characters: character_ids,
        })
        .id();

    commands.trigger(CharactersExported(entity));
    Ok(())
}

fn default_route(start: UVec2, orientation: &Orientation) -> Option<Route> {
    let first: UVec2 = (start.as_ivec2() + orientation.grid_direction() * 4)
        .try_into()
        .ok()?;
    let second: UVec2 = (first.as_ivec2() + orientation.rotate().grid_direction() * 4)
        .try_into()
        .ok()?;
    let third = (second.as_ivec2() + orientation.rotate().rotate().grid_direction() * 4)
        .try_into()
        .ok()?;

    Some(Route {
        targets: vec![first, second, third, start],
        cycle: true,
    })
}
