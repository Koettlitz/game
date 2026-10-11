use bevy::prelude::*;
use bevy_elf::{AssetRef, FromDef};
use bevy_entity_lookup::EntityId;
use bevy_event_cascade::{EventCascadeAppExt, InStage};
use engine::overworld::tile::GridSize;

use crate::{
    io::resolve_and_flush,
    lozo::{LozoSave, SaveLozoObjects, SavedLozo},
    object::{GameObject, asset::GameObjectKindAsset},
    tile::io::TileGridSaved,
};

pub struct GameObjectSavePlugin;

impl Plugin for GameObjectSavePlugin {
    fn build(&self, app: &mut App) {
        app.register_event::<GameObjectsSaved>()
            .add_observer(save_game_objects);
    }
}

#[derive(Asset, FromDef, TypePath)]
pub struct SavedGameObject {
    pub kind: AssetRef<GameObjectKindAsset>,
    pub position: UVec2,
}

#[derive(Component)]
pub struct GameObjectsSave {
    pub object_ids: Vec<String>,
}

#[derive(EntityEvent)]
pub struct GameObjectsSaved(Entity);

impl InStage for GameObjectsSaved {
    type Stage = SaveLozoObjects;
}

fn save_game_objects(
    event: On<TileGridSaved>,
    lozo_save: Query<&LozoSave>,
    grid_size: Query<&GridSize>,
    objects_query: Query<(&GameObject, &Transform, Option<&EntityId>)>,
    mut commands: Commands,
) -> Result {
    let mut objects: Vec<(String, SavedGameObjectDef)> = Vec::new();

    for (i, (object, transform, id)) in objects_query.iter().enumerate() {
        let position = grid_size
            .get(event.lozo_entity)?
            .world_to_grid(transform.translation.truncate())
            .ok_or_else(|| format!("object position out of grid bounds: {:?}", transform))?;

        objects.push((
            id.map(|id| id.value().to_owned())
                .unwrap_or_else(|| format!("game_object{i}")),
            SavedGameObjectDef {
                kind: object.kind_ref.id().to_owned(),
                position: position.as_uvec2(),
            },
        ));
    }

    let object_ids = objects.iter().map(|(id, _)| id.to_owned()).collect();

    let base_path = lozo_save.get(event.event_target())?.base_path()?;
    resolve_and_flush(objects, SavedLozo::objects_resolver(&base_path.to_string()));

    commands
        .entity(event.event_target())
        .insert(GameObjectsSave { object_ids });

    commands.trigger(GameObjectsSaved(event.event_target()));

    Ok(())
}
