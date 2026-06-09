use std::hash::Hash;

use serde::{Deserialize, Serialize};

use crate::{
    asset::{animation::sprite::SpriteAnimationAsset, spritesheet::SpritesheetKind},
    overworld::tile::Passability,
};
use bevy::prelude::*;
use macros::FromDef;

#[derive(FromDef)]
pub struct TileAsset {
    pub passability: Passability,
    pub sprite_stack: Vec<TileVisualsAsset>,
}

impl Default for TileDef {
    fn default() -> Self {
        Self {
            passability: Passability::default(),
            sprite_stack: Vec::default(),
        }
    }
}

#[derive(FromDef)]
pub struct TileVisualsAsset {
    #[from_def(with_resolver(SpritesheetKind::Tile))]
    pub spritesheet: Handle<Image>,

    #[from_def(with_spec(base_path = "tiles/spritesheets/layouts", extension = "layout.ron"))]
    #[expose_resolver]
    pub layout: Handle<TextureAtlasLayout>,
    pub kind: TileVisualKind,
}

#[derive(FromDef)]
#[def_type(TileVisualKindDef)]
pub enum TileVisualKind {
    Static {
        idx: usize,
    },
    Animated {
        animation: Handle<SpriteAnimationAsset>,
    },
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TileVisualKindDef {
    Static { idx: usize },
    Animated { animation: String },
}

impl Default for TileVisualKindDef {
    fn default() -> Self {
        Self::Static { idx: 0 }
    }
}
