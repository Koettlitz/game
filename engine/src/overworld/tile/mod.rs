use std::ops;
use std::{collections::HashMap, fmt::Debug};

use bevy::prelude::*;
use bevy_elf::{FromDef, PathResolver};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::animation::Animated;
use crate::asset::overworld::lozo::LozoAsset;
use crate::asset::overworld::tile::TileVisualKind;
use crate::asset::{AssetsExt, Phantom};
use crate::overworld::lozo::{Lozo, LozoCommands, LozoSpawned};
use crate::overworld::object::ObjectSpriteLookup;
use crate::{asset::animation::sprite::SpriteAnimationAsset, overworld::lozo::NextLozo};

pub use grid::{
    Grid, GridCommands, GridCursor, GridIndex, GridPosition, GridSize, IterAll, IterAround,
    Neighbor, create_grid_bundle, shrink_grid,
};

pub const TILE_SIZE: u32 = 32;
pub const TILE_SIZE_VEC2: Vec2 = Vec2::splat(TILE_SIZE as f32);

mod grid;

pub struct TilePlugin;
impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_tile_grid)
            .add_observer(spawn_edge_events)
            .add_observer(on_load_next_lozo)
            .add_observer(on_activate_next_lozo)
            .add_observer(on_unload_next_lozo)
            .add_observer(on_play_sprite_animation);
    }
}

#[derive(Component, Debug)]
#[require(Visibility, Transform)]
pub struct Tile {
    pub passability: Passability,
}

#[derive(FromDef, Component, PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize, Hash)]
#[elf(def_type(Self))]
pub enum Passability {
    Always,
    Never,
    Bike,
    Surf,
    Waterfall,
}

impl Default for Passability {
    fn default() -> Self {
        Self::Always
    }
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

#[derive(Hash, PartialEq, Eq, Serialize, Deserialize, Clone)]
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

    pub fn trigger(&self, edge: &TileEdge, lozo_entity: Entity, commands: &mut Commands) {
        if let Some(actions) = self.0.get(edge) {
            for action in actions {
                action.trigger_event(lozo_entity, commands);
            }
        }
    }
}

pub struct CharLeftTile;
pub struct CharEnteredTile;
pub struct CharReachedTile;

#[derive(FromDef, Debug, Clone)]
pub enum TileEventAction {
    LoadNextLozo(String),
    UnloadNextLozo,
    SpriteAnimation {
        sprite_id: String,

        #[elf(with_resolver(PathResolver))]
        animation: Handle<SpriteAnimationAsset>,
    },
    ActivateNextLozo,
}

#[derive(EntityEvent)]
pub struct TileGridSpawned(#[event_target] Entity);

impl TileGridSpawned {
    pub fn entity(&self) -> Entity {
        self.0
    }
}

fn spawn_tile_grid(
    event: On<LozoSpawned>,
    lozo: Query<&Lozo>,
    lozo_assets: Res<Assets<LozoAsset>>,
    mut commands: LozoCommands,
) -> Result {
    let lozo = lozo.get(event.entity())?;
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
    event: On<LozoSpawned>,
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
    pub fn trigger_event(&self, lozo_entity: Entity, commands: &mut Commands) {
        match self {
            Self::LoadNextLozo(id) => commands.trigger(LoadNextLozoEvent {
                current: lozo_entity,
                next: id.clone(),
            }),
            Self::SpriteAnimation {
                sprite_id,
                animation: open_animation,
            } => commands.trigger(PlaySpriteAnimationEvent {
                sprite_id: sprite_id.clone(),
                animation: open_animation.clone(),
                lozo_entity: lozo_entity,
            }),
            Self::ActivateNextLozo => commands.trigger(ActivateNextLozoEvent(lozo_entity)),
            Self::UnloadNextLozo => commands.trigger(UnloadNextLozoEvent(lozo_entity)),
        };
    }
}

#[derive(Event)]
struct LoadNextLozoEvent {
    current: Entity,
    next: String,
}

#[derive(Event)]
struct UnloadNextLozoEvent(Entity);

#[derive(Event)]
struct ActivateNextLozoEvent(Entity);

#[derive(Event)]
struct PlaySpriteAnimationEvent {
    sprite_id: String,
    animation: Handle<SpriteAnimationAsset>,
    lozo_entity: Entity,
}

fn on_load_next_lozo(event: On<LoadNextLozoEvent>, mut next_lozo: Query<&mut NextLozo>) -> Result {
    next_lozo
        .get_mut(event.current)?
        .set(event.event().next.clone());

    Ok(())
}

fn on_activate_next_lozo(
    event: On<ActivateNextLozoEvent>,
    mut next_lozo: Query<&mut NextLozo>,
) -> Result {
    let mut next_lozo = next_lozo.get_mut(event.0)?;
    if let Some(ready) = next_lozo.ready() {
        ready.activate();
    } else {
        next_lozo.auto_activate = true;
    }

    Ok(())
}

fn on_unload_next_lozo(
    event: On<UnloadNextLozoEvent>,
    mut next_lozo: Query<&mut NextLozo>,
) -> Result {
    Ok(next_lozo.get_mut(event.0)?.reset())
}

fn on_play_sprite_animation(
    event: On<PlaySpriteAnimationEvent>,
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

#[derive(Error, Debug)]
#[error("invalid tile position {0}")]
pub struct InvalidTilePosition(UVec2);

#[derive(Error, Debug)]
#[error("could not execute switch lozo event, cause lozo was not loaded yet")]
pub struct LozoNotLoaded;
