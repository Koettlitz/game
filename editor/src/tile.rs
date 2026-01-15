use bevy::prelude::*;
use engine::{
    assets::{SpriteSheet, TILE_SIZE},
    overworld::tile::Passability,
};

use crate::ui::PlaceTile;

const DEFAULT_TILE_GRID_WIDTH: u32 = 32;
const DEFAULT_TILE_GRID_HEIGHT: u32 = 24;

const GROUND_TILE_LAYER: isize = 0;

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_ground_tile_grid)
            .add_systems(Update, place_tile);
    }
}

fn init_ground_tile_grid(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let width = DEFAULT_TILE_GRID_WIDTH;
    let height = DEFAULT_TILE_GRID_HEIGHT;
    let tile_kind = GroundTileKind::default();
    let sprite_sheet = tile_kind.sprite_sheet();
    let mut grid = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            let sprite_pos = grid_pos_to_sprite_pos(UVec2::new(x, y), UVec2::new(width, height));
            let sprite_entity = commands
                .spawn((
                    Sprite {
                        image: asset_server.load(sprite_sheet.path()),
                        texture_atlas: Some(TextureAtlas {
                            layout: texture_atlas_layouts.add(sprite_sheet.texture_atlas_layout()),
                            index: tile_kind.texture_atlas_index(),
                        }),
                        ..default()
                    },
                    Transform::from_translation(sprite_pos.extend(GROUND_TILE_LAYER as f32)),
                ))
                .id();

            grid.push(GroundTile {
                kind: tile_kind,
                sprites: vec![sprite_entity],
            });
        }
    }

    commands.insert_resource(GroundTileGrid {
        width,
        height,
        tiles: grid,
    });
}

fn place_tile(
    mut event_reader: MessageReader<PlaceTile>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut ground_tile_grid: ResMut<GroundTileGrid>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    for m in event_reader.read() {
        let sprite_sheet = m.tile_kind.sprite_sheet();
        let sprite_pos = grid_pos_to_sprite_pos(m.pos, ground_tile_grid.size());
        let sprite_entity = commands
            .spawn((
                Sprite {
                    image: asset_server.load(sprite_sheet.path()),
                    texture_atlas: Some(TextureAtlas {
                        layout: texture_atlas_layouts.add(sprite_sheet.texture_atlas_layout()),
                        index: m.tile_kind.texture_atlas_index(),
                    }),
                    ..default()
                },
                Transform::from_translation(sprite_pos.extend(GROUND_TILE_LAYER as f32)),
            ))
            .id();
        let tile = ground_tile_grid
            .get_mut(m.pos)
            .unwrap_or_else(|| panic!("Invalid tile grid coords in PlaceTileEvent: {:?}", m.pos));
        for sprite in &tile.sprites {
            commands.entity(*sprite).despawn();
        }
        *tile = GroundTile {
            kind: m.tile_kind,
            sprites: vec![sprite_entity],
        };
    }
}

fn grid_pos_to_sprite_pos(grid_pos: UVec2, grid_size: UVec2) -> Vec2 {
    let half_grid_size = grid_size.as_vec2() / 2.0;
    let sprite_pos = grid_pos.as_vec2() - half_grid_size;
    sprite_pos.with_y(-sprite_pos.y) * TILE_SIZE.as_vec2()
}

#[derive(Resource)]
pub struct GroundTileGrid {
    width: u32,
    height: u32,
    tiles: Vec<GroundTile>,
}

impl GroundTileGrid {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    fn size(&self) -> UVec2 {
        UVec2::new(self.width(), self.height())
    }

    fn _set(&mut self, coords: impl Into<UVec2>, tile: GroundTile) {
        let coords = coords.into();
        if coords.x < self.width && coords.y < self.height {
            self.tiles[(coords.y * self.width + coords.x) as usize] = tile;
        } else {
            panic!("Invalid tile coords: {coords}");
        }
    }

    fn _get(&self, coords: impl Into<UVec2>) -> Option<&GroundTile> {
        let coords = coords.into();
        if coords.x < self.width && coords.y < self.height {
            Some(&self.tiles[(coords.y * self.width + coords.x) as usize])
        } else {
            None
        }
    }

    fn get_mut(&mut self, coords: impl Into<UVec2>) -> Option<&mut GroundTile> {
        let coords = coords.into();
        if coords.x < self.width && coords.y < self.height {
            Some(&mut self.tiles[(coords.y * self.width + coords.x) as usize])
        } else {
            None
        }
    }
}

struct GroundTile {
    kind: GroundTileKind,
    sprites: Vec<Entity>,
}

#[derive(Clone, Copy, Default)]
pub enum GroundTileKind {
    #[default]
    Gras,
    WaterCalm,
}

impl GroundTileKind {
    fn passable(&self) -> Passability {
        match self {
            Self::Gras => Passability::Always,
            Self::WaterCalm => Passability::Surf,
        }
    }

    fn sprite_sheet(&self) -> SpriteSheet {
        match self {
            Self::Gras => SpriteSheet::Outside,
            Self::WaterCalm => SpriteSheet::WaterCalm,
        }
    }

    fn texture_atlas_index(&self) -> usize {
        match self {
            Self::Gras => 1,
            Self::WaterCalm => 7,
        }
    }
}
