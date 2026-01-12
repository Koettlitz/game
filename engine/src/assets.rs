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
}

impl SpriteSheet {
    pub fn path(&self) -> &'static str {
        match self {
            Self::Outside => "assets/sprites/tilesets/Outside.png",
            Self::Inside => "assets/sprites/tilesets/Inside.png",
            Self::WaterCalm => "assets/sprites/autotiles/water_calm.png",
        }
    }

    pub fn texture_atlas_layout(&self) -> TextureAtlasLayout {
        match self {
            Self::Outside => TextureAtlasLayout::from_grid(TILE_SIZE, 8, 888, None, None),
            Self::Inside => TextureAtlasLayout::from_grid(TILE_SIZE, 8, 736, None, None),
            Self::WaterCalm => TextureAtlasLayout::from_grid(TILE_SIZE, 3, 4, None, None),
        }
    }
}
