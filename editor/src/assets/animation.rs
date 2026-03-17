use bevy::prelude::*;
use engine::animation::SpriteAnimation;
use engine::assets::EntityFolderPlugin;
use engine::assets::animations::sprite::{SpriteAnimationAsset, SpriteAnimationAssetLoader};
use macros::asset_set;

pub struct SpriteAnimationPlugin;
impl Plugin for SpriteAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<SpriteAnimationAssetLoader>()
            .init_asset::<SpriteAnimationAsset>()
            .add_plugins(EntityFolderPlugin::<AnimationFolder, SpriteAnimation>::default());
    }
}

#[asset_set(
    name = "SpriteAnimations",
    folder = "sprite_animations",
    asset_type(SpriteAnimationAsset)
)]
pub struct AnimationFolder;
