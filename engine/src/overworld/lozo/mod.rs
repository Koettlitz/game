use std::{
    collections::HashSet,
    ops::{Deref, DerefMut},
};

use crate::overworld::{
    camera::{CameraOf, HasCamera},
    object::ObjectSpriteLookup,
    tile::{CameraAnimation, PlayCameraAnimation},
};
use bevy::{
    asset::RecursiveDependencyLoadState, camera::visibility::RenderLayers,
    ecs::system::SystemParam, log, prelude::*,
};
use bevy_elf::{AppExt, AssetResolver, HasResolver, ResolveError};

pub use asset::*;
mod asset;

pub struct LozoPlugin;
impl Plugin for LozoPlugin {
    fn build(&self, app: &mut App) {
        app.register_required_components::<Sprite, NeedsRenderLayers>()
            .init_ron_asset::<LozoAsset>()
            .add_systems(
                PostUpdate,
                (
                    (check_transitions, init_loaded_lozo).chain(),
                    attach_render_layers,
                ),
            )
            .add_observer(on_lozo_added)
            .add_observer(transition_entities)
            .add_observer(commit_transition);
    }
}

#[derive(Component, Default)]
#[require(Visibility, Transform, ObjectSpriteLookup)]
pub struct Lozo(Handle<LozoAsset>);

impl Lozo {
    pub fn new(handle: Handle<LozoAsset>) -> Self {
        Self(handle)
    }

    pub fn from_id(id: &str, asset_server: &AssetServer) -> Result<Self, ResolveError> {
        Ok(Self(asset_server.load(LozoAsset::resolver().resolve(id)?)))
    }

    pub fn handle(&self) -> &Handle<LozoAsset> {
        &self.0
    }
}

#[derive(Component)]
pub struct InLozo(Entity);

impl InLozo {
    pub fn entity(&self) -> Entity {
        self.0
    }
}

#[derive(SystemParam)]
pub struct LozoCommands<'w, 's> {
    commands: Commands<'w, 's>,
    query: Query<'w, 's, Entity, With<Lozo>>,
}

impl<'w, 's> LozoCommands<'w, 's> {
    pub fn spawn_into_lozo(
        &mut self,
        lozo: Entity,
        bundle: impl Bundle,
    ) -> Result<EntityCommands<'_>> {
        let entity = self.commands.spawn((bundle, InLozo(lozo))).id();
        self.commands
            .entity(self.query.get(lozo)?)
            .add_child(entity);

