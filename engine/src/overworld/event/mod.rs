use std::collections::HashMap;

use bevy::{log, prelude::*};
use bevy_elf::{AssetResolver, FromDef, HasResolver, PathResolver};
use bevy_entity_lookup::{EntityRef, IntoLookedUp, LookupMap};
use bevy_spawn_phase_events::{InSpawnPhase, LozoAppExt, SpawnPhaseCompleted};

use crate::{
    animation::{Animated, SpriteAnimationAsset},
    asset::{AssetsExt, Phantom},
    overworld::{
        camera::{HasCamera, ZoomWarp},
        character::{CharEnteredTile, CharLeftTile, CharReachedTile},
        lozo::{
            InLozo, Lozo, LozoAsset, LozoCommands, LozoTransition, SpawnOverworldEvents,
            SpawnOverworldObjects,
        },
        tile::TileEdge,
    },
};

pub struct OverworldEventPlugin;

impl Plugin for OverworldEventPlugin {
    fn build(&self, app: &mut App) {
        app.register_spawn_event::<TileEdgeEventsSpawned>()
            .add_observer(spawn_edge_events)
            .add_observer(trigger_actions::<CharLeftTile>)
            .add_observer(trigger_actions::<CharEnteredTile>)
            .add_observer(trigger_actions::<CharReachedTile>)
            .add_observer(on_load_next_lozo)
            .add_observer(on_activate_next_lozo)
            .add_observer(on_unload_next_lozo)
            .add_observer(on_play_sprite_animation)
            .add_observer(on_play_zoom_warp);
    }
}

#[derive(Component, Default)]
pub struct TileEdgeEvents<T: Send + Sync>(
    HashMap<TileEdge, Vec<TileEventActionLookedUp>>,
    Phantom<T>,
);

pub trait TileEdgeEvent {
    fn trigger_entity(&self) -> Entity;
    fn edge(&self) -> &TileEdge;
}

impl<T: Send + Sync> TileEdgeEvents<T> {
    fn new(events: HashMap<TileEdge, Vec<TileEventActionLookedUp>>) -> Self {
        Self(events, Phantom::default())
    }

    pub fn trigger(
        &self,
        trigger: Entity,
        edge: &TileEdge,
        lozo_entity: Entity,
        commands: &mut Commands,
    ) {
        if let Some(actions) = self.0.get(edge) {
            for action in actions {
                action.trigger_event(trigger, lozo_entity, commands);
            }
        }
    }
}

#[derive(FromDef, Debug, Clone, IntoLookedUp)]
pub enum TileEventAction {
    LoadNextLozo {
        next_lozo_id: String,
        after_animation: Option<CameraAnimation>,
    },
    UnloadNextLozo,
    ActivateNextLozo,
    SpriteAnimation {
        sprite_entity: EntityRef,

        #[elf(with_resolver(PathResolver))]
        animation: Handle<SpriteAnimationAsset>,
    },
    CameraAnimation(CameraAnimation),
}

impl TileEventActionLookedUp {
    pub fn trigger_event(&self, trigger: Entity, lozo: Entity, commands: &mut Commands) {
        match self {
            Self::LoadNextLozo {
                next_lozo_id,
                after_animation,
            } => commands.trigger(LoadNextLozo {
                current: lozo,
                next: next_lozo_id.clone(),
                trigger,
                after_animation: after_animation.clone(),
            }),
            Self::SpriteAnimation {
                sprite_entity,
                animation,
            } => commands.trigger(PlaySpriteAnimation {
                sprite_entity: *sprite_entity,
                animation: animation.clone(),
            }),
            Self::ActivateNextLozo => commands.trigger(ActivateNextLozo { trigger }),
            Self::UnloadNextLozo => commands.trigger(UnloadNextLozo { trigger }),
            Self::CameraAnimation(kind) => commands.trigger(PlayCameraAnimation {
                trigger,
                kind: kind.clone(),
            }),
        };
    }
}

#[derive(Event)]
struct TileEdgeEventsSpawned {
    lozo_entity: Entity,
}

impl InSpawnPhase for TileEdgeEventsSpawned {
    type SpawnPhase = SpawnOverworldEvents;

    fn entity(&self) -> Entity {
        self.lozo_entity
    }
}

