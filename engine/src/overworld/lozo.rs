use std::ops::{Deref, DerefMut};

use crate::{asset::overworld::lozo::LozoAsset, overworld::object::ObjectSpriteLookup};
use bevy::{asset::RecursiveDependencyLoadState, ecs::system::SystemParam, log, prelude::*};
use bevy_elf::{AssetResolver, HasResolver, RonAssetPlugin};

pub struct LozoPlugin;
impl Plugin for LozoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RonAssetPlugin::<LozoAsset>::default())
            .init_resource::<NextLozo>()
            .init_state::<LozoState>()
            .add_systems(
                PostUpdate,
                (
                    detect_lozo_transition
                        .run_if(resource_changed::<NextLozo>)
                        .run_if(in_state(LozoState::Default)),
                    (abort_transition, change_transition_target)
                        .before(detect_lozo_loaded)
                        .run_if(
                            resource_changed::<NextLozo>.and_then(
                                in_state(LozoState::LoadingLozoAsset)
                                    .or_else(in_state(LozoState::NextReady)),
                            ),
                        ),
                    detect_lozo_loaded.run_if(in_state(LozoState::LoadingLozoAsset)),
                    activate_switch.after(detect_lozo_loaded).run_if(
                        in_state(LozoState::LoadingLozoAsset)
                            .or_else(in_state(LozoState::NextReady)),
                    ),
                ),
            )
            .add_systems(
                OnEnter(LozoState::Switching),
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
#[require(Visibility, Transform, ObjectSpriteLookup)]
pub struct Lozo(Handle<LozoAsset>);

impl Lozo {
    pub fn handle(&self) -> &Handle<LozoAsset> {
        &self.0
    }
}

#[derive(SystemParam)]
pub struct LozoCommands<'w, 's> {
    commands: Commands<'w, 's>,
    query: Single<'w, 's, Entity, With<Lozo>>,
}

impl<'w, 's> LozoCommands<'w, 's> {
    pub fn spawn_into_lozo(&mut self, bundle: impl Bundle) -> EntityCommands<'_> {
        let entity = self.commands.spawn(bundle).id();
        self.commands.entity(*self.query).add_child(entity);
        self.commands.entity(entity)
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

#[derive(Resource, Default)]
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

#[derive(Resource)]
struct LozoTransition {
    next_lozo: String,
    asset_handle: Handle<LozoAsset>,
}

fn detect_lozo_transition(
    next_lozo: ResMut<NextLozo>,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<LozoState>>,
    mut commands: Commands,
) -> Result<()> {
    let Some(next_lozo) = next_lozo.id.as_ref() else {
        return Ok(());
    };

    log::info!("loading requested next lozo {next_lozo}");
    let asset_path = <LozoAsset as HasResolver>::resolver().resolve(next_lozo)?;
    commands.insert_resource(LozoTransition {
        next_lozo: next_lozo.to_string(),
        asset_handle: asset_server.load(asset_path),
    });
    next_state.set(LozoState::LoadingLozoAsset);
    Ok(())
}

fn change_transition_target(
    next_lozo: Res<NextLozo>,
    mut transition: ResMut<LozoTransition>,
    asset_server: Res<AssetServer>,
    current_state: Res<State<LozoState>>,
    mut next_state: ResMut<NextState<LozoState>>,
) -> Result<()> {
    let Some(id) = next_lozo.id.as_ref() else {
        return Ok(());
    };

    if id != &transition.next_lozo {
        log::info!(
            "next lozo changed from {} to {id} - loading {id} now instead",
            transition.next_lozo
        );
        transition.next_lozo = id.to_string();
        let asset_path = <LozoAsset as HasResolver>::resolver().resolve(id)?;
        transition.asset_handle = asset_server.load(asset_path);
        if !matches!(current_state.get(), LozoState::LoadingLozoAsset) {
            next_state.set(LozoState::LoadingLozoAsset);
        }
    }

    Ok(())
}

fn abort_transition(
    next_lozo: Res<NextLozo>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<LozoState>>,
) {
    if next_lozo.id.is_none() {
        log::info!("unloading next lozo");
        commands.remove_resource::<LozoTransition>();
        next_state.set(LozoState::Default);
    }
}

fn detect_lozo_loaded(
    asset_server: Res<AssetServer>,
    transition: Res<LozoTransition>,
    mut next_state: ResMut<NextState<LozoState>>,
    mut next_lozo: ResMut<NextLozo>,
    mut commands: Commands,
) {
    match asset_server.recursive_dependency_load_state(transition.asset_handle.id()) {
        RecursiveDependencyLoadState::Loaded => {
            next_state.set(LozoState::NextReady);
            next_lozo.ready = Some(ReadyNextLozo {
                activate: next_lozo.auto_activate,
            });
        }
        RecursiveDependencyLoadState::Failed(e) => {
            error!("failed to load lozo: \"{e}\"");
            commands.remove_resource::<LozoTransition>();
            next_lozo.reset();
            next_state.set(LozoState::Default);
        }
        _ => {}
    }
}

fn activate_switch(mut next_lozo: ResMut<NextLozo>, mut next_state: ResMut<NextState<LozoState>>) {
    if let Some(ready) = next_lozo.ready() {
        if ready.activate {
            next_state.set(LozoState::Switching);
            next_lozo.reset();
        }
    }
}

fn despawn_lozo_entities(mut commands: Commands, current: Single<Entity, With<Lozo>>) {
    commands.entity(*current).despawn_children();
}

fn spawn_next_lozo(
    mut commands: Commands,
    transition: Option<Res<LozoTransition>>,
    mut current: Query<(Entity, &mut Lozo)>,
) {
    let Some(transition) = transition else {
        return;
    };
    let current = match current.single_mut() {
        Ok((current_entity, mut current)) => {
            current.0 = transition.asset_handle.clone();
            current_entity
        }
        Err(_) => commands.spawn(Lozo(transition.asset_handle.clone())).id(),
    };
    commands.trigger(LozoSpawned(current));
}

// TODO: Spawn CharTileEvents

fn commit_lozo_transition(mut commands: Commands, mut next_state: ResMut<NextState<LozoState>>) {
    commands.remove_resource::<LozoTransition>();
    next_state.set(LozoState::Default);
}

#[derive(States, Default, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum LozoState {
    #[default]
    Default,
    LoadingLozoAsset,
    NextReady,
    Switching,
}
