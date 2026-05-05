use std::{fmt::Display, time::Duration};

use bevy::prelude::*;
use thiserror::Error;

use crate::asset::{AssetRef, animations::sprite::SpriteAnimationAsset};

#[derive(Component)]
pub struct Animated(AssetRef<SpriteAnimationAsset>);
impl Animated {
    pub fn by(animation: AssetRef<SpriteAnimationAsset>) -> Self {
        Self(animation)
    }

    pub fn id(&self) -> &str {
        self.0.id()
    }
}

#[derive(Component)]
pub struct SpriteAnimation {
    asset_ref: AssetRef<SpriteAnimationAsset>,
    current: usize,
    timer: Timer,
}

impl SpriteAnimation {
    pub fn new(frame_duration: Duration, asset_ref: AssetRef<SpriteAnimationAsset>) -> Self {
        Self {
            asset_ref,
            current: 0,
            timer: Timer::new(frame_duration, TimerMode::Repeating),
        }
    }
}

pub struct AnimationPlugin;
impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, update_animations)
            .add_observer(on_insert);
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
        .get(animated.0.handle().id())
        .ok_or_else(|| MissingAnimationAssetError)?;
    let animation = tile_animations
        .iter()
        .find(|a| a.asset_ref.id() == animated.0.id())
        .ok_or_else(|| MissingAnimationError)?;
    let index = asset.indices[animation.current];
    if let Some(ref mut atlas) = sprite.texture_atlas.as_mut() {
        atlas.index = index;
    } else {
        warn!("animated sprite had no texture atlas to animate on");
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
            .get(animation.asset_ref.handle().id())
            .ok_or_else(|| MissingAnimationAssetError)?;
        animation.current = (animation.current + 1) % asset.indices.len();
        let index = asset.indices[animation.current];
        for (mut sprite, animated) in &mut sprites {
            if animated.0 == animation.asset_ref {
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
