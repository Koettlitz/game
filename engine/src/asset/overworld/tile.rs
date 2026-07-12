use std::hash::Hash;

use serde::{Deserialize, Serialize};

use crate::{
    asset::{animation::sprite::SpriteAnimationAsset, spritesheet::SpritesheetKind},
    overworld::tile::Passability,
};
use bevy::prelude::*;
use bevy_elf::FromDef;

#[derive(FromDef, Debug)]
#[elf(on_def(#[derive(Serialize, Deserialize, Default)]))]
pub struct TileAsset {
    pub passability: Passability,
    pub sprite_stack: Vec<TileVisualsAsset>,
}

#[derive(FromDef, Debug)]
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

#[derive(FromDef, Debug)]
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
