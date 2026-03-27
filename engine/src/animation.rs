use std::fmt::Display;

use bevy::prelude::*;

use crate::assets::animations::sprite::SpriteAnimationAsset;

#[derive(Component)]
pub struct Animated(Entity);
impl Animated {
    pub fn by(animation_entity: Entity) -> Self {
        Self(animation_entity)
    }
}

#[derive(Component)]
pub struct SpriteAnimation {
    handle: Handle<SpriteAnimationAsset>,
    current: usize,
    timer: Timer,
}

impl SpriteAnimation {
    pub fn new(asset: &SpriteAnimationAsset, handle: Handle<SpriteAnimationAsset>) -> Self {
        Self {
            handle,
            current: 0,
            timer: Timer::new(asset.frame_duration, TimerMode::Repeating),
        }
    }

    pub fn with_asset<'a>(
        &'a self,
        assets: &'a Assets<SpriteAnimationAsset>,
    ) -> Result<SpriteAnimationView<'a>, MissingAnimationAssetError> {
        let Some(asset) = assets.get(self.handle.id()) else {
            return Err(MissingAnimationAssetError);
        };
        Ok(SpriteAnimationView {
            asset,
            origin: self,
        })
    }

    pub fn with_asset_mut<'a>(
        &'a mut self,
        assets: &'a Assets<SpriteAnimationAsset>,
    ) -> Result<SpriteAnimationViewMut<'a>, MissingAnimationAssetError> {
        let Some(asset) = assets.get(self.handle.id()) else {
            return Err(MissingAnimationAssetError);
        };
        Ok(SpriteAnimationViewMut {
            asset,
            origin: self,
        })
    }
}

pub struct AnimationPlugin;
impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, update_animations);
    }
}

fn update_animations(
    tile_animations: Query<(Entity, &mut SpriteAnimation)>,
    assets: Res<Assets<SpriteAnimationAsset>>,
    mut tiles: Query<(&mut Sprite, &Animated)>,
    time: Res<Time>,
) {
    for (entity, mut animation) in tile_animations {
        let Ok(mut animation) = animation.with_asset_mut(&assets) else {
            continue;
        };
        let Some(index) = animation.update(&time) else {
            continue;
        };
        for (mut sprite, animated) in tiles.iter_mut() {
            if animated.0 == entity {
                sprite
                    .texture_atlas
                    .as_mut()
                    .expect("missing texture atlas in animated sprite")
                    .index = index;
            }
        }
    }
}

pub struct SpriteAnimationViewMut<'a> {
    origin: &'a mut SpriteAnimation,
    asset: &'a SpriteAnimationAsset,
}

impl<'a> SpriteAnimationView<'a> {
    pub fn current_idx(&self) -> usize {
        self.asset.indices[self.origin.current]
    }
}

pub struct SpriteAnimationView<'a> {
    origin: &'a SpriteAnimation,
    asset: &'a SpriteAnimationAsset,
}

impl<'a> SpriteAnimationViewMut<'a> {
    pub fn current_idx(&self) -> usize {
        self.asset.indices[self.origin.current]
    }

    fn update(&mut self, time: &Time) -> Option<usize> {
        if self.origin.timer.tick(time.delta()).just_finished() {
            Some(self.next_index())
        } else {
            None
        }
    }

    fn next_index(&mut self) -> usize {
        if self.origin.current == self.asset.indices.len() - 1 {
            self.origin.current = 0;
        } else {
            self.origin.current += 1;
        }
        self.current_idx()
    }
}

#[derive(Debug)]
pub struct MissingAnimationAssetError;

impl Display for MissingAnimationAssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "missing AnimationAsset for animation")
    }
}
