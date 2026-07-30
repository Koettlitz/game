use crate::animation::SpriteAnimationAsset;
use crate::asset::spritesheet::SpritesheetKind;
use bevy::prelude::*;
use bevy_elf::{FromDef, asset_spec};

#[derive(FromDef, Asset, TypePath)]
#[asset_spec(base_path = "game://lozo/objects/sprites", extension = "objsprite.ron")]
pub struct GameObjectSpriteAsset {
    #[elf(with_resolver(SpritesheetKind::Object))]
    pub image: Handle<Image>,
    pub sprite_kind: Option<TextureAtlasData>,
    pub world_position: Vec3,
}

#[derive(FromDef)]
pub struct TextureAtlasData {
    #[elf(
        with_spec(base_path = "objects/spritesheets/layouts", extension = "layout.ron"),
        expose_resolver
    )]
    pub layout: Handle<TextureAtlasLayout>,
    pub kind: SpriteKind,
}

#[derive(FromDef)]
pub enum SpriteKind {
    Static {
        idx: usize,
    },
    Animated {
        #[elf(implicit, with_spec(sub_path = "animations", extension = "ani.ron"))]
        animation: Handle<SpriteAnimationAsset>,
    },
}
