use bevy::{asset::AssetPath, prelude::*};
use bevy_elf::{AssetResolver, FromDef, HasResolver, ResolveStrategy, asset_spec};
use bevy_event_cascade::{StageCompleted, StageProgress};

use crate::{
    character::io::save::{CharactersSave, SavedCharacter},
    io::resolve_and_flush,
    object::io::save::{GameObjectsSave, SavedGameObject},
    tile::io::{SavedTileGrid, SavedTileGridDef},
};

pub struct LozoSavePlugin;

impl Plugin for LozoSavePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(init_lozo_save)
            .add_observer(commit_lozo_save);
    }
}

#[derive(Asset, FromDef, TypePath)]
#[asset_spec(
    base_path = "editor://lozos",
    resolve_strategy = ResolveStrategy::DirectoryNameIsId { file_name: "lozo.ron" }
)]
pub struct SavedLozo {
    tile_grid: SavedTileGrid,

    #[elf(with_spec(sub_path = "game_objects"), expose_resolver)]
    objects: Vec<Handle<SavedGameObject>>,

    #[elf(with_spec(sub_path = "characters"), expose_resolver)]
    characters: Vec<Handle<SavedCharacter>>,
}

#[derive(Event)]
pub struct SaveLozo;

#[derive(EntityEvent)]
pub struct LozoSaveInitialized(Entity);

#[derive(Component)]
pub struct LozoSave {
    id: String,
}

impl LozoSave {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn base_path(&self) -> AssetPath<'static> {
        match SavedLozo::resolver().resolve(&self.id) {
            Ok(b) => b,
            Err(e) => panic!("Could not resolve SavedLozo id \"{}\" - {e}", self.id),
        }
    }
}

pub struct SaveLozoObjects;

fn init_lozo_save(_: On<SaveLozo>, mut commands: Commands) {
    let entity = commands
        .spawn((
            LozoSave {
                id: "world".to_owned(),
            },
            StageProgress::<SaveLozoObjects>::default(),
        ))
        .id();

    commands.trigger(LozoSaveInitialized(entity));
}

fn commit_lozo_save(event: On<StageCompleted<SaveLozoObjects>>, mut commands: Commands) {
    commands
        .entity(event.event_target())
        .queue(|mut entity: EntityWorldMut| {
            let Some((lozo, tile_grid, game_objects, characters)) =
                entity.take::<(LozoSave, SavedTileGridDef, GameObjectsSave, CharactersSave)>()
            else {
                return Err(BevyError::from(
                    "commit_lozo_export triggered, but not all the export components were there.",
                ));
            };

            entity.despawn();

            let lozo_def = SavedLozoDef {
                tile_grid,
                objects: game_objects.object_ids,
                characters: characters.character_ids,
            };

            resolve_and_flush(vec![(lozo.id.clone(), lozo_def)], SavedLozo::resolver());

            Ok(())
        });
}
