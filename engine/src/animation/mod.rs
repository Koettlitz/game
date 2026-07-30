use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Deref, DerefMut},
    time::Duration,
};

use bevy::{asset::AssetEventSystems, prelude::*};
use thiserror::Error;

use crate::asset::AssetsExt;
use asset::{AnimationKind, AnimationTimerApi, SpriteAnimationAssetPlugin};

pub use asset::{AnimationTimersAsset, SpriteAnimationAsset};

mod asset;

pub struct SpriteAnimationPlugin;
impl Plugin for SpriteAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TimerMap>()
            .add_plugins(SpriteAnimationAssetPlugin)
            .add_observer(on_remove_animated)
            .add_observer(on_insert_timer)
            .add_observer(on_remove_timer)
            .add_systems(PreUpdate, cleanup_unused_timers)
            .add_systems(Update, update_timers)
            .add_systems(
                PostUpdate,
                (
                    update_changed_sprites,
                    apply_animations.in_set(AnimationUpdate),
                )
                    .chain(),
            )
            .add_systems(
                PreUpdate,
                (hot_reload_animations, hot_reload_timers)
                    .after(AssetEventSystems)
                    .before(cleanup_unused_timers),
            );
    }
}

#[derive(SystemSet, PartialEq, Eq, Clone, Hash, Debug)]
pub struct AnimationUpdate;

#[derive(Component)]
pub struct Animated(Handle<SpriteAnimationAsset>);

impl Deref for Animated {
    type Target = Handle<SpriteAnimationAsset>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Animated {
    pub fn by(animation: Handle<SpriteAnimationAsset>) -> Self {
        Self(animation)
    }
}

#[derive(Component)]
#[relationship(relationship_target = Times)]
struct TimedBy(Entity);

#[derive(Component)]
#[relationship_target(relationship = TimedBy)]
struct Times(Vec<Entity>);

#[derive(Component)]
struct AnimationTimer {
    timer: Timer,
    current: usize,
    frame_count: usize,
    last_applied: Option<usize>,
}

#[derive(Component)]
struct TimerId(String);
impl Deref for TimerId {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Resource, Default)]
struct TimerMap(HashMap<String, Entity>);

impl Deref for TimerMap {
    type Target = HashMap<String, Entity>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TimerMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Component)]
struct SpriteAnimation {
    handle: Handle<SpriteAnimationAsset>,
    frames: Vec<usize>,
    kind: AnimationKind,
}

fn update_changed_sprites(
    mut sprites: Query<(Entity, &mut Sprite, &Animated), Changed<Animated>>,
    timer_map: Res<TimerMap>,
    timer_query: Query<(Entity, &AnimationTimer, &TimerId)>,
    animation_assets: Res<Assets<SpriteAnimationAsset>>,
    timers_asset: Res<Assets<AnimationTimersAsset>>,
    mut commands: Commands,
) -> Result<()> {
    for (sprite_entity, mut sprite, Animated(handle)) in &mut sprites {
        let asset = animation_assets.require_handle(handle)?;
        let spawned_timer = spawn_timer(
            &animation_assets,
            handle,
            &timer_map,
            &timer_query,
            &timers_asset,
            &mut commands,
        )?;
        commands.entity(sprite_entity).insert((
            SpriteAnimation {
                handle: handle.clone(),
                frames: asset.frames.clone(),
                kind: asset.kind,
            },
            TimedBy(spawned_timer.entity),
        ));
        if let Some(ref mut atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = asset.frames[spawned_timer.current_idx];
        } else {
            warn!("animated sprite had no texture atlas to animate on");
        }
    }

    Ok(())
}

fn on_remove_animated(event: On<Remove, Animated>, mut commands: Commands) {
    let entity = event.entity;
    commands.queue(move |world: &mut World| {
        if let Ok(mut entity) = world.get_entity_mut(entity) {
            entity.remove::<(SpriteAnimation, TimedBy)>();
        }
    });
}

fn on_insert_timer(
    event: On<Insert, TimerId>,
    timers: Query<&TimerId>,
    mut timer_map: ResMut<TimerMap>,
) -> Result<()> {
    let TimerId(id) = timers.get(event.entity)?;
    timer_map.insert(id.clone(), event.entity);
    Ok(())
}

fn on_remove_timer(
    event: On<Remove, TimerId>,
    timers: Query<&TimerId>,
    mut timer_map: ResMut<TimerMap>,
) -> Result<()> {
    let TimerId(id) = timers.get(event.entity)?;
    timer_map.remove(id);
    Ok(())
}

#[allow(clippy::type_complexity)]
fn cleanup_unused_timers(
    timers: Query<(Entity, &Times), (Changed<Times>, With<AnimationTimer>)>,
    mut commands: Commands,
) {
    for (entity, Times(animations)) in &timers {
        if animations.is_empty() {
            commands.entity(entity).despawn();
        }
    }
}

fn update_timers(mut timers: Query<&mut AnimationTimer>, time: Res<Time>) {
    for mut timer in &mut timers {
        if timer.timer.tick(time.delta()).just_finished() {
            timer.current = (timer.current + 1) % timer.frame_count;
        }
    }
}

