use bevy::prelude::*;

use crate::assets::{AssetMap, AssetSet};

pub const TILE_SIZE: UVec2 = UVec2::splat(32);

#[derive(Component)]
pub struct SpriteSheet {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

pub trait SpriteSheetMap: Resource {
    fn insert(&mut self, id: String, value: SpriteSheet);
}

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
        if image.size() % TILE_SIZE != UVec2::splat(0) {
            panic!("sprite sheet size not a multiple of {TILE_SIZE}");
        }
        let size_in_tiles = image.size() / TILE_SIZE;
        let layout =
            TextureAtlasLayout::from_grid(TILE_SIZE, size_in_tiles.x, size_in_tiles.y, None, None);
        let layout = layouts.add(layout);
        let sprite_sheet = SpriteSheet {
            image: handle.clone(),
            layout,
        };
        sprite_sheet_map.insert(id.clone(), sprite_sheet);
    }
}