fn spawn_edge_events(
    event: On<SpawnPhaseCompleted<SpawnOverworldObjects>>,
    mut commands: LozoCommands,
    lozo_query: Query<&Lozo>,
    lozo_assets: Res<Assets<LozoAsset>>,
    lookup_map: Res<LookupMap>,
) -> Result {
    let lozo = lozo_query.get(event.entity())?;
    let lozo_asset = lozo_assets.require_handle(lozo.handle())?;

    commands.entity(event.entity()).insert((
        TileEdgeEvents::<CharLeftTile>::new(
            lozo_asset.char_left_events.into_looked_up(&lookup_map)?,
        ),
        TileEdgeEvents::<CharEnteredTile>::new(
            lozo_asset.char_entered_events.into_looked_up(&lookup_map)?,
        ),
        TileEdgeEvents::<CharReachedTile>::new(
            lozo_asset.char_reached_events.into_looked_up(&lookup_map)?,
        ),
    ));

    commands.trigger(TileEdgeEventsSpawned {
        lozo_entity: event.entity(),
    });

    Ok(())
}

#[derive(Event)]
struct LoadNextLozo {
    current: Entity,
    next: String,
    trigger: Entity,
    after_animation: Option<CameraAnimationLookedUp>,
}

#[derive(Event)]
struct UnloadNextLozo {
    trigger: Entity,
}

#[derive(Event)]
struct ActivateNextLozo {
    trigger: Entity,
}

#[derive(Event)]
struct PlaySpriteAnimation {
    sprite_entity: Entity,
    animation: Handle<SpriteAnimationAsset>,
}

#[derive(Event)]
pub struct PlayCameraAnimation {
    pub trigger: Entity,
    pub kind: CameraAnimationLookedUp,
}

#[derive(FromDef, Debug, Clone, IntoLookedUp)]
pub enum CameraAnimation {
    ZoomWarp { reverse: bool },
}

fn trigger_actions<E: Event + TileEdgeEvent>(
    event: On<E>,
    in_lozo: Query<&InLozo>,
    events: Query<&TileEdgeEvents<E>>,
    mut commands: Commands,
) -> Result {
    let lozo_entity = in_lozo.get(event.trigger_entity())?.entity();

    events.get(lozo_entity)?.trigger(
        event.trigger_entity(),
        event.edge(),
        lozo_entity,
        &mut commands,
    );

    Ok(())
}

fn on_load_next_lozo(
    event: On<LoadNextLozo>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) -> Result {
    commands.spawn(LozoTransition::new(
        event.current,
        asset_server.load(LozoAsset::resolver().resolve(&event.next)?),
        event.trigger,
        event.after_animation.clone(),
    ));
    Ok(())
}

fn on_activate_next_lozo(event: On<ActivateNextLozo>, mut transitions: Query<&mut LozoTransition>) {
    if let Some(mut transition) = transitions
        .iter_mut()
        .find(|transition| transition.entity == event.trigger)
    {
        transition.activate = true;
    } else {
        log::warn!("ActivateNextLozoEvent was triggered, but no transition was found");
    }
}

fn on_unload_next_lozo(
    event: On<UnloadNextLozo>,
    mut commands: Commands,
    transitions: Query<(Entity, &LozoTransition)>,
) {
    if let Some(entity) = transitions
        .iter()
        .find_map(|(e, t)| (t.entity == event.trigger).then_some(e))
    {
        commands.entity(entity).despawn();
    } else {
        log::warn!("no transition for aborting found containing the event trigger");
    }
}

fn on_play_sprite_animation(event: On<PlaySpriteAnimation>, mut commands: Commands) -> Result {
    commands
        .entity(event.sprite_entity)
        .insert(Animated::by(event.animation.clone()));

    Ok(())
}

fn on_play_zoom_warp(
    event: On<PlayCameraAnimation>,
    mut commands: Commands,
    has_camera: Query<&HasCamera>,
) {
    if let Ok(has_camera) = has_camera.get(event.trigger) {
        match event.kind {
            CameraAnimationLookedUp::ZoomWarp { reverse } => {
                commands.trigger(ZoomWarp {
                    camera_entity: has_camera.entity(),
                    reverse,
                });
            }
        }
    } else {
        log::warn!(
            "Camera animation could not be played, cause the triggering entity has no camera"
        );
    }
}
