use std::{fmt::Display, time::Duration};

use bevy::{asset::AssetEventSystems, prelude::*};
use thiserror::Error;

use crate::asset::{AssetsExt, animation::sprite::SpriteAnimationAsset};

#[derive(Component)]
pub struct Animated(Handle<SpriteAnimationAsset>);
impl Animated {
    pub fn by(animation: Handle<SpriteAnimationAsset>) -> Self {
        Self(animation)
    }
}

#[derive(Component)]
struct SpriteAnimation {
    asset_id: AssetId<SpriteAnimationAsset>,
    current: usize,
    timer: Timer,
}

impl SpriteAnimation {
    fn new(frame_duration: Duration, asset_id: AssetId<SpriteAnimationAsset>) -> Self {
        Self {
            asset_id,
            current: 0,
            timer: Timer::new(frame_duration, TimerMode::Repeating),
        }
    }
}

pub struct AnimationPlugin;
impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, update_animations)
            .add_observer(on_insert)
            .add_systems(
                PreUpdate,
                sync_entities_with_assets.after(AssetEventSystems),
            );
    }
}

fn on_insert(
    on_insert: On<Insert, Animated>,
    mut sprites: Query<(&mut Sprite, &Animated)>,
    tile_animations: Query<&SpriteAnimation>,
    assets: Res<Assets<SpriteAnimationAsset>>,
) -> Result<()> {
    let (mut sprite, animated) = sprites.get_mut(on_insert.entity)?;
    let asset = assets
        .get(animated.0.id())
        .ok_or_else(|| MissingAnimationAssetError)?;
    let animation = tile_animations
        .iter()
        .find(|a| a.asset_id == animated.0.id())
        .ok_or_else(|| MissingAnimationError)?;
    let index = asset.indices[animation.current];
    if let Some(ref mut atlas) = sprite.texture_atlas.as_mut() {
        atlas.index = index;
    } else {
        warn!("animated sprite had no texture atlas to animate on");
    }
    Ok(())
}

fn sync_entities_with_assets(
    mut message_reader: MessageReader<AssetEvent<SpriteAnimationAsset>>,
    assets: Res<Assets<SpriteAnimationAsset>>,
    mut query: Query<(Entity, &mut SpriteAnimation)>,
    mut commands: Commands,
) -> Result<()> {
    for msg in message_reader.read() {
        match msg {
            AssetEvent::LoadedWithDependencies { id } => {
                let asset = assets.require(*id)?;
                if let Some((_, mut animation)) = query.iter_mut().find(|(_, a)| &a.asset_id == id)
                {
                    animation.timer.set_duration(asset.frame_duration);
                } else {
                    commands.spawn(SpriteAnimation::new(asset.frame_duration, *id));
                }
            }
            AssetEvent::Removed { id } => {
                if let Some((entity, _)) = query.iter().find(|(_, a)| a.asset_id == *id) {
                    commands.entity(entity).despawn();
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn update_animations(
    mut tile_animations: Query<&mut SpriteAnimation>,
    assets: Res<Assets<SpriteAnimationAsset>>,
    mut sprites: Query<(&mut Sprite, &Animated)>,
    time: Res<Time>,
) -> Result<()> {
    for mut animation in &mut tile_animations {
        if !animation.timer.tick(time.delta()).just_finished() {
            continue;
        }
        let asset = assets
            .get(animation.asset_id)
            .ok_or_else(|| MissingAnimationAssetError)?;
        animation.current = (animation.current + 1) % asset.indices.len();
        let index = asset.indices[animation.current];
        for (mut sprite, animated) in &mut sprites {
            if animated.0.id() == animation.asset_id {
                if let Some(ref mut atlas) = sprite.texture_atlas.as_mut() {
                    atlas.index = index;
                } else {
                    warn!("animated sprite had no texture atlas to animate on");
                }
            }
        }
    }
    Ok(())
}

#[derive(Error, Debug)]
pub struct MissingAnimationAssetError;

impl Display for MissingAnimationAssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "missing SpriteAnimationAsset for SpriteAnimation")
    }
}

#[derive(Error, Debug)]
pub struct MissingAnimationError;

impl Display for MissingAnimationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "missing SpriteAnimation for SpriteAnimationAsset")
    }
}
