use std::ops;
use std::{collections::HashMap, fmt::Debug};

use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_elf::FromDef;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::animation::Animated;
use crate::asset::AssetsExt;
use crate::asset::overworld::lozo::LozoAsset;
use crate::asset::overworld::tile::TileVisualKind;
use crate::overworld::lozo::{Lozo, LozoCommands, LozoSpawned};
use crate::overworld::object::ObjectSpriteLookup;
use crate::{
    asset::{
        animation::sprite::SpriteAnimationAsset,
        overworld::tile::{TileEventAction, TileEventTrigger},
    },
    overworld::{
        character::{CharEnteredTile, CharLeftTile, CharReachedTile},
        lozo::NextLozo,
    },
};

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
            .add_observer(on_char_left)
            .add_observer(on_char_entered)
            .add_observer(on_char_reached)
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
    pub events: HashMap<TileEventTrigger, Vec<TileEventAction>>,
}

impl Tile {
    pub fn new(
        passability: Passability,
        events: HashMap<TileEventTrigger, Vec<TileEventAction>>,
    ) -> Self {
        Self {
            passability,
            events,
        }
    }
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
            .spawn_into_lozo((
                Tile::new(tile_asset.passability, tile_asset.events.clone()),
                Transform::from_translation(pos.to_world_pos().extend(0.0)),
            ))
            .id();
        commands.entity(tile_entity).add_children(&sprite_stack);
        Ok(Some(tile_entity))
    })?;

    let entity = commands.spawn_into_lozo((grid, grid_size)).id();
    commands.trigger(TileGridSpawned(entity));

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
    Ok(match &visual {
        TileVisualKind::Static { idx } => {
            let sprite = if let Some(layout_handle) = layout_handle {
                Sprite::from_atlas_image(
                    image_handle.clone(),
                    TextureAtlas {
                        layout: layout_handle,
                        index: *idx,
                    },
                )
            } else {
                Sprite::from_image(image_handle.clone())
            };
            commands.spawn((sprite, transform)).id()
        }
        TileVisualKind::Animated { animation } => {
            let sprite = if let Some(layout_handle) = layout_handle {
                Sprite::from_atlas_image(
                    image_handle.clone(),
                    TextureAtlas {
                        layout: layout_handle,
                        ..Default::default()
                    },
                )
            } else {
                Sprite::from_image(image_handle.clone())
            };
            commands
                .spawn((sprite, transform, Animated::by(animation.clone())))
                .id()
        }
    })
}

impl TileEventAction {
    fn trigger_event(&self, commands: &mut Commands) {
        match self {
            Self::LoadNextLozo(id) => commands.trigger(LoadNextLozoEvent(id.clone())),
            Self::SpriteAnimation {
                sprite_id,
                animation: open_animation,
            } => commands.trigger(PlaySpriteAnimationEvent {
                sprite_id: sprite_id.clone(),
                animation: open_animation.clone(),
            }),
            Self::ActivateNextLozo => commands.trigger(ActivateNextLozoEvent),
            Self::UnloadNextLozo => commands.trigger(UnloadNextLozoEvent),
        };
    }
}

#[derive(Event)]
struct LoadNextLozoEvent(String);

#[derive(Event)]
struct UnloadNextLozoEvent;

#[derive(Event)]
struct ActivateNextLozoEvent;

#[derive(Event)]
struct PlaySpriteAnimationEvent {
    sprite_id: String,
    animation: Handle<SpriteAnimationAsset>,
}

#[derive(SystemParam)]
struct CharTileEvents<'w, 's> {
    tile_grid: Single<'w, 's, (&'static Grid<Option<Entity>>, &'static GridSize)>,
    tiles: Query<'w, 's, &'static Tile>,
    commands: Commands<'w, 's>,
}

impl<'w, 's> CharTileEvents<'w, 's> {
    fn dispatch_tile_events(&mut self, trigger: &TileEventTrigger, tile: UVec2) -> Result<()> {
        let tile =
            GridPosition::new(tile, self.tile_grid.1).ok_or_else(|| InvalidTilePosition(tile))?;
        let Some(tile) = self.tile_grid.0[tile] else {
            return Ok(());
        };
        let tile = self.tiles.get(tile)?;
        let Some(actions) = tile.events.get(trigger) else {
            return Ok(());
        };
        for action in actions {
            action.trigger_event(&mut self.commands);
        }
        Ok(())
    }
}

fn on_char_left(event: On<CharLeftTile>, mut char_tile_events: CharTileEvents) -> Result<()> {
    char_tile_events.dispatch_tile_events(&TileEventTrigger::CharLeftFrom, event.from)?;
    char_tile_events.dispatch_tile_events(&TileEventTrigger::CharLeftTo, event.to)?;
    Ok(())
}

fn on_char_entered(event: On<CharEnteredTile>, mut char_tile_events: CharTileEvents) -> Result<()> {
    char_tile_events.dispatch_tile_events(&TileEventTrigger::CharEnteredFrom, event.from)?;
    char_tile_events.dispatch_tile_events(&TileEventTrigger::CharEntered, event.to)?;
    Ok(())
}

fn on_char_reached(event: On<CharReachedTile>, mut char_tile_events: CharTileEvents) -> Result<()> {
    char_tile_events.dispatch_tile_events(&TileEventTrigger::CharReachedFrom, event.from)?;
    char_tile_events.dispatch_tile_events(&TileEventTrigger::CharReached, event.to)?;
    Ok(())
}

fn on_load_next_lozo(event: On<LoadNextLozoEvent>, mut next_lozo: ResMut<NextLozo>) {
    next_lozo.set(event.event().0.clone());
}

fn on_activate_next_lozo(_: On<ActivateNextLozoEvent>, mut next_lozo: ResMut<NextLozo>) {
    if let Some(next_lozo) = next_lozo.ready() {
        next_lozo.activate();
    } else {
        next_lozo.auto_activate = true;
    }
}

fn on_unload_next_lozo(_: On<UnloadNextLozoEvent>, mut next_lozo: ResMut<NextLozo>) {
    next_lozo.reset();
}

fn on_play_sprite_animation(
    event: On<PlaySpriteAnimationEvent>,
    object_lookup: Single<&ObjectSpriteLookup>,
    mut commands: Commands,
) -> Result {
    let object_entity = object_lookup.lookup(&event.sprite_id)?;
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
