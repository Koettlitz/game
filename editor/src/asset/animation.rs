use bevy::prelude::*;
use engine::asset::animation::sprite::SpriteAnimationAsset;
use engine::asset::{AssetSetPlugin, RonAssetPlugin};

pub struct SpriteAnimationPlugin;
impl Plugin for SpriteAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RonAssetPlugin::<SpriteAnimationAsset>::default())
            .add_plugins(AssetSetPlugin::<SpriteAnimationAsset>::default());
    }
}
