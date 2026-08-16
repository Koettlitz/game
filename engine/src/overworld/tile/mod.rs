use std::ops;
use std::{collections::HashMap, fmt::Debug};

use bevy::log;
use bevy::prelude::*;
use bevy_elf::{AssetResolver, FromDef, HasResolver, PathResolver};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::overworld::camera::{HasCamera, ZoomWarp};
use crate::overworld::lozo::LozoTransition;
use crate::{
    animation::{Animated, SpriteAnimationAsset},
    asset::{AssetsExt, Phantom},
    overworld::{
        lozo::{InitLozo, Lozo, LozoAsset, LozoCommands},
        object::ObjectSpriteLookup,
    },
};

pub use grid::{
    Grid, GridCommands, GridCursor, GridIndex, GridPosition, GridSize, IterAll, IterAround,
    Neighbor, create_grid_bundle,
};

pub use asset::*;

pub const TILE_SIZE: u32 = 32;
pub const TILE_SIZE_VEC2: Vec2 = Vec2::splat(TILE_SIZE as f32);

mod asset;
mod grid;

pub struct TilePlugin;
impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_tile_grid)
            .add_observer(spawn_edge_events)
            .add_observer(on_load_next_lozo)
            .add_observer(on_activate_next_lozo)
            .add_observer(on_unload_next_lozo)
            .add_observer(on_play_sprite_animation)
            .add_observer(on_play_zoom_warp);
    }
}

#[derive(Component, Debug)]
#[require(Visibility, Transform)]
pub struct Tile {
    pub passability: Passability,
}

#[derive(
    FromDef, Component, Default, PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize, Hash,
)]
#[elf(def_type(Self))]
pub enum Passability {
    #[default]
    Always,
    Never,
    Bike,
    Surf,
    Waterfall,
}

impl ops::BitAnd for Passability {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        match self {
            Self::Always => rhs,
            Self::Bike => match rhs {
                Self::Always | Self::Bike => Self::Bike,
                other => other,
            },
            Self::Surf => match rhs {
                Self::Always | Self::Bike | Self::Surf => Self::Surf,
                other => other,
            },
            Self::Waterfall => match rhs {
                Self::Always | Self::Bike | Self::Surf | Self::Waterfall => Self::Waterfall,
                other => other,
            },
            Self::Never => Self::Never,
        }
    }
}

