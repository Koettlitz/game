use std::ops::{Deref, DerefMut};

use crate::{asset::overworld::lozo::LozoAsset, overworld::object::ObjectSpriteLookup};
use bevy::{asset::RecursiveDependencyLoadState, ecs::system::SystemParam, log, prelude::*};
use bevy_elf::{AssetResolver, HasResolver, RonAssetPlugin};

pub struct LozoPlugin;
impl Plugin for LozoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RonAssetPlugin::<LozoAsset>::default())
            .add_systems(
                PostUpdate,
                (
                    detect_lozo_transition,
                    (abort_transition, change_transition_target).before(detect_lozo_loaded),
                    detect_lozo_loaded,
                    activate_switch.after(detect_lozo_loaded),
                ),
            )
            .add_systems(
                First,
                (
                    despawn_lozo_entities,
                    spawn_next_lozo,
                    commit_lozo_transition,
                )
                    .chain(),
            );
    }
}

#[derive(Component, Default)]
#[require(Visibility, Transform, ObjectSpriteLookup, NextLozo, LozoState)]
pub struct Lozo(Handle<LozoAsset>);

impl Lozo {
    pub fn handle(&self) -> &Handle<LozoAsset> {
        &self.0
    }
}

#[derive(Component)]
pub struct InLozo(Entity);

impl Deref for InLozo {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Component)]
pub struct SurviveLozoTransition;

#[derive(SystemParam)]
pub struct LozoCommands<'w, 's> {
    commands: Commands<'w, 's>,
    query: Query<'w, 's, Entity, With<Lozo>>,
}

impl<'w, 's> LozoCommands<'w, 's> {
    pub fn spawn_lozo(&mut self, id: String) {
        self.commands.spawn(NextLozo {
            id: Some(id),
            ready: None,
            auto_activate: true,
        });
    }

    pub fn spawn_into_lozo(
        &mut self,
        lozo: Entity,
        bundle: impl Bundle,
    ) -> Result<EntityCommands<'_>> {
        let entity = self.commands.spawn(bundle).insert(InLozo(lozo)).id();
        self.commands
            .entity(self.query.get(lozo)?)
            .add_child(entity);
        Ok(self.commands.entity(entity))
    }
}

impl<'w, 's> Deref for LozoCommands<'w, 's> {
    type Target = Commands<'w, 's>;

    fn deref(&self) -> &Self::Target {
        &self.commands
    }
}

impl<'w, 's> DerefMut for LozoCommands<'w, 's> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.commands
    }
}

#[derive(Component, Default)]
pub struct NextLozo {
    id: Option<String>,
    ready: Option<ReadyNextLozo>,
    pub auto_activate: bool,
}

impl NextLozo {
    pub fn set(&mut self, target: String) {
        if let Some(id) = self.id.as_ref() {
            if *id == target {
                return;
            }
        }
        self.reset();
        self.id = Some(target);
    }

    pub fn ready(&mut self) -> Option<&mut ReadyNextLozo> {
        self.ready.as_mut()
    }

    pub fn reset(&mut self) {
        self.id = None;
        self.ready = None;
        self.auto_activate = false;
    }
}

#[derive(Default)]
pub struct ReadyNextLozo {
    activate: bool,
}

impl ReadyNextLozo {
    pub fn activate(&mut self) {
        self.activate = true;
    }
}

