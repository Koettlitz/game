use std::time::Duration;

use bevy::prelude::*;
use macros::{FromDef, asset_set};
use serde::{Deserialize, Serialize};

use super::{deserialize_duration_millis, serialize_duration_millis};
use crate::asset::{AssetRef, spawn::Spawn};

#[derive(FromDef, Asset, TypePath, Serialize, Deserialize, Debug)]
#[def_type(Self)]
#[asset_set(base_path = "sprite_animations", progress_name = "sprite_animations")]
pub struct SpriteAnimationAsset {
    pub indices: Vec<usize>,
    #[serde(
        serialize_with = "serialize_duration_millis",
        deserialize_with = "deserialize_duration_millis"
    )]
    pub frame_duration: Duration,
}

impl Spawn for SpriteAnimationAsset {
    type B = crate::animation::SpriteAnimation;
    fn spawn(&self, asset_ref: AssetRef<Self>) -> Self::B
    where
        Self: Sized,
    {
        crate::animation::SpriteAnimation::new(self.frame_duration, asset_ref)
    }
}
