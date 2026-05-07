use bevy::{asset::AssetPath, prelude::*};
use serde::{Deserialize, Serialize};

use crate::asset::{
    AssetResolver, FromDef, FromDefError,
    overworld::{object::ObjectSpritesheet, tile::TileKindSpritesheet},
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum SpritesheetKind {
    Tile,
    Object,
}

impl SpritesheetKind {
    fn resolve(&self, id: &str) -> Result<AssetPath<'static>, FromDefError> {
        match self {
            Self::Tile => TileKindSpritesheet::resolve(id),
            Self::Object => ObjectSpritesheet::resolve(id),
        }
    }
}
