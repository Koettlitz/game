use bevy::{asset::AssetPath, prelude::*};
use bevy_elf::{
    AppExt, AssetRef, AssetResolver, FromDef, HasResolver, ResolveError, ResolveStrategy,
    asset_spec,
};
use bevy_event_cascade::{StageCompleted, StageProgress};
use engine::overworld::lozo::LozoSource;
use thiserror::Error;

use crate::{
    character::io::save::{CharactersSave, SavedCharacter},
    io::resolve_and_flush,
    object::io::save::{GameObjectsSave, SavedGameObject},
    tile::io::{SavedTileGrid, SavedTileGridDef},
};

pub struct LozoSavePlugin;

impl Plugin for LozoSavePlugin {
    fn build(&self, app: &mut App) {
        app.init_ron_asset::<SavedLozo>()
            .add_observer(init_lozo_save)
            .add_observer(commit_lozo_save);
    }
}

#[derive(Asset, FromDef, TypePath)]
#[asset_spec(
    base_path = "editor://lozos",
    resolve_strategy = ResolveStrategy::DirectoryNameIsId { file_name: "lozo.ron" }
)]
pub struct SavedLozo {
    pub tile_grid: SavedTileGrid,

    #[elf(with_spec(sub_path = "game_objects"), expose_resolver)]
    pub objects: Vec<Handle<SavedGameObject>>,

    #[elf(with_spec(sub_path = "characters"), expose_resolver)]
    pub characters: Vec<Handle<SavedCharacter>>,
}

#[derive(Component, Clone)]
pub struct SavedLozoSource(AssetRef<SavedLozo>);

impl From<AssetRef<SavedLozo>> for SavedLozoSource {
    fn from(value: AssetRef<SavedLozo>) -> Self {
        Self(value)
    }
}

impl LozoSource for SavedLozoSource {
    type Asset = SavedLozo;

    fn handle(&self) -> &Handle<Self::Asset> {
        self.0.handle()
    }
}

#[derive(EntityEvent)]
pub struct SaveLozo {
    #[event_target]
    pub lozo_entity: Entity,
}

#[derive(EntityEvent)]
pub struct LozoSaveInitialized {
    pub lozo_entity: Entity,

    #[event_target]
    pub save_entity: Entity,
}

#[derive(Component)]
pub struct LozoSave {
    id: String,
}

impl LozoSave {
    pub fn base_path(&self) -> Result<AssetPath<'static>, LozoBasePathError> {
        let asset_path = SavedLozo::resolver().resolve(&self.id)?;
        asset_path
            .parent()
            .ok_or_else(|| LozoBasePathError::NoParent(asset_path))
    }
}

#[derive(Error, Debug)]
pub enum LozoBasePathError {
    #[error("Lozos AssetPath: \"{0}\" had no parent")]
    NoParent(AssetPath<'static>),

    #[error("Failed to resolve lozo id - {0}")]
    Resolve(#[from] ResolveError),
}

pub struct SaveLozoObjects;

fn init_lozo_save(event: On<SaveLozo>, mut commands: Commands) {
    let save_entity = commands
        .spawn((
            LozoSave {
                id: "world".to_owned(),
            },
            StageProgress::<SaveLozoObjects>::default(),
        ))
        .id();

    commands.trigger(LozoSaveInitialized {
        lozo_entity: event.lozo_entity,
        save_entity,
    });
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
