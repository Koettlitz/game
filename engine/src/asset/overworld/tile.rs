use std::{collections::HashMap, hash::Hash};

use serde::{Deserialize, Serialize};

use crate::{
    asset::{animation::sprite::SpriteAnimationAsset, spritesheet::SpritesheetKind},
    overworld::tile::Passability,
};
use bevy::prelude::*;
use bevy_elf::{FromDef, PathResolver};

#[derive(FromDef)]
#[elf(on_def(#[derive(Serialize, Deserialize, Default)]))]
pub struct TileAsset {
    pub passability: Passability,
    pub sprite_stack: Vec<TileVisualsAsset>,
    pub events: HashMap<TileEventTrigger, Vec<TileEventAction>>,
}

#[derive(FromDef, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, Hash)]
#[elf(def_type(Self))]
pub enum TileEventTrigger {
    CharLeftFrom,
    CharLeftTo,
    CharEnteredFrom,
    CharEntered,
    CharReachedFrom,
    CharReached,
}

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

#[derive(FromDef)]
pub struct TileVisualsAsset {
    #[elf(with_resolver(SpritesheetKind::Tile))]
    pub spritesheet: Handle<Image>,

    #[elf(
        with_spec(base_path = "tiles/spritesheets/layouts", extension = "layout.ron"),
        expose_resolver
    )]
    pub layout: Handle<TextureAtlasLayout>,
    pub kind: TileVisualKind,
    pub z: f32,
}

#[derive(FromDef)]
#[elf(def_type(TileVisualKindDef))]
pub enum TileVisualKind {
    Static {
        idx: usize,
    },
    Animated {
        #[elf(with_spec(base_path = "tiles/animations", extension = "ani.ron"))]
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
