use bevy::prelude::*;
use macros::{FromDef, resolver};

use crate::assets::{
    AssetMap, AssetResolver,
    folder::AssetSet,
    sprite_sheet::{SpriteSheet, SpriteSheetMap},
};

#[derive(FromDef, Asset, TypePath)]
#[resolver(base_path = "game_objects", extension = "obj.ron", asset_type(Self))]
pub struct GameObjectAsset {
    pub x: u32,
    pub y: u32,
    pub sprite_sheed: Handle<Image>,
}

pub fn derive_texture_atlas_layouts<S, M>(
    tile_sprites: Res<AssetMap<S>>,
    images: Res<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut sprite_sheet_map: ResMut<M>,
) where
    S: AssetSet + 'static,
    S::Resolver: AssetResolver<Asset = Image>,
    M: SpriteSheetMap,
{
    for (id, handle) in tile_sprites.0.iter() {
        let Some(image) = images.get(handle.id()) else {
            continue;
        };
        let layout =
            TextureAtlasLayout::from_grid(image.size(), image.size().y, image.size().x, None, None);
        let layout = layouts.add(layout);
        let sprite_sheet = SpriteSheet {
            image: handle.clone(),
            layout,
        };
        sprite_sheet_map.insert(id.clone(), sprite_sheet);
    }
}
