use std::time::Duration;

use bevy::prelude::*;
use macros::{FromDef, asset_set};
use serde::{Deserialize, Serialize};

use super::{deserialize_duration_millis, serialize_duration_millis};
use crate::{animation::SpriteAnimation, assets::spawn::Spawn};

#[derive(FromDef, Asset, TypePath, Serialize, Deserialize, Debug)]
#[asset_set(
    base_path = "sprite_animations",
    extension = "ani.ron",
    asset_registry(crate::assets::registry),
    asset_type(Self)
)]
pub struct SpriteAnimationAsset {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    pub indices: Vec<usize>,
    #[serde(
        serialize_with = "serialize_duration_millis",
        deserialize_with = "deserialize_duration_millis"
    )]
    pub frame_duration: Duration,
}

impl Spawn for SpriteAnimationAsset {
    type B = SpriteAnimation;
    fn spawn(&self, handle: Handle<Self>) -> Self::B
    where
        Self: Sized,
    {
        SpriteAnimation::new(self, handle)
    }
}
