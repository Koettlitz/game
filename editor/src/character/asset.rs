use std::collections::HashMap;

use bevy::prelude::*;
use bevy_elf::FromDef;
use engine::{
    asset::AssetMap,
    overworld::character::asset::{CharacterSpritesheet, CharacterState, CharacterVisual},
};
use macros::asset_set;

pub type CharacterKindMap = AssetMap<CharacterResolverSet, CharacterKindAsset>;

#[derive(Asset, TypePath, FromDef)]
#[asset_set(base_path = "characters", progress_name = "characters")]
pub struct CharacterKindAsset {
    pub spritesheet: CharacterSpritesheet,
    pub animations: HashMap<CharacterState, CharacterVisual>,
}

impl CharacterKindAsset {
    pub fn default_visual(&self) -> &CharacterVisual {
        &self.animations[&CharacterState::default()]
    }
}
