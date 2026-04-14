use bevy::prelude::*;

use crate::asset::{
    animation::SpriteAnimationPlugin, object::ObjectKindAssetPlugin, tile::GroundTileAssetsPlugin,
};
pub mod animation;
pub mod object;
pub mod tile;

pub struct AssetPlugin;
impl Plugin for AssetPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins((
            GroundTileAssetsPlugin,
            ObjectKindAssetPlugin,
            SpriteAnimationPlugin,
        ));
    }
}