#[derive(EntityEvent)]
pub struct LozoSpawned(#[event_target] Entity);

impl LozoSpawned {
    pub fn entity(&self) -> Entity {
        self.0
    }
}

#[derive(Component)]
struct LozoTransition {
    next_lozo: String,
    asset_handle: Handle<LozoAsset>,
}

fn detect_lozo_transition(
    lozo_query: Query<
        (Entity, &NextLozo, Option<&mut LozoState>),
        (Without<LozoTransition>, Changed<NextLozo>),
    >,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) -> Result<()> {
    for (entity, next_lozo, state) in lozo_query {
        let Some(next_lozo) = next_lozo.id.as_ref() else {
            continue;
        };
        log::info!("loading requested next lozo {next_lozo}");
        let asset_path = <LozoAsset as HasResolver>::resolver().resolve(next_lozo)?;
        commands.entity(entity).insert(LozoTransition {
            next_lozo: next_lozo.to_string(),
            asset_handle: asset_server.load(asset_path),
        });
        if let Some(mut state) = state {
            *state = LozoState::LoadingLozoAsset;
        } else {
            commands.entity(entity).insert(LozoState::LoadingLozoAsset);
        }
    }
    Ok(())
}

fn change_transition_target(
    lozo_query: Query<(&NextLozo, &mut LozoState, &mut LozoTransition), Changed<NextLozo>>,
    asset_server: Res<AssetServer>,
) -> Result<()> {
    for (next_lozo, mut state, mut transition) in lozo_query {
        let Some(id) = next_lozo.id.as_ref() else {
            continue;
        };
        if id != &transition.next_lozo {
            log::info!(
                "next lozo changed from {} to {id} - loading {id} now instead",
                transition.next_lozo
            );
            transition.next_lozo = id.to_string();
            let asset_path = <LozoAsset as HasResolver>::resolver().resolve(id)?;
            transition.asset_handle = asset_server.load(asset_path);
            if !matches!(*state, LozoState::LoadingLozoAsset) {
                *state = LozoState::LoadingLozoAsset;
            }
        }
    }

    Ok(())
}

fn abort_transition(
    lozo_query: Query<(Entity, &NextLozo, &mut LozoState), Changed<NextLozo>>,
    mut commands: Commands,
) {
    for (entity, next_lozo, mut state) in lozo_query {
        if next_lozo.id.is_none() {
            log::info!("unloading next lozo");
            commands.entity(entity).remove::<LozoTransition>();
            *state = LozoState::Initialized;
        }
    }
}

fn detect_lozo_loaded(
    lozo_query: Query<(Entity, &mut NextLozo, &mut LozoState, &LozoTransition)>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    for (entity, mut next_lozo, mut state, transition) in lozo_query {
        if *state != LozoState::LoadingLozoAsset {
            continue;
        }
        match asset_server.recursive_dependency_load_state(transition.asset_handle.id()) {
            RecursiveDependencyLoadState::Loaded => {
                *state = LozoState::NextReady;
                next_lozo.ready = Some(ReadyNextLozo {
                    activate: next_lozo.auto_activate,
                });
            }
            RecursiveDependencyLoadState::Failed(e) => {
                error!("failed to load lozo: \"{e}\"");
                commands.entity(entity).remove::<LozoTransition>();
                next_lozo.reset();
                *state = LozoState::Initialized;
            }
            _ => {}
        }
    }
}

fn activate_switch(lozo_query: Query<(&mut NextLozo, &mut LozoState), With<LozoTransition>>) {
    for (mut next_lozo, mut state) in lozo_query {
        if let Some(ready) = next_lozo.ready() {
            if ready.activate {
                *state = LozoState::Switching;
                next_lozo.reset();
            }
        }
    }
}

fn despawn_lozo_entities(
    mut commands: Commands,
    lozo_query: Query<(&LozoState, &Children), With<LozoTransition>>,
    lozo_entities: Query<Entity, Without<SurviveLozoTransition>>,
) {
    for (state, children) in lozo_query {
        if *state != LozoState::Switching {
            continue;
        }

        for child in children {
            if lozo_entities.contains(*child) {
                commands.entity(*child).despawn();
            }
        }
    }
}

fn spawn_next_lozo(
    mut commands: Commands,
    lozo_query: Query<(Entity, Option<&mut Lozo>, &LozoState, &LozoTransition)>,
) {
    for (entity, lozo, state, transition) in lozo_query {
        if *state == LozoState::Switching {
            if let Some(mut lozo) = lozo {
                lozo.0 = transition.asset_handle.clone();
            } else {
                commands
                    .entity(entity)
                    .insert(Lozo(transition.asset_handle.clone()));
            }
            commands.trigger(LozoSpawned(entity));
        }
    }
}

fn commit_lozo_transition(
    lozo_query: Query<(Entity, &mut LozoState), With<LozoTransition>>,
    mut commands: Commands,
) {
    for (entity, mut state) in lozo_query {
        if *state == LozoState::Switching {
            commands.entity(entity).remove::<LozoTransition>();
            *state = LozoState::Initialized;
        }
    }
}

#[derive(Component, Debug, PartialEq, Eq, Default)]
pub enum LozoState {
    #[default]
    LoadingLozoAsset,
    NextReady,
    Switching,
    Initialized,
}
