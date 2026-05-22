use bevy::prelude::*;

use self::{
    animation::SpriteAnimationPlugin, object::GameObjectAssetPlugin, tile::TileAssetPlugin,
};

pub mod animation;
pub mod object;
pub mod tile;

pub struct AssetPlugin;
impl Plugin for AssetPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins((
            TileAssetPlugin,
            GameObjectAssetPlugin,
            SpriteAnimationPlugin,
        ));
    }
}
