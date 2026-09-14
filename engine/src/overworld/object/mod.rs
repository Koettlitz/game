use bevy_elf::AppExt;
use bevy_entity_lookup::EntityId;

use bevy::prelude::*;

use crate::{
    animation::Animated,
    asset::AssetsExt,
    overworld::lozo::{
        InLozoSpawnPhase, InitLozo, Lozo, LozoAppExt, LozoAsset, LozoCommands,
        SpawnOverworldObjects,
    },
};

pub use asset::*;

mod asset;

pub struct GameObjectPlugin;

impl Plugin for GameObjectPlugin {
    fn build(&self, app: &mut App) {
        app.init_ron_asset::<GameObjectSpriteAsset>()
            .register_lozo_spawn_event::<GameObjectsSpawned>()
            .add_observer(spawn_objects);
    }
}

#[derive(Event)]
struct GameObjectsSpawned(Entity);

impl InLozoSpawnPhase for GameObjectsSpawned {
    type SpawnPhase = SpawnOverworldObjects;

    fn lozo_entity(&self) -> Entity {
        self.0
    }
}

fn spawn_objects(
    event: On<InitLozo>,
    lozo_query: Query<&Lozo>,
    lozo_assets: Res<Assets<LozoAsset>>,
    mut commands: LozoCommands,
    object_assets: Res<Assets<GameObjectSpriteAsset>>,
) -> Result {
    let lozo = lozo_query.get(event.entity())?;
    let lozo_asset = lozo_assets.require_handle(lozo.handle())?;

    for object in &lozo_asset.objects {
        let asset = object_assets.require_handle(object.handle())?;
        spawn_object_sprite(asset.id.clone(), event.entity(), asset, &mut commands)?;
    }

    commands.trigger(GameObjectsSpawned(event.entity()));

    Ok(())
}

fn spawn_object_sprite(
    id: EntityId,
    lozo_entity: Entity,
    object_asset: &GameObjectSpriteAsset,
    commands: &mut LozoCommands,
) -> Result<Entity> {
    let transform = Transform::from_translation(object_asset.world_position);
    if let Some(TextureAtlasData { layout, kind }) = &object_asset.sprite_kind {
        match kind {
            SpriteKind::Static { idx } => commands.spawn_into_lozo(
                lozo_entity,
                (
                    id,
                    Sprite::from_atlas_image(
                        object_asset.image.clone(),
                        TextureAtlas {
                            layout: layout.clone(),
                            index: *idx,
                        },
                    ),
                    transform,
                ),
            ),
            SpriteKind::Animated { animation } => commands.spawn_into_lozo(
                lozo_entity,
                (
                    id,
                    Sprite::from_atlas_image(
                        object_asset.image.clone(),
                        TextureAtlas {
                            layout: layout.clone(),
                            ..Default::default()
                        },
                    ),
                    Animated::by(animation.clone()),
                    transform,
                ),
            ),
        }
    } else {
        commands.spawn_into_lozo(
            lozo_entity,
            (
                id,
                Sprite::from_image(object_asset.image.clone()),
                transform,
            ),
        )
    }
    .map(|e| e.id())
}
