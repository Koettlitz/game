use std::borrow::Cow;

use bevy::prelude::*;
use macros::FromDef;
use serde::{Deserialize, Serialize};

use crate::asset::{
    AssetPathSpecProvider, AssetResolver, FromDef, FromDefError,
    animation::sprite::SpriteAnimationAsset,
};

pub struct Spritesheet {
    pub image: Handle<Image>,
    pub layout: Option<TextureAtlasLayout>,
}

#[derive(Serialize, Deserialize)]
pub struct SpritesheetDef {
    pub image: String,
    pub layout: Option<TextureAtlasLayout>,
    pub kind: SpritesheetKind,
}

impl FromDef for Spritesheet {
    type Def = SpritesheetDef;
    type Error = FromDefError;

    fn from_def(def: Self::Def, ctx: &mut bevy::asset::LoadContext) -> Result<Self, Self::Error> {
        Ok(Self {
            image: ctx.load(def.kind.resolve(&def.image)?),
            layout: def.layout,
        })
    }
}

#[derive(FromDef)]
pub enum SpriteKind {
    Static {
        idx: usize,
    },
    Animated {
        animation: Handle<SpriteAnimationAsset>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum SpritesheetKind {
    Tile,
    Object,
}

impl AssetPathSpecProvider for SpritesheetKind {
    fn base_path(&self) -> Cow<'static, str> {
        match self {
            Self::Tile => Cow::Borrowed("tiles/spritesheets"),
            Self::Object => Cow::Borrowed("objects/spritesheets"),
        }
    }

    fn extension(&self) -> Option<&'static str> {
        Some("png")
    }
}
