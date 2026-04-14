use bevy::prelude::*;
use engine::asset::{
    animations::sprite::SpriteAnimationAsset,
    overworld::{lozo::LozoAsset, object::GameObjectAsset, tile::TileVisualsAsset},
};

type GameAssetPlugin<A> = engine::asset::RonAssetPlugin<A, A>;

pub struct AssetPlugin;
impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            GameAssetPlugin::<LozoAsset>::default(),
            GameAssetPlugin::<TileVisualsAsset>::default(),
            GameAssetPlugin::<GameObjectAsset>::default(),
            GameAssetPlugin::<SpriteAnimationAsset>::default(),
        ));
    }
}
