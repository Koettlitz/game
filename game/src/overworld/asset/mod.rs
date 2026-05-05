use bevy::prelude::*;
use engine::asset::{
    RonAssetPlugin, animations::sprite::SpriteAnimationAsset, overworld::lozo::LozoAsset,
};

pub struct AssetPlugin;
impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RonAssetPlugin::<LozoAsset>::default(),
            RonAssetPlugin::<SpriteAnimationAsset>::default(),
        ));
    }
}
