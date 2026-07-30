use std::{collections::HashMap, ops::Deref, time::Duration};

use crate::asset::duration_millis;
use bevy::prelude::*;
use bevy_elf::{AppExt, FromDef};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct SpriteAnimationAssetPlugin;

impl Plugin for SpriteAnimationAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_ron_asset::<SpriteAnimationAsset>()
            .init_ron_asset::<AnimationTimersAsset>();
    }
}

#[derive(FromDef, Asset, TypePath, Debug)]
pub struct SpriteAnimationAsset {
    pub frames: Vec<usize>,
    pub timer: AnimationTimerApi,
    pub kind: AnimationKind,
}

#[derive(Serialize, Deserialize, FromDef, Debug)]
#[elf(def_type(Self))]
#[serde(untagged)]
pub enum AnimationTimerApi {
    TimerGroup(String),
    FrameDuration(#[serde(with = "duration_millis")] Duration),
}

#[derive(FromDef, Debug, Clone, Copy)]
pub enum AnimationKind {
    Repeating,
    Once,
}

impl AnimationKind {
    pub fn is_repeating(&self) -> bool {
        matches!(self, Self::Repeating)
    }
}

#[derive(Asset, TypePath, Debug, Serialize, Deserialize, FromDef)]
#[elf(def_type(Self))]
pub struct AnimationTimersAsset(HashMap<String, u64>);

impl Deref for AnimationTimersAsset {
    type Target = HashMap<String, u64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
