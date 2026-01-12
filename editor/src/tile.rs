use bevy::prelude::*;
use engine::{assets::{SpriteSheet, TILE_SIZE}, overworld::tile::Passability};

use crate::ui::PlaceTileEvent;

const DEFAULT_TILE_GRID_WIDTH: u32 = 32;
const DEFAULT_TILE_GRID_HEIGHT: u32 = 32;

const GROUND_TILE_LAYER: isize = 0;

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_ground_tile_grid)
            .add_observer(place_tile);
    }
}

fn init_ground_tile_grid(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let size = (DEFAULT_TILE_GRID_WIDTH * DEFAULT_TILE_GRID_HEIGHT) as usize;
    let tile_kind = GroundTileKind::default();
    let sprite_sheet = tile_kind.sprite_sheet();
    let mut grid = Vec::with_capacity(size);
    for y in 0..DEFAULT_TILE_GRID_HEIGHT {
        for x in 0..DEFAULT_TILE_GRID_WIDTH {
            let sprite_entity = commands.spawn((
                Sprite {
                    image: asset_server.load(sprite_sheet.path()),
                    texture_atlas: Some(TextureAtlas {
                        layout: texture_atlas_layouts.add(sprite_sheet.texture_atlas_layout()),
                        index: tile_kind.texture_atlas_index()
                    }),
                    ..default()
                },
                Transform::from_xyz((x * TILE_SIZE.x) as f32, (y * TILE_SIZE.y) as f32, GROUND_TILE_LAYER as f32),
            )).id();

            grid.push(GroundTile {
                kind: tile_kind,
                sprites: vec![sprite_entity],
            });
        }
    }

    commands.insert_resource(GroundTileGrid {
        width: DEFAULT_TILE_GRID_WIDTH,
        height: DEFAULT_TILE_GRID_HEIGHT,
        tiles: grid,
    });
}

fn place_tile(event: On<PlaceTileEvent>, 
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut ground_tile_grid: ResMut<GroundTileGrid>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let sprite_sheet = event.tile_kind().sprite_sheet();
    let sprite_entity = commands.spawn((
        Sprite {
            image: asset_server.load(sprite_sheet.path()),
            texture_atlas: Some(TextureAtlas {
                layout: texture_atlas_layouts.add(sprite_sheet.texture_atlas_layout()),
                index: event.tile_kind().texture_atlas_index()
            }),
            ..default()
        },
        Transform::from_xyz(event.coords().x as f32, event.coords().y as f32, GROUND_TILE_LAYER as f32),
    )).id();
    let tile = ground_tile_grid.get_mut(event.coords()).unwrap_or_else(|| panic!("Invalid tile grid coords in PlaceTileEvent: {:?}", event.coords()));
    for entity in &tile.sprites {
        commands.entity(*entity).despawn();
    }
    *tile = GroundTile { kind: event.tile_kind(), sprites: vec![sprite_entity] };
}

#[derive(Resource)]
pub struct GroundTileGrid {
    width: u32,
    height: u32,
    tiles: Vec<GroundTile>,
}

impl GroundTileGrid {
    fn get_mut(&mut self, coords: impl Into<UVec2>) -> Option<&mut GroundTile> {
        let coords = coords.into();
        if coords.x < self.width && coords.y < self.height {
            Some(&mut self.tiles[(coords.y * self.height + coords.x) as usize])
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
    WaterCalm
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
