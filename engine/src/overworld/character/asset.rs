use crate::overworld::character::Orientation;
use crate::{animation::SpriteAnimationAsset, asset::AssetsExt};
use bevy::prelude::*;
use bevy_elf::{AssetRef, FromDef, asset_spec};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Asset, TypePath, FromDef)]
#[asset_spec(base_path = "game://characters", extension = "char.ron")]
pub struct CharacterAsset {
    pub animations: HashMap<CharacterState, CharacterVisual>,

    pub spritesheet: CharacterSpritesheet,

    pub position: Vec3,
    pub orientation: Orientation,
    pub behaviour: Option<CharacterBehaviour>,
    pub dialog: Option<String>,
}

#[derive(FromDef, Clone)]
pub struct CharacterSpritesheet {
    #[elf(with_spec(base_path = "characters/images"))]
    pub image: AssetRef<Image>,

    #[elf(with_spec(base_path = "characters/layouts", extension = "tl.ron"))]
    pub layout: AssetRef<TextureAtlasLayout>,
}

impl CharacterSpritesheet {
    pub fn image(&self) -> &AssetRef<Image> {
        &self.image
    }

    pub fn layout(&self) -> &AssetRef<TextureAtlasLayout> {
        &self.layout
    }

    pub fn into_def(self) -> <Self as FromDef>::Def {
        CharacterSpritesheetDef {
            image: self.image.id().to_owned(),
            layout: self.layout.id().to_owned(),
        }
    }
}

#[derive(FromDef, Clone)]
pub enum CharacterBehaviour {
    Walking(Route),
}

#[derive(FromDef, Clone, Serialize, Deserialize, Debug)]
#[elf(def_type(Self))]
pub struct Route {
    pub targets: Vec<UVec2>,
    pub cycle: bool,
}

#[derive(FromDef, Debug, Clone)]
pub enum CharacterVisual {
    Static(usize),
    Animated(
        #[elf(with_spec(base_path = "characters/animations", extension = "ani.ron"))]
        AssetRef<SpriteAnimationAsset>,
    ),
}

impl CharacterVisual {
    pub fn atlas_index(&self, animations: &Assets<SpriteAnimationAsset>) -> Result<usize> {
        Ok(match self {
            Self::Static(idx) => *idx,
            Self::Animated(animation) => animations.require_handle(&animation.handle())?.frames[0],
        })
    }

    pub fn into_def(self) -> <Self as FromDef>::Def {
        match self {
            Self::Static(idx) => CharacterVisualDef::Static(idx),
            Self::Animated(asset_ref) => CharacterVisualDef::Animated(asset_ref.id().to_owned()),
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub enum CharacterState {
    Standing(Orientation),
    Walking(Orientation),
}

impl Default for CharacterState {
    fn default() -> Self {
        Self::Standing(Orientation::default())
    }
}

impl From<(super::CharacterState, Orientation)> for CharacterState {
    fn from((state, orientation): (super::CharacterState, Orientation)) -> Self {
        match state {
            super::CharacterState::Standing => Self::Standing(orientation),
            super::CharacterState::Walking => Self::Walking(orientation),
        }
    }
}

impl Into<super::CharacterState> for CharacterState {
    fn into(self) -> super::CharacterState {
        match self {
            Self::Standing(_) => super::CharacterState::Standing,
            Self::Walking(_) => super::CharacterState::Walking,
        }
    }
}

impl Into<(super::CharacterState, super::Orientation)> for CharacterState {
    fn into(self) -> (super::CharacterState, super::Orientation) {
        match self {
            Self::Standing(orientation) => (super::CharacterState::Standing, orientation.into()),
            Self::Walking(orientation) => (super::CharacterState::Walking, orientation.into()),
        }
    }
}
