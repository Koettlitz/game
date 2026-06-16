use bevy::prelude::*;
use engine::asset::{
    RonAssetLoader, RonAssetPlugin,
    overworld::{character::CharacterAsset, lozo::LozoAsset, object::GameObjectSpriteAsset},
};

pub struct AssetPlugin;
impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RonAssetPlugin::<LozoAsset>::default(),
            RonAssetPlugin::<CharacterAsset>::default(),
            RonAssetPlugin::<GameObjectSpriteAsset>::default(),
        ))
        .init_asset_loader::<RonAssetLoader<TextureAtlasLayout>>();
    }
}
