use bevy::prelude::*;
use engine::animation::SpriteAnimation;
use engine::assets::animations::sprite::SpriteAnimationAsset;
use engine::assets::{EntityFolderPlugin, GameAssetLoader};

pub struct SpriteAnimationPlugin;
impl Plugin for SpriteAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<GameAssetLoader<SpriteAnimationAsset, SpriteAnimationAsset>>()
            .init_asset::<SpriteAnimationAsset>()
            .add_plugins(EntityFolderPlugin::<SpriteAnimationAsset, SpriteAnimation>::default());
    }
}
