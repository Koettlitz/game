use bevy::prelude::*;

use crate::assets::{AssetMap, AssetSet, SpriteSheet, SpriteSheetMap};

pub fn derive_texture_atlas_layouts<S, R>(
    tile_sprites: Res<AssetMap<S>>,
    images: Res<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut sprite_sheet_map: ResMut<R>,
) where
    S: AssetSet<Asset = Image>,
    R: SpriteSheetMap,
{
    for (id, handle) in tile_sprites.0.iter() {
        let Some(image) = images.get(handle.id()) else {
            continue;
        };
        let layout =
            TextureAtlasLayout::from_grid(image.size(), image.size().x, image.size().y, None, None);
        let layout = layouts.add(layout);
        let sprite_sheet = SpriteSheet {
            image: handle.clone(),
            layout,
        };
        sprite_sheet_map.insert(id.clone(), sprite_sheet);
    }
}
