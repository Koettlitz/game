use std::fmt::{self, Display};

use thiserror::Error;

use crate::{
    asset::{
        AssetRef, AssetResolver, FromDef, FromDefError, animations::sprite::SpriteAnimationAsset,
    },
    overworld::tile::Passability,
};
use bevy::prelude::*;
use macros::{FromDef, asset_spec};

use crate::overworld::tile::TILE_SIZE;

#[derive(FromDef, Asset, TypePath)]
#[asset_spec(base_path = "game://tile", extension = "tile.ron")]
pub struct TileAsset {
    pub passability: Passability,
    pub sprite_stack: Vec<AssetRef<TileVisualsAsset>>,
}

#[derive(FromDef, Asset, TypePath)]
#[asset_spec(base_path = "game://tile_visuals", extension = "ts.ron")]
pub struct TileVisualsAsset {
    pub kind: TileVisualKindAsset,
    pub image: TileSpriteSheet,
}

#[derive(Debug)]
#[asset_spec(base_path = "tiles/spritesheets")]
pub struct TileSpriteSheet {
    image: AssetRef<Image>,
    layout: Option<Handle<TextureAtlasLayout>>,
}

impl FromDef for TileSpriteSheet {
    type Def = String;
    type Error = FromDefError;

    fn from_def(def: Self::Def, ctx: &mut bevy::asset::LoadContext) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let handle = ctx.load(Self::resolve(&def)?);
        Ok(Self {
            image: AssetRef::new(def, handle),
            layout: None,
        })
    }
}

impl TileSpriteSheet {
    pub fn id(&self) -> &str {
        self.image.id()
    }

    pub fn image(&self) -> &Handle<Image> {
        &self.image.handle()
    }

    pub fn layout(&self) -> Result<&Handle<TextureAtlasLayout>, TileSpriteLayoutError> {
        self.layout
            .as_ref()
            .ok_or_else(|| TileSpriteLayoutError(self.image.id.clone()))
    }

    pub fn derive_layout<'a>(
        &'a mut self,
        images: &Assets<Image>,
        layouts: &'a mut Assets<TextureAtlasLayout>,
    ) -> Result<&'a Handle<TextureAtlasLayout>, TextureAtlasLayoutError> {
        let image =
            images
                .get(self.image.handle().id())
                .ok_or_else(|| TextureAtlasLayoutError {
                    id: self.image.id().to_string(),
                    kind: TextureAtlasLayoutErrorKind::MissingImage,
                })?;
        let layout = derive_texture_atlas_layout(image).ok_or_else(|| TextureAtlasLayoutError {
            id: self.image.id().to_string(),
            kind: TextureAtlasLayoutErrorKind::InvalidSize,
        })?;
        let handle = layouts.add(layout);
        self.layout = Some(handle.clone());
        Ok(self.layout.as_ref().unwrap())
    }
}

#[derive(Error, Debug)]
pub struct TextureAtlasLayoutError {
    pub id: String,
    pub kind: TextureAtlasLayoutErrorKind,
}

#[derive(Debug)]
pub enum TextureAtlasLayoutErrorKind {
    InvalidSize,
    MissingLayout,
    MissingImage,
}

impl Display for TextureAtlasLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            TextureAtlasLayoutErrorKind::InvalidSize => {
                write!(
                    f,
                    "size of sprite sheet \"{}\" not a multiple of tile size: {TILE_SIZE}",
                    self.id
                )
            }
            TextureAtlasLayoutErrorKind::MissingLayout => {
                write!(f, "missing TextureAtlasLayout for sprite sheet {}", self.id)
            }
            TextureAtlasLayoutErrorKind::MissingImage => {
                write!(f, "missing image for sprite sheet {}", self.id)
            }
        }
    }
}

#[derive(Error, Debug)]
#[error("missing TextureAtlasLayout for tile sprite \"{0}\"")]
pub struct TileSpriteLayoutError(String);

#[derive(FromDef)]
pub enum TileVisualKindAsset {
    Static {
        idx: usize,
    },
    Animated {
        animation: Handle<SpriteAnimationAsset>,
    },
}

pub fn derive_texture_atlas_layout(image: &Image) -> Option<TextureAtlasLayout> {
    if image.size() % TILE_SIZE != UVec2::splat(0) {
        return None;
    }
    let size_in_tiles = image.size() / TILE_SIZE;
    let layout =
        TextureAtlasLayout::from_grid(TILE_SIZE, size_in_tiles.x, size_in_tiles.y, None, None);
    Some(layout)
}
