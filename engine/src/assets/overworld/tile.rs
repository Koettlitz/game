use std::fmt::{self, Display};

use crate::{
    assets::{AssetResolver, animations::sprite::SpriteAnimationAsset, folder::AssetSet},
    overworld::tile::Passability,
};
use bevy::prelude::*;
use macros::{FromDef, resolver};

use crate::{
    assets::{
        AssetMap,
        sprite_sheet::{SpriteSheet, SpriteSheetMap},
    },
    overworld::tile::TILE_SIZE,
};

#[derive(FromDef, Asset, TypePath)]
#[resolver(base_path = "game://tile", extension = "tile.ron", asset_type(Self))]
pub struct TileAsset {
    pub passability: Passability,
    pub sprite_stack: Vec<Handle<TileSpriteAsset>>,
}

#[derive(FromDef, Asset, TypePath)]
#[resolver(
    base_path = "game://tile_sprites",
    extension = "ts.ron",
    asset_type(Self)
)]
pub struct TileSpriteAsset {
    pub kind: TileSpriteKindAsset,
    pub image: Handle<Image>,
}

#[derive(FromDef)]
pub enum TileSpriteKindAsset {
    Static {
        idx: usize,
    },
    Animated {
        animation: Handle<SpriteAnimationAsset>,
    },
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
        let layout = derive_texture_atlas_layout(&image).unwrap_or_else(|e| panic!("{e}"));
        let layout = layouts.add(layout);
        let sprite_sheet = SpriteSheet {
            image: handle.clone(),
            layout,
        };
        sprite_sheet_map.insert(id.clone(), sprite_sheet);
    }
}

pub fn derive_texture_atlas_layout(
    image: &Image,
) -> Result<TextureAtlasLayout, TextureAtlasLayoutError> {
    if image.size() % TILE_SIZE != UVec2::splat(0) {
        return Err(TextureAtlasLayoutError);
    }
    let size_in_tiles = image.size() / TILE_SIZE;
    let layout =
        TextureAtlasLayout::from_grid(TILE_SIZE, size_in_tiles.x, size_in_tiles.y, None, None);
    Ok(layout)
}

pub struct TextureAtlasLayoutError;
impl Display for TextureAtlasLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "sprite sheet size not a multiple of tile size: {TILE_SIZE}"
        )
    }
}
