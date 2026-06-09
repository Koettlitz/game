use crate::asset::spritesheet::{SpriteKind, SpritesheetKind};
use bevy::prelude::*;
use macros::{FromDef, asset_spec};

#[derive(FromDef, Asset, TypePath)]
#[asset_spec(base_path = "game://lozo/objects/sprites", extension = "objsprite.ron")]
pub struct GameObjectSpriteAsset {
    #[from_def(with_resolver(SpritesheetKind::Object))]
    pub image: Handle<Image>,
    pub sprite_kind: Option<TextureAtlasData>,
    pub world_position: Vec3,
}

#[derive(FromDef)]
pub struct TextureAtlasData {
    #[from_def(with_spec(base_path = "objects/spritesheets/layouts", extension = "layout.ron"))]
    #[expose_resolver]
    pub layout: Handle<TextureAtlasLayout>,
    pub kind: SpriteKind,
}
