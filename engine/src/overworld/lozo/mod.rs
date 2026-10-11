use std::{
    collections::HashSet,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::{
    asset::Phantom,
    overworld::{
        camera::{CameraOf, HasCamera},
        event::{CameraAnimationLookedUp, PlayCameraAnimation},
    },
};
use bevy::{
    asset::RecursiveDependencyLoadState, camera::visibility::RenderLayers,
    ecs::system::SystemParam, log, prelude::*,
};

pub use asset::*;
use bevy_elf::{AppExt, AssetResolver, HasResolver};
use bevy_entity_lookup::EntityId;
use bevy_event_cascade::{StageCompleted, StageProgress};
mod asset;

pub struct LozoPluginWithSource<S>(Phantom<S>);

#[derive(Default)]
pub struct LozoPlugin;

impl Plugin for LozoPlugin {
    fn build(&self, app: &mut App) {
        app.init_ron_asset::<LozoAsset>();
        LozoPluginWithSource::<LozoAssetSource>::default().build(app);
    }
}

impl<S> Default for LozoPluginWithSource<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<S: LozoSource> Plugin for LozoPluginWithSource<S> {
    fn build(&self, app: &mut App) {
        app.register_required_components::<Sprite, NeedsRenderLayers>()
            .add_systems(
                PostUpdate,
                (
                    (check_transitions::<S>, init_loaded_lozo::<S>).chain(),
                    attach_render_layers,
                ),
            )
            .add_observer(propagate_in_lozo)
            .add_observer(on_lozo_added::<S>)
            .add_observer(transition_entities::<S>)
            .add_observer(commit_transition::<S>)
            .add_observer(cleanup_lookup_data);
    }
}

#[derive(Component, Default)]
#[require(Visibility, Transform)]
pub struct Lozo;

pub trait LozoSource: Component + Clone {
    type Asset: Asset;

    fn handle(&self) -> &Handle<Self::Asset>;
}

#[derive(Component, Clone)]
pub struct LozoAssetSource(Handle<LozoAsset>);

impl LozoAssetSource {
    pub fn from_id(id: &str, asset_server: &AssetServer) -> Result<Self> {
        Ok(Self(asset_server.load(LozoAsset::resolver().resolve(id)?)))
    }
}

impl LozoSource for LozoAssetSource {
    type Asset = LozoAsset;

    fn handle(&self) -> &Handle<Self::Asset> {
        &self.0
    }
}

impl From<Handle<LozoAsset>> for LozoAssetSource {
    fn from(value: Handle<LozoAsset>) -> Self {
        Self(value)
    }
}

fn on_lozo_added<S: LozoSource>(event: On<Add<S>>, mut commands: Commands) {
    commands.entity(event.entity).insert((Lozo, Loading));
}

fn init_loaded_lozo<S>(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    query: Query<(Entity, &S, Option<&RenderLayers>), (With<Loading>, With<Lozo>)>,
    render_layers: Query<&RenderLayers>,
) where
    S: LozoSource,
{
    let mut used_render_layers = Vec::new();
    for (entity, source, lozo_render_layers) in query {
        match asset_server.get_recursive_dependency_load_state(source.handle().id()) {
            None | Some(RecursiveDependencyLoadState::Loaded) => {
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

                entity_commands.insert((
                    StageProgress::<SpawnOverworldEvents>::default(),
                    StageProgress::<SpawnOverworldObjects>::default(),
                ));
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

fn propagate_in_lozo(
    event: On<Insert<ChildOf>>,
    child_of: Query<&ChildOf>,
    in_lozo_entities: Query<&InLozo, With<Children>>,
    mut commands: Commands,
) -> Result {
    if let Ok(in_lozo) = in_lozo_entities.get(child_of.get(event.entity)?.parent()) {
        commands.entity(event.entity).insert(InLozo(in_lozo.0));
    };

    Ok(())
}

#[derive(EntityEvent)]
pub struct InitLozo(#[event_target] Entity);

#[derive(PartialEq, Eq, Hash)]
pub struct SpawnOverworldObjects;

#[derive(PartialEq, Eq, Hash)]
pub struct SpawnOverworldEvents;

fn cleanup_lookup_data(
    event: On<StageCompleted<SpawnOverworldEvents>>,
    entity_ids: Query<(Entity, &InLozo), With<EntityId>>,
    mut commands: Commands,
) {
    for e in entity_ids
        .iter()
        .filter_map(|(e, in_lozo)| (in_lozo.0 == event.event_target()).then_some(e))
    {
        commands.entity(e).remove::<EntityId>();
    }
}

#[derive(Component)]
pub struct LozoTransitionWithAsset<S: LozoSource> {
    from: Entity,
    pub to: S,
    pub entity: Entity,
    pub after_animation: Option<CameraAnimationLookedUp>,
    pub activate: bool,
}

pub type LozoTransition = LozoTransitionWithAsset<LozoAssetSource>;

impl<S: LozoSource> LozoTransitionWithAsset<S> {
    pub fn new(
        from: Entity,
        to: S,
        entity: Entity,
        after_animation: Option<CameraAnimationLookedUp>,
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

fn check_transitions<S: LozoSource>(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    transitions: Query<(Entity, &LozoTransitionWithAsset<S>)>,
    lozos: Query<(Entity, &S), With<Lozo>>,
    render_layers: Query<&RenderLayers>,
) {
    for (transition_entity, transition) in transitions {
        match asset_server.get_recursive_dependency_load_state(transition.to.handle().id()) {
            Some(RecursiveDependencyLoadState::Loaded) => {
                if !transition.activate {
                    continue;
                }

                let target_lozo = lozos
                    .iter()
                    .find_map(|(e, source)| {
                        (source.handle().id() == transition.to.handle().id()).then_some(e)
                    })
                    .unwrap_or_else(|| {
                        let render_layer = find_free_lozo_render_layer(
                            render_layers.iter().flat_map(|rl| rl.iter()),
                        );
                        commands
                            .spawn((
                                Lozo,
                                transition.to.clone(),
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
fn transition_entities<S: LozoSource>(
    event: On<TransitionEntities>,
    render_layers: Query<&RenderLayers, (Without<InLozo>, Without<CameraOf>)>,
    transitions: Query<&LozoTransitionWithAsset<S>>,
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

fn commit_transition<S: LozoSource>(
    event: On<CommitTransition>,
    transitions: Query<(Entity, &LozoTransitionWithAsset<S>)>,
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
