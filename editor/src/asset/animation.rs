use bevy::prelude::*;
use engine::animation::SpriteAnimation;
use engine::asset::animations::sprite::SpriteAnimationAsset;
use engine::asset::{EntityFolderPlugin, HasResolver, RonAssetPlugin};

pub struct SpriteAnimationPlugin;
impl Plugin for SpriteAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RonAssetPlugin::<SpriteAnimationAsset>::default())
            .add_plugins(EntityFolderPlugin::<
                <SpriteAnimationAsset as HasResolver>::Resolver,
                SpriteAnimationAsset,
                SpriteAnimation,
            >::default());
    }
}
