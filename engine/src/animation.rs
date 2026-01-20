use std::time::Duration;

use bevy::prelude::*;

pub struct AnimationPlugin;
impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, update_animations);
    }
}

fn update_animations(
    tile_animations: Query<(Entity, &mut SpriteAnimation)>,
    mut tiles: Query<(&mut Sprite, &Animated)>,
    time: Res<Time>,
) {
    for (entity, mut animation) in tile_animations {
        if let Some(index) = animation.update(&time) {
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
}

#[derive(Component)]
pub struct SpriteAnimation {
    indices: Vec<usize>,
    current: usize,
    timer: Timer,
}

impl SpriteAnimation {
    pub fn new(indices: impl Into<Vec<usize>>, frame_duration: Duration) -> Self {
        Self {
            indices: indices.into(),
            current: 0,
            timer: Timer::new(frame_duration, TimerMode::Repeating),
        }
    }

    pub fn current_idx(&self) -> usize {
        self.indices[self.current]
    }

    pub fn update(&mut self, time: &Time) -> Option<usize> {
        if self.timer.tick(time.delta()).just_finished() {
            Some(self.next_index())
        } else {
            None
        }
    }

    fn next_index(&mut self) -> usize {
        if self.current == self.indices.len() - 1 {
            self.current = 0;
        } else {
            self.current += 1;
        }
        self.current_idx()
    }
}

#[derive(Component)]
pub struct Animated(Entity);
impl Animated {
    pub fn by(animation_entity: Entity) -> Self {
        Self(animation_entity)
    }
}