fn apply_animations(
    mut timers: Query<(&mut AnimationTimer, &Times), Changed<AnimationTimer>>,
    mut animations: Query<(&mut SpriteAnimation, &mut Sprite), With<TimedBy>>,
    mut commands: Commands,
) -> Result<()> {
    for (mut timer, Times(entities)) in &mut timers {
        if let Some(last_applied) = timer.last_applied
            && last_applied == timer.current
        {
            continue;
        }

        for entity in entities {
            let (mut animation, mut sprite) = animations.get_mut(*entity)?;
            if let Some(ref mut atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = animation.frames[timer.current];
            } else {
                warn!("animated sprite had no texture atlas to animate on");
            }
            if let AnimationKind::Once = &mut animation.kind
                && timer.current == animation.frames.len() - 1
            {
                commands.entity(*entity).remove::<Animated>();
            }
        }

        timer.last_applied = Some(timer.current);
    }

    Ok(())
}

fn hot_reload_timers(
    mut message_reader: MessageReader<AssetEvent<AnimationTimersAsset>>,
    assets: Res<Assets<AnimationTimersAsset>>,
    mut query: Query<(&mut AnimationTimer, &TimerId)>,
) -> Result<()> {
    for msg in message_reader.read() {
        let AssetEvent::LoadedWithDependencies { id } = msg else {
            continue;
        };
        let asset = assets.require(*id)?;
        for (mut timer, TimerId(id)) in &mut query {
            let Some(duration) = asset.get(id) else {
                warn!(
                    "unexpected removal of used animation timer - timer keeps running until unused"
                );
                continue;
            };
            timer.timer.set_duration(Duration::from_millis(*duration));
        }
    }
    Ok(())
}

#[allow(clippy::type_complexity)]
fn hot_reload_animations(
    mut message_reader: MessageReader<AssetEvent<SpriteAnimationAsset>>,
    assets: Res<Assets<SpriteAnimationAsset>>,
    mut animations: Query<(Entity, &mut SpriteAnimation)>,
    timer_map: Res<TimerMap>,
    mut timers: ParamSet<(
        Query<(Entity, &AnimationTimer, &TimerId)>,
        Query<(Entity, &mut AnimationTimer, &TimerId)>,
    )>,
    timers_asset: Res<Assets<AnimationTimersAsset>>,
    mut commands: Commands,
) -> Result<()> {
    for msg in message_reader.read() {
        let AssetEvent::LoadedWithDependencies { id } = msg else {
            continue;
        };
        for (entity, mut animation) in &mut animations {
            if animation.handle.id() != *id {
                continue;
            }
            let asset = assets.require(*id)?;
            animation.frames = asset.frames.clone();
            animation.kind = asset.kind;

            let spawned_timer = spawn_timer(
                &assets,
                &animation.handle,
                &timer_map,
                &timers.p0(),
                &timers_asset,
                &mut commands,
            )?;
            if spawned_timer.existed {
                let mut query = timers.p1();
                let (_, mut timer, _) = query.get_mut(spawned_timer.entity)?;
                timer.frame_count = asset.frames.len();
                timer.current %= timer.frame_count;
            }
            commands
                .entity(entity)
                .insert(TimedBy(spawned_timer.entity));
        }
    }
    Ok(())
}

fn spawn_timer(
    assets: &Assets<SpriteAnimationAsset>,
    handle: &Handle<SpriteAnimationAsset>,
    timer_map: &TimerMap,
    timer_query: &Query<(Entity, &AnimationTimer, &TimerId)>,
    timers_asset: &Assets<AnimationTimersAsset>,
    commands: &mut Commands,
) -> Result<TimerSpawnResult> {
    let asset = assets.require_handle(handle)?;
    match &asset.timer {
        AnimationTimerApi::FrameDuration(frame_duration) => {
            let entity = commands
                .spawn(AnimationTimer {
                    timer: Timer::new(*frame_duration, TimerMode::Repeating),
                    current: 0,
                    frame_count: asset.frames.len(),
                    last_applied: None,
                })
                .id();
            Ok(TimerSpawnResult {
                entity,
                current_idx: 0,
                existed: false,
            })
        }
        AnimationTimerApi::TimerGroup(timer_id) => {
            if let Some((timer_entity, timer, _)) = timer_map
                .get(timer_id)
                .map(|e| timer_query.get(*e))
                .transpose()?
            {
                Ok(TimerSpawnResult {
                    entity: timer_entity,
                    current_idx: timer.current,
                    existed: true,
                })
            } else {
                // expects that only one AnimationTimersAsset is ever present
                // maybe change this to a Resource in the future
                let timers_asset = timers_asset
                    .iter()
                    .next()
                    .ok_or("missing AnimationTimersAsset")?
                    .1;
                let frame_duration = Duration::from_millis(
                    *timers_asset
                        .get(timer_id)
                        .ok_or_else(|| format!("missing animation timer \"{timer_id}\""))?,
                );
                let entity = commands
                    .spawn((
                        AnimationTimer {
                            timer: Timer::new(frame_duration, TimerMode::Repeating),
                            current: 0,
                            frame_count: asset.frames.len(),
                            last_applied: None,
                        },
                        TimerId(timer_id.clone()),
                    ))
                    .id();
                Ok(TimerSpawnResult {
                    entity,
                    current_idx: 0,
                    existed: false,
                })
            }
        }
    }
}

struct TimerSpawnResult {
    entity: Entity,
    current_idx: usize,
    existed: bool,
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
