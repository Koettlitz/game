use bevy::prelude::*;
use engine::asset::{RonAssetLoader, RonAssetPlugin, overworld::lozo::LozoAsset};

use crate::overworld::asset::character::CharacterAsset;

pub mod character;

pub struct AssetPlugin;
impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RonAssetPlugin::<LozoAsset>::default(),
            RonAssetPlugin::<CharacterAsset>::default(),
        ))
        .init_asset_loader::<RonAssetLoader<TextureAtlasLayout>>();
    }
}
