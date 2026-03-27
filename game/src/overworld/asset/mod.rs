use bevy::prelude::*;
use engine::assets::{
    animations::sprite::SpriteAnimationAsset,
    overworld::{lozo::LozoAsset, object::GameObjectAsset, tile::TileSpriteAsset},
};

type GameAssetPlugin<A> = engine::assets::GameAssetPlugin<A, A>;

pub struct AssetPlugin;
impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            GameAssetPlugin::<LozoAsset>::default(),
            GameAssetPlugin::<TileSpriteAsset>::default(),
            GameAssetPlugin::<GameObjectAsset>::default(),
            GameAssetPlugin::<SpriteAnimationAsset>::default(),
        ));
    }
}
