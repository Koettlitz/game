use std::ops::Deref;

use bevy::prelude::*;
use macros::{FromDef, asset_spec};

use crate::asset::{AssetRef, AssetResolver, FromDef, FromDefError};

#[derive(FromDef, Asset, TypePath)]
#[asset_spec(base_path = "game_objects", extension = "obj.ron")]
pub struct GameObjectAsset {
    pub x: u32,
    pub y: u32,
    pub sprite_sheet: ObjectSpritesheet,
}

#[asset_spec(base_path = "objects/spritesheets")]
pub struct ObjectSpritesheet(pub AssetRef<Image>);
impl Deref for ObjectSpritesheet {
    type Target = AssetRef<Image>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl FromDef for ObjectSpritesheet {
    type Def = String;
    type Error = FromDefError;

    fn from_def(def: Self::Def, ctx: &mut bevy::asset::LoadContext) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let handle = ctx.load(Self::resolve(&def)?);
        Ok(Self(AssetRef::new(def, handle)))
    }
}
