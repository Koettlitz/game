use crate::animation::SpriteAnimationAsset;
use crate::asset::spritesheet::Spritesheet;
use bevy::prelude::*;
use bevy_elf::{FromDef, asset_spec};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Asset, TypePath, FromDef)]
#[asset_spec(base_path = "game://character", extension = "char.ron")]
pub struct CharacterAsset {
    pub animations: HashMap<CharacterState, CharacterVisual>,

    #[elf(from_default)]
    pub spritesheet: Spritesheet,
}

#[derive(FromDef, Debug)]
pub enum CharacterVisual {
    Static(usize),
    Animated(
        #[elf(with_spec(sub_path = "game://character/animations", extension = "ani.ron"))]
        Handle<SpriteAnimationAsset>,
    ),
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub enum Orientation {
    Up,
    Left,
    Right,
    Down,
}

impl From<crate::overworld::character::Orientation> for Orientation {
    fn from(orientation: crate::overworld::character::Orientation) -> Self {
        match orientation {
            crate::overworld::character::Orientation::Up => Self::Up,
            crate::overworld::character::Orientation::Left => Self::Left,
            crate::overworld::character::Orientation::Right => Self::Right,
            crate::overworld::character::Orientation::Down => Self::Down,
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub enum CharacterState {
    Standing(Orientation),
    Walking(Orientation),
}

impl
    From<(
        crate::overworld::character::CharacterState,
        crate::overworld::character::Orientation,
    )> for CharacterState
{
    fn from(
        (state, orientation): (
            crate::overworld::character::CharacterState,
            crate::overworld::character::Orientation,
        ),
    ) -> Self {
        let orientation = Orientation::from(orientation);
        match state {
            crate::overworld::character::CharacterState::Standing => Self::Standing(orientation),
            crate::overworld::character::CharacterState::Walking => Self::Walking(orientation),
        }
    }
}
