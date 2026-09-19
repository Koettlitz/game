use std::fmt::Debug;
use std::ops;

use bevy::prelude::*;
use bevy_elf::FromDef;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    animation::Animated,
    asset::AssetsExt,
    overworld::lozo::{InitLozo, Lozo, LozoAsset, LozoCommands},
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
        app.add_observer(spawn_tile_grid);
    }
}

#[derive(Component, Debug)]
#[require(Visibility, Transform)]
pub struct Tile {
    pub passability: Passability,
    pub blocked: bool,
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
                        blocked: tile_asset.blocked,
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

#[derive(Error, Debug)]
#[error("invalid tile position {0}")]
pub struct InvalidTilePosition(UVec2);
