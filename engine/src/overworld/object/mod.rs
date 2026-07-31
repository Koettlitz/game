use bevy_elf::AppExt;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};
use thiserror::Error;

use bevy::{camera::visibility::RenderLayers, prelude::*};

use crate::{
    animation::Animated,
    asset::AssetsExt,
    overworld::lozo::{Lozo, LozoAsset, LozoCamAttached, LozoCommands},
};

pub use asset::*;

mod asset;

pub struct GameObjectPlugin;

impl Plugin for GameObjectPlugin {
    fn build(&self, app: &mut App) {
        app.init_ron_asset::<GameObjectSpriteAsset>()
            .add_observer(spawn_objects);
    }
}

fn spawn_objects(
    event: On<LozoCamAttached>,
    lozo_query: Query<&Lozo>,
    render_layers: Query<&RenderLayers>,
    lozo_assets: Res<Assets<LozoAsset>>,
    mut object_lookup: Query<&mut ObjectSpriteLookup, With<Lozo>>,
    mut commands: LozoCommands,
    object_assets: Res<Assets<GameObjectSpriteAsset>>,
) -> Result {
    let lozo = lozo_query.get(event.lozo_entity)?;
    let lozo_asset = lozo_assets.require_handle(lozo.handle())?;
    let mut object_lookup = object_lookup.get_mut(event.lozo_entity)?;

    for object in &lozo_asset.objects {
        let asset = object_assets.require_handle(object.handle())?;
        let entity = spawn_object_sprite(
            event.lozo_entity,
            asset,
            render_layers.get(event.camera_entity)?.clone(),
            &mut commands,
        )?;
        object_lookup.insert(object.id().to_string(), entity);
    }

    Ok(())
}

fn spawn_object_sprite(
    lozo_entity: Entity,
    object_asset: &GameObjectSpriteAsset,
    render_layers: RenderLayers,
    commands: &mut LozoCommands,
) -> Result<Entity> {
    let transform = Transform::from_translation(object_asset.world_position);
    if let Some(TextureAtlasData { layout, kind }) = &object_asset.sprite_kind {
        match kind {
            SpriteKind::Static { idx } => commands.spawn_into_lozo(
                lozo_entity,
                (
                    Sprite::from_atlas_image(
                        object_asset.image.clone(),
                        TextureAtlas {
                            layout: layout.clone(),
                            index: *idx,
                        },
                    ),
                    render_layers,
                    transform,
                ),
            ),
            SpriteKind::Animated { animation } => commands.spawn_into_lozo(
                lozo_entity,
                (
                    Sprite::from_atlas_image(
                        object_asset.image.clone(),
                        TextureAtlas {
                            layout: layout.clone(),
                            ..Default::default()
                        },
                    ),
                    render_layers,
                    Animated::by(animation.clone()),
                    transform,
                ),
            ),
        }
    } else {
        commands.spawn_into_lozo(
            lozo_entity,
            (
                Sprite::from_image(object_asset.image.clone()),
                render_layers,
                transform,
            ),
        )
    }
    .map(|e| e.id())
}

#[derive(Component, Default)]
pub struct ObjectSpriteLookup(HashMap<String, Entity>);

impl Deref for ObjectSpriteLookup {
    type Target = HashMap<String, Entity>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ObjectSpriteLookup {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ObjectSpriteLookup {
    pub fn lookup(&self, id: &str) -> Result<Entity> {
        Ok(self
            .get(id)
            .ok_or_else(|| ObjectSpriteLookupFailed(id.to_string()))
            .copied()?)
    }
}

#[derive(Error, Debug)]
#[error("missing object sprite \"{0}\" in ObjectSpriteLookup")]
pub struct ObjectSpriteLookupFailed(String);
