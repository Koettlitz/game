use std::{io, time::Duration};

use bevy::{asset::AssetLoader, prelude::*};
use ron::de::SpannedError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{deserialize_duration_millis, serialize_duration_millis};
use crate::{animation::SpriteAnimation, assets::FileAsset};

#[derive(TypePath, Default)]
pub struct SpriteAnimationAssetLoader;
impl AssetLoader for SpriteAnimationAssetLoader {
    type Asset = SpriteAnimationAsset;
    type Error = SpriteAssetLoadingError;
    type Settings = ();
    async fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _: &Self::Settings,
        _: &mut bevy::asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(ron::de::from_bytes(&mut bytes)?)
    }

    fn extensions(&self) -> &[&str] {
        &["ani.ron"]
    }
}

#[derive(Error, Debug)]
pub enum SpriteAssetLoadingError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Ron(#[from] SpannedError),
}

#[derive(Debug, Asset, TypePath, Serialize, Deserialize)]
pub struct SpriteAnimationAsset {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    indices: Vec<usize>,
    #[serde(
        serialize_with = "serialize_duration_millis",
        deserialize_with = "deserialize_duration_millis"
    )]
    frame_duration: Duration,
}

impl FileAsset for SpriteAnimationAsset {}

impl Into<SpriteAnimation> for SpriteAnimationAsset {
    fn into(self) -> SpriteAnimation {
        SpriteAnimation::new(self.indices, self.frame_duration)
    }
}
