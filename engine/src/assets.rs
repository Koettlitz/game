use bevy::{platform::collections::HashMap, prelude::*};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

pub const TILE_SIZE: UVec2 = UVec2::splat(32);

pub struct AssetsPlugin;
impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpriteSheetMap>()
            .add_systems(Startup, load_sprite_sheets)
            .add_systems(Update, derive_texture_atlas_layouts);
    }
}

fn load_sprite_sheets(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut loading_map = HashMap::new();
    for sprite_sheet in SpriteSheetId::iter() {
        let handle = asset_server.load(sprite_sheet.path());
        loading_map.insert(sprite_sheet, handle);
    }
    commands.insert_resource(LoadingSpriteSheetMap(loading_map));
}

fn derive_texture_atlas_layouts(
    mut commands: Commands,
    loading_map: Option<ResMut<LoadingSpriteSheetMap>>,
    images: Res<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut sprite_sheet_map: ResMut<SpriteSheetMap>,
) {
    let Some(mut loading_map) = loading_map else {
        return;
    };
    for id in SpriteSheetId::iter() {
        let Some(image_handle) = loading_map.0.get(&id) else {
            continue;
        };
        let Some(image) = images.get(image_handle.id()) else {
            continue;
        };
        let size_in_tiles = image.size() / TILE_SIZE;
        let layout =
            TextureAtlasLayout::from_grid(TILE_SIZE, size_in_tiles.x, size_in_tiles.y, None, None);
        let layout = layouts.add(layout);
        let image = loading_map.0.remove(&id).expect("where did it go?");
        sprite_sheet_map.0.insert(id, SpriteSheet { image, layout });
    }
    if loading_map.0.is_empty() {
        commands.remove_resource::<LoadingSpriteSheetMap>();
    }
}

#[derive(Resource, Default)]
pub struct LoadingSpriteSheetMap(HashMap<SpriteSheetId, Handle<Image>>);

#[derive(Resource, Default)]
pub struct SpriteSheetMap(HashMap<SpriteSheetId, SpriteSheet>);
impl SpriteSheetMap {
    pub fn get(&self, id: &SpriteSheetId) -> &SpriteSheet {
        self.0
            .get(id)
            .unwrap_or_else(|| panic!("missing sprite sheet"))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

pub struct SpriteSheet {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, EnumIter)]
pub enum SpriteSheetId {
    Outside,
    Inside,
    WaterCalm,
    WaterDeep,
}

impl SpriteSheetId {
    pub fn path(&self) -> &'static str {
        match self {
            Self::Outside => "sprites/tilesets/Outside.png",
            Self::Inside => "sprites/tilesets/Inside.png",
            Self::WaterCalm => "sprites/autotiles/water_calm.png",
            Self::WaterDeep => "sprites/autotiles/water_with_shore.png",
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
