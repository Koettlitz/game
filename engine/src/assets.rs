use bevy::prelude::*;

pub const TILE_SIZE: UVec2 = UVec2::splat(32);

pub struct AssetsPlugin;
impl Plugin for AssetsPlugin {
    fn build(&self, _app: &mut App) {
        unimplemented!()
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub enum SpriteSheet {
    Outside,
    Inside,
    WaterCalm,
    WaterDeep,
}

impl SpriteSheet {
    pub fn path(&self) -> &'static str {
        match self {
            Self::Outside => "sprites/tilesets/Outside.png",
            Self::Inside => "sprites/tilesets/Inside.png",
            Self::WaterCalm => "sprites/autotiles/water_calm.png",
            Self::WaterDeep => "sprites/autotiles/water_deep.png",
        }
    }

    pub fn texture_atlas_layout(&self) -> TextureAtlasLayout {
        match self {
            Self::Outside => TextureAtlasLayout::from_grid(TILE_SIZE, 8, 888, None, None),
            Self::Inside => TextureAtlasLayout::from_grid(TILE_SIZE, 8, 736, None, None),
            Self::WaterCalm => TextureAtlasLayout::from_grid(TILE_SIZE, 3, 4, None, None),
            Self::WaterDeep => TextureAtlasLayout::from_grid(TILE_SIZE, 24, 4, None, None),
        }
    }
}
