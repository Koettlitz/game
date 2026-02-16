use bevy::prelude::*;

use crate::assets::{
    animation::SpriteAnimationPlugin, objects::ObjectAssetPlugin, tile::GroundTileAssetsPlugin,
};
pub mod animation;
mod objects;
pub mod tile;

pub struct AssetsPlugin;
impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins((
            GroundTileAssetsPlugin,
            ObjectAssetPlugin,
            SpriteAnimationPlugin,
        ));
    }
}
