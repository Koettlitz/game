use bevy::prelude::*;
use bevy_elf::AppExt;
use bevy_event_cascade::{EventCascadeAppExt, InStage};
use engine::{
    asset::AssetsExt,
    overworld::{
        lozo::{LozoSource, SpawnOverworldObjects},
        tile::GridSize,
    },
};

use crate::{
    lozo::{SavedLozo, SavedLozoSource},
    object::io::save::SavedGameObject,
    tile::io::TileGridSpawned,
    ui::PlaceObject,
};

pub struct GameObjectLoadPlugin;

impl Plugin for GameObjectLoadPlugin {
    fn build(&self, app: &mut App) {
        app.register_event::<GameObjectsSpawned>()
            .init_ron_asset::<SavedGameObject>()
            .add_observer(spawn_objects);
    }
}

#[derive(EntityEvent)]
struct GameObjectsSpawned {
    #[event_target]
    lozo_entity: Entity,
}

impl InStage for GameObjectsSpawned {
    type Stage = SpawnOverworldObjects;
}

fn spawn_objects(
    event: On<TileGridSpawned>,
    lozos: Query<(&SavedLozoSource, &GridSize)>,
    lozo_assets: Res<Assets<SavedLozo>>,
    objects: Res<Assets<SavedGameObject>>,
    mut message_writer: MessageWriter<PlaceObject>,
    mut commands: Commands,
) -> Result {
    let (lozo, grid_size) = lozos.get(event.event_target())?;
    let saved_lozo = lozo_assets.require_handle(lozo.handle())?;

    for saved_object in &saved_lozo.objects {
        let saved_object = objects.require_handle(saved_object)?;
        let world_position = grid_size.grid_to_world(saved_object.position.as_vec2());

        message_writer.write(PlaceObject {
            lozo_entity: event.event_target(),
            world_position,
            object_kind: saved_object.kind.clone(),
        });
    }

    commands.trigger(GameObjectsSpawned {
        lozo_entity: event.event_target(),
    });

    Ok(())
}
