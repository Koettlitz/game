use std::{collections::HashMap, ops::Deref, time::Duration};

use crate::asset::duration_millis;
use bevy::prelude::*;
use macros::{FromDef, asset_set};
use serde::{Deserialize, Serialize};

use crate::asset::RonAssetPlugin;

#[derive(Default)]
pub struct SpriteAnimationAssetPlugin;

impl Plugin for SpriteAnimationAssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RonAssetPlugin::<SpriteAnimationAsset>::default(),
            RonAssetPlugin::<AnimationTimersAsset>::default(),
        ));
    }
}

#[derive(FromDef, Asset, TypePath, Debug)]
#[asset_set(base_path = "sprite_animations", progress_name = "sprite_animations")]
pub struct SpriteAnimationAsset {
    pub frames: Vec<usize>,
    pub timer: AnimationTimerApi,
}

#[derive(Serialize, Deserialize, FromDef, Debug)]
#[def_type(Self)]
#[serde(untagged)]
pub enum AnimationTimerApi {
    TimerGroup(String),
    FrameDuration(#[serde(with = "duration_millis")] Duration),
}

#[derive(Asset, TypePath, Debug, Serialize, Deserialize, FromDef)]
#[def_type(Self)]
pub struct AnimationTimersAsset(HashMap<String, u64>);

impl Deref for AnimationTimersAsset {
    type Target = HashMap<String, u64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