        self.commands.queue(move |world: &mut World| {
            let mut children: Vec<Entity> = world
                .get::<Children>(entity)
                .map(|c| c.to_vec())
                .unwrap_or_default();

            while let Some(child) = children.pop() {
                if let Some(grandchildren) = world.get::<Children>(child) {
                    children.extend(grandchildren.iter());
                }
                world.entity_mut(child).insert(InLozo(lozo));
            }
        });

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

#[derive(EntityEvent)]
pub struct InitLozo(#[event_target] Entity);

impl InitLozo {
    pub fn entity(&self) -> Entity {
        self.0
    }
}

#[derive(Component)]
pub struct LozoTransition {
    from: Entity,
    pub to: Handle<LozoAsset>,
    pub entity: Entity,
    pub after_animation: Option<CameraAnimation>,
    pub activate: bool,
}

impl LozoTransition {
    pub fn new(
        from: Entity,
        to: Handle<LozoAsset>,
        entity: Entity,
        after_animation: Option<CameraAnimation>,
    ) -> Self {
        Self {
            from,
            to,
            entity,
            after_animation,
            activate: false,
        }
    }

    pub fn from(&self) -> Entity {
        self.from
    }
}

#[derive(Component)]
struct Loading;

fn check_transitions(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    transitions: Query<(Entity, &LozoTransition)>,
    lozos: Query<(Entity, &Lozo)>,
    render_layers: Query<&RenderLayers>,
) {
    for (transition_entity, transition) in transitions {
        match asset_server.get_recursive_dependency_load_state(transition.to.id()) {
            Some(RecursiveDependencyLoadState::Loaded) => {
                if !transition.activate {
                    continue;
                }

                let target_lozo = lozos
                    .iter()
                    .find_map(|(e, lozo)| (lozo.handle().id() == transition.to.id()).then_some(e))
                    .unwrap_or_else(|| {
                        let render_layer = find_free_lozo_render_layer(
                            render_layers.iter().flat_map(|rl| rl.iter()),
                        );
                        commands
                            .spawn((
                                Lozo(transition.to.clone()),
                                RenderLayers::layer(render_layer),
                            ))
                            .id()
                    });

                commands.trigger(TransitionEntities {
                    target_lozo,
                    transition: transition_entity,
                });
            }
            Some(RecursiveDependencyLoadState::Failed(e)) => {
                log::error!("Failed loading lozo - {e}");
                commands.entity(transition_entity).despawn();
            }
            _ => {}
        }
    }
}

#[derive(Event)]
struct TransitionEntities {
    target_lozo: Entity,
    transition: Entity,
}

#[allow(clippy::too_many_arguments)]
fn transition_entities(
    event: On<TransitionEntities>,
    render_layers: Query<&RenderLayers, (Without<InLozo>, Without<CameraOf>)>,
    transitions: Query<&LozoTransition>,
    in_lozo_entities: Query<&Children, With<InLozo>>,
    mut render_layers_in_lozo: Query<&mut RenderLayers, (With<InLozo>, Without<CameraOf>)>,
    mut camera_render_layers: Query<&mut RenderLayers, With<CameraOf>>,
    has_camera_query: Query<&HasCamera, With<InLozo>>,
    mut commands: Commands,
) -> Result {
    let lozo_render_layers = render_layers.get(event.target_lozo)?;
    let transition = transitions.get(event.transition)?;

    commands
        .entity(transition.entity)
        .insert(ChildOf(event.target_lozo));

    let mut children = vec![transition.entity];
    while let Some(e) = children.pop() {
        if let Ok(grandchildren) = in_lozo_entities.get(e) {
            children.extend(grandchildren.iter());
        }

        commands.entity(e).insert(InLozo(event.target_lozo));

        if let Ok(mut render_layers) = render_layers_in_lozo.get_mut(e) {
            *render_layers = lozo_render_layers.clone();
        }

        if let Ok(has_camera) = has_camera_query.get(e) {
            *camera_render_layers.get_mut(has_camera.entity())? = lozo_render_layers.clone();
        }
    }

    commands.trigger(CommitTransition(event.transition));
    Ok(())
}

#[derive(Event)]
struct CommitTransition(Entity);

fn commit_transition(
    event: On<CommitTransition>,
    transitions: Query<(Entity, &LozoTransition)>,
    cameras: Query<&InLozo, With<HasCamera>>,
    mut commands: Commands,
) -> Result {
    let (transition_entity, transition) = transitions.get(event.0)?;

    if let Some(animation) = &transition.after_animation {
        commands.trigger(PlayCameraAnimation {
            trigger: transition.entity,
            kind: animation.clone(),
        });
    }

    if !cameras
        .iter()
        .any(|in_lozo| in_lozo.entity() == transition.from)
    {
        commands.entity(transition.from).despawn();
    }

    commands.entity(transition_entity).despawn();

    Ok(())
}

fn on_lozo_added(event: On<Add, Lozo>, mut commands: Commands) {
    commands.entity(event.entity).insert(Loading);
}

fn init_loaded_lozo(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    query: Query<(Entity, &Lozo, Option<&RenderLayers>), With<Loading>>,
    render_layers: Query<&RenderLayers>,
) {
    let mut used_render_layers = Vec::new();
    for (entity, Lozo(handle), lozo_render_layers) in query {
        match asset_server.get_recursive_dependency_load_state(handle.id()) {
            Some(RecursiveDependencyLoadState::Loaded) => {
                let mut entity_commands = commands.entity(entity);

                if lozo_render_layers.is_none() {
                    let render_layer = find_free_lozo_render_layer(
                        render_layers
                            .iter()
                            .flat_map(|rl| rl.iter())
                            .chain(used_render_layers.iter().copied()),
                    );

                    used_render_layers.push(render_layer);
                    entity_commands.insert(RenderLayers::layer(render_layer));
                }

                entity_commands.remove::<Loading>();
                commands.trigger(InitLozo(entity));
            }
            Some(RecursiveDependencyLoadState::Failed(e)) => {
                log::error!("Failed loading lozo - {e}");
                commands.entity(entity).despawn();
            }
            _ => {}
        }
    }
}

fn find_free_lozo_render_layer(layers: impl Iterator<Item = usize>) -> usize {
    let taken: HashSet<usize> = layers.collect();
    (1..).find(|l| !taken.contains(l)).unwrap()
}

#[derive(Component, Default)]
struct NeedsRenderLayers;

fn attach_render_layers(
    need_render_layers: Query<Entity, (With<NeedsRenderLayers>, Without<RenderLayers>)>,
    parents: Query<&ChildOf>,
    render_layers: Query<&RenderLayers>,
    mut commands: Commands,
) {
    for entity in need_render_layers {
        let mut current = entity;

        while let Ok(child_of) = parents.get(current) {
            if let Ok(render_layers) = render_layers.get(child_of.0) {
                commands.entity(entity).insert(render_layers.clone());
                break;
            } else {
                current = child_of.0;
            }
        }
    }
}