impl ops::BitAndAssign for Passability {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

#[derive(Hash, PartialEq, Eq, Serialize, Deserialize, Clone, Debug)]
pub struct TileEdge {
    pub from: UVec2,
    pub to: UVec2,
}

impl From<(UVec2, UVec2)> for TileEdge {
    fn from((from, to): (UVec2, UVec2)) -> Self {
        Self { from, to }
    }
}

impl TileEdge {
    pub fn reverse(&self) -> Self {
        Self {
            from: self.to,
            to: self.from,
        }
    }
}

#[derive(Component, Default)]
pub struct TileEdgeEvents<T: Send + Sync>(HashMap<TileEdge, Vec<TileEventAction>>, Phantom<T>);

impl<T: Send + Sync> TileEdgeEvents<T> {
    fn new(events: HashMap<TileEdge, Vec<TileEventAction>>) -> Self {
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

pub struct CharLeftTile;
pub struct CharEnteredTile;
pub struct CharReachedTile;

#[derive(FromDef, Debug, Clone)]
pub enum TileEventAction {
    LoadNextLozo {
        next_lozo_id: String,
        after_animation: Option<CameraAnimation>,
    },
    UnloadNextLozo,
    ActivateNextLozo,
    SpriteAnimation {
        sprite_id: String,

        #[elf(with_resolver(PathResolver))]
        animation: Handle<SpriteAnimationAsset>,
    },
    CameraAnimation(CameraAnimation),
}

#[derive(EntityEvent)]
pub struct TileGridSpawned(#[event_target] Entity);

impl TileGridSpawned {
    pub fn entity(&self) -> Entity {
        self.0
    }
}

fn spawn_tile_grid(
    event: On<InitLozo>,
    lozo_query: Query<&Lozo>,
    lozo_assets: Res<Assets<LozoAsset>>,
    mut commands: LozoCommands,
) -> Result {
    let lozo = lozo_query.get(event.entity())?;
    let lozo_asset = lozo_assets.require_handle(lozo.handle())?;

    let (grid, grid_size) = create_grid_bundle(lozo_asset.grid_size(), |pos| {
        let Some(tile_asset) = &lozo_asset.tile_grid[*pos.as_index()] else {
            return Ok(None);
        };
        let mut sprite_stack = Vec::new();
        for visual in tile_asset.sprite_stack.iter() {
            let spritesheet = &visual.spritesheet;
            let entity = spawn_tile_sprite(
                &visual.kind,
                spritesheet.clone(),
                Some(visual.layout.clone()),
                visual.z,
                &mut commands,
            )?;
            sprite_stack.push(entity);
        }

        let tile_entity = commands
            .spawn_into_lozo(
                event.entity(),
                (
                    Tile {
                        passability: tile_asset.passability,
                    },
                    Transform::from_translation(pos.to_world_pos().extend(0.0)),
                ),
            )?
            .add_children(&sprite_stack)
            .id();
        Ok(Some(tile_entity))
    })?;

    commands.entity(event.entity()).insert((grid, grid_size));
    commands.trigger(TileGridSpawned(event.entity()));

    Ok(())
}

pub fn spawn_tile_sprite(
    visual: &TileVisualKind,
    image_handle: Handle<Image>,
    layout_handle: Option<Handle<TextureAtlasLayout>>,
    z: f32,
    commands: &mut Commands,
) -> Result<Entity> {
    let transform = Transform::from_translation(Vec3::new(0.0, 0.0, z));
    let sprite = |index: usize| match layout_handle {
        Some(layout) => Sprite::from_atlas_image(image_handle, TextureAtlas { layout, index }),
        None => Sprite::from_image(image_handle),
    };
    Ok(match &visual {
        TileVisualKind::Static { idx } => commands.spawn((sprite(*idx), transform)).id(),
        TileVisualKind::Animated { animation } => commands
            .spawn((sprite(0), transform, Animated::by(animation.clone())))
            .id(),
    })
}

fn spawn_edge_events(
    event: On<InitLozo>,
    mut commands: LozoCommands,
    lozo_query: Query<&Lozo>,
    lozo_assets: Res<Assets<LozoAsset>>,
) -> Result {
    let lozo = lozo_query.get(event.entity())?;
    let lozo_asset = lozo_assets.require_handle(lozo.handle())?;

    commands.entity(event.entity()).insert((
        TileEdgeEvents::<CharLeftTile>::new(lozo_asset.char_left_events.clone()),
        TileEdgeEvents::<CharEnteredTile>::new(lozo_asset.char_entered_events.clone()),
        TileEdgeEvents::<CharReachedTile>::new(lozo_asset.char_reached_events.clone()),
    ));

    Ok(())
}

impl TileEventAction {
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
                sprite_id,
                animation,
            } => commands.trigger(PlaySpriteAnimation {
                sprite_id: sprite_id.clone(),
                animation: animation.clone(),
                lozo_entity: lozo,
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
struct LoadNextLozo {
    current: Entity,
    next: String,
    trigger: Entity,
    after_animation: Option<CameraAnimation>,
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
    sprite_id: String,
    animation: Handle<SpriteAnimationAsset>,
    lozo_entity: Entity,
}

#[derive(Event)]
pub struct PlayCameraAnimation {
    pub trigger: Entity,
    pub kind: CameraAnimation,
}

#[derive(FromDef, Debug, Clone)]
pub enum CameraAnimation {
    ZoomWarp { reverse: bool },
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

fn on_play_sprite_animation(
    event: On<PlaySpriteAnimation>,
    object_lookups: Query<&ObjectSpriteLookup>,
    mut commands: Commands,
) -> Result {
    let lookup = object_lookups.get(event.lozo_entity)?;
    let object_entity = lookup.lookup(&event.sprite_id)?;
    commands
        .entity(object_entity)
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
            CameraAnimation::ZoomWarp { reverse } => {
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

#[derive(Error, Debug)]
#[error("invalid tile position {0}")]
pub struct InvalidTilePosition(UVec2);
