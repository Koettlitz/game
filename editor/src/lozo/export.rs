use std::collections::HashMap;

use bevy::prelude::*;
use bevy_elf::HasResolver;
use bevy_event_cascade::{StageCompleted, StageProgress};
use engine::overworld::{
    event::TileEventActionDef,
    lozo::{LozoAsset, LozoDef},
    tile::TileEdge,
};

use crate::{
    character::io::export::CharacterExport, io::resolve_and_flush,
    object::io::export::GameObjectExport, tile::io::TileGridExport,
};

pub struct LozoExportPlugin;
impl Plugin for LozoExportPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(init_lozo_export)
            .add_observer(commit_lozo_export);
    }
}

#[derive(EntityEvent)]
pub struct ExportLozo {
    #[event_target]
    pub lozo_entity: Entity,
}

#[derive(EntityEvent)]
pub struct LozoExportInitialized {
    pub lozo_entity: Entity,

    #[event_target]
    pub export_entity: Entity,
}

#[derive(Component)]
pub struct LozoExport;

pub struct ExportLozoObjects;

#[derive(Component, Default)]
pub struct EventsExport {
    pub char_left_events: HashMap<TileEdge, Vec<TileEventActionDef>>,
    pub char_entered_events: HashMap<TileEdge, Vec<TileEventActionDef>>,
    pub char_reached_events: HashMap<TileEdge, Vec<TileEventActionDef>>,
}

fn init_lozo_export(event: On<ExportLozo>, mut commands: Commands) {
    let entity = commands
        .spawn((
            LozoExport,
            StageProgress::<ExportLozoObjects>::default(),
            EventsExport::default(),
        ))
        .id();

    commands.trigger(LozoExportInitialized {
        lozo_entity: event.lozo_entity,
        export_entity: entity,
    });
}

fn commit_lozo_export(event: On<StageCompleted<ExportLozoObjects>>, mut commands: Commands) {
    commands
        .entity(event.event_target())
        .queue(|mut entity: EntityWorldMut| {
            let Some((tile_grid, game_objects, characters, events)) = entity.take::<(
                TileGridExport,
                GameObjectExport,
                CharacterExport,
                EventsExport,
            )>() else {
                return Err(BevyError::from(
                    "commit_lozo_export triggered, but not all the export components were there.",
                ));
            };
            entity.despawn();

            let lozo_def = LozoDef {
                width: tile_grid.width,
                height: tile_grid.height,
                tile_grid: tile_grid.grid,
                char_left_events: events.char_left_events,
                char_entered_events: events.char_entered_events,
                char_reached_events: events.char_reached_events,
                objects: game_objects.objects,
                characters: characters.characters,
            };
            resolve_and_flush(vec![("world".to_string(), lozo_def)], LozoAsset::resolver());
            Ok(())
        });
}
