use std::{collections::HashSet, time::Duration};

use bevy::{platform::collections::HashMap, prelude::*};
use engine::{
    animation::{Animated, SpriteAnimation},
    assets::{SpriteSheetId, SpriteSheetMap, TILE_SIZE},
    overworld::tile::{GridSize, TileGrid},
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::{
    State,
    tile::{GroundTile, GroundTileGrid, GroundTilesChanged},
};

const TILE_LAYER_BASE: i32 = 1;
const TILE_LAYER_MIDDLE: i32 = 500;
const TILE_LAYER_TOP: i32 = 1000;

pub struct TileVisualsPlugin;
impl Plugin for TileVisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_tile_animations)
            .add_message::<UpdateTileSprites>()
            .add_observer(on_ground_tile_changed)
            .add_systems(
                OnTransition {
                    exited: State::LoadingAssets,
                    entered: State::Initialized,
                },
                init_sprite_grid,
            )
            .add_systems(
                PostUpdate,
                update_sprites.run_if(in_state(State::Initialized)),
            );
    }
}

fn spawn_tile_animations(mut commands: Commands) {
    let mut lookup_table = GroundTileAnimationLookupTable::default();

    for edge in GroundTileEdge::iter() {
        let indices = match edge {
            GroundTileEdge::TopLeft => vec![24, 27, 30, 33, 36, 39, 42, 45],
            GroundTileEdge::Top => vec![25, 28, 31, 34, 37, 40, 43, 46],
            GroundTileEdge::TopRight => vec![26, 29, 32, 35, 38, 41, 44, 47],
            GroundTileEdge::Left => vec![48, 51, 54, 57, 60, 63, 66, 69],
            GroundTileEdge::Right => vec![50, 53, 56, 59, 62, 65, 68, 71],
            _ => vec![49, 52, 55, 58, 61, 64, 67, 70],
        };
        let animation = SpriteAnimation::new(indices, Duration::from_millis(200));
        let entity = commands.spawn(animation).id();

        let animation_id = GroundTileAnimationId {
            tile_kind: GroundTile::WaterDeep,
            edge,
        };
        lookup_table.0.insert(animation_id, entity);
    }

    commands.insert_resource(lookup_table);
}

fn init_sprite_grid(
    mut commands: Commands,
    grid_size: Res<GridSize>,
    mut message_writer: MessageWriter<UpdateTileSprites>,
    mut initialized: Local<bool>,
) {
    if *initialized {
        return;
    }
    commands.insert_resource(TileSpriteGrid(TileGrid::new(*grid_size)));
    for y in 0..grid_size.0.y {
        for x in 0..grid_size.0.x {
            message_writer.write(UpdateTileSprites(UVec2::new(x, y)));
        }
    }
    *initialized = true;
}

fn on_ground_tile_changed(
    event: On<GroundTilesChanged>,
    mut message_writer: MessageWriter<UpdateTileSprites>,
    grid_size: Res<GridSize>,
) {
    let mut sprites_to_update = HashSet::new();
    for changed in &event.0 {
        sprites_to_update.insert(*changed);
        sprites_to_update.insert(changed + UVec2::Y);
        sprites_to_update.insert(changed - UVec2::Y);
        sprites_to_update.insert(changed + UVec2::X);
        sprites_to_update.insert(changed - UVec2::X);
    }
    for sprite_to_update in sprites_to_update {
        if grid_size.contains(sprite_to_update.as_vec2()) {
            message_writer.write(UpdateTileSprites(sprite_to_update));
        }
    }
}

fn update_sprites(
    mut message_reader: MessageReader<UpdateTileSprites>,
    ground_tile_grid: Res<GroundTileGrid>,
    grid_size: Res<GridSize>,
    mut tile_sprite_grid: ResMut<TileSpriteGrid>,
    sprite_sheets: Res<SpriteSheetMap>,
    animation_lookup_table: Res<GroundTileAnimationLookupTable>,
    animations: Query<&SpriteAnimation>,
    mut commands: Commands,
) {
    for coords in message_reader.read() {
        let constellation = GroundTileConstellation::from_grid(&ground_tile_grid, coords.0);
        let visual = constellation.visual();
        let sprites = tile_sprite_grid
            .0
            .get_mut(coords.0)
            .expect("invalid grid position of UpdateTileSprites message");
        for sprite in sprites.drain(..) {
            commands.entity(sprite).despawn();
        }
        for (layer, visual) in visual.iter() {
            let entity = match visual {
                GroundTileVisual::Static {
                    sprite_sheet,
                    atlas_idx,
                } => {
                    let sprite_sheet = sprite_sheets.get(sprite_sheet);
                    commands.spawn((
                        Sprite {
                            image: sprite_sheet.image.clone(),
                            texture_atlas: Some(TextureAtlas {
                                layout: sprite_sheet.layout.clone(),
                                index: *atlas_idx,
                            }),
                            ..Default::default()
                        },
                        Transform::from_translation(
                            grid_pos_to_sprite_pos(coords.0, grid_size.0).extend(*layer as f32),
                        ),
                    ))
                }
                GroundTileVisual::Animated(animation_id) => {
                    let sprite_sheet = sprite_sheets.get(&animation_id.tile_kind.sprite_sheet());
                    let animation_entity = animation_lookup_table
                        .0
                        .get(animation_id)
                        .unwrap_or_else(|| {
                            panic!(
                                "missing entry in lookup_table for animation_id: {animation_id:?}"
                            )
                        });
                    let animation = animations
                        .get(*animation_entity)
                        .unwrap_or_else(|_| panic!("missing animation for id: {animation_id:?}"));
                    commands.spawn((
                        Sprite {
                            image: sprite_sheet.image.clone(),
                            texture_atlas: Some(TextureAtlas {
                                layout: sprite_sheet.layout.clone(),
                                index: animation.current_idx(),
                            }),
                            ..Default::default()
                        },
                        Transform::from_translation(
                            grid_pos_to_sprite_pos(coords.0, grid_size.0).extend(*layer as f32),
                        ),
                        Animated::by(*animation_entity),
                    ))
                }
            }
            .id();
            sprites.push(entity);
        }
    }
}

fn grid_pos_to_sprite_pos(grid_pos: UVec2, grid_size: UVec2) -> Vec2 {
    let half_grid_size = grid_size.as_vec2() / 2.0;
    let sprite_pos = grid_pos.as_vec2() - half_grid_size;
    sprite_pos.with_y(-sprite_pos.y) * TILE_SIZE.as_vec2()
}

#[derive(Message)]
struct UpdateTileSprites(UVec2);

#[derive(Resource)]
struct TileSpriteGrid(TileGrid<Vec<Entity>>);

#[derive(Resource, Default)]
struct GroundTileAnimationLookupTable(HashMap<GroundTileAnimationId, Entity>);

#[derive(PartialEq, Eq, Hash)]
struct GroundTileConstellation {
    tile_kind: GroundTile,
    top: Option<GroundTile>,
    left: Option<GroundTile>,
    right: Option<GroundTile>,
    bot: Option<GroundTile>,
}

impl GroundTileConstellation {
    fn from_grid(grid: &GroundTileGrid, position: impl Into<UVec2>) -> Self {
        let position = position.into();
        Self {
            tile_kind: grid
                .0
                .get(position)
                .unwrap_or_else(|| panic!("invalid grid position: {position}")),
            top: position.checked_sub(UVec2::Y).and_then(|p| grid.0.get(p)),
            left: position.checked_sub(UVec2::X).and_then(|p| grid.0.get(p)),
            right: grid.0.get(position + UVec2::X),
            bot: grid.0.get(position + UVec2::Y),
        }
    }

    fn visual(&self) -> HashMap<i32, GroundTileVisual> {
        let mut result = HashMap::new();
        match self.tile_kind {
            GroundTile::Gras => {
                result.insert(
                    TILE_LAYER_BASE,
                    GroundTileVisual::Static {
                        sprite_sheet: SpriteSheetId::Outside,
                        atlas_idx: 1,
                    },
                );
                if matches!(self.top, Some(top) if top.is_water()) {
                    result.insert(
                        TILE_LAYER_MIDDLE,
                        GroundTileVisual::Static {
                            sprite_sheet: SpriteSheetId::Outside,
                            atlas_idx: 21,
                        },
                    );
                    result.insert(
                        TILE_LAYER_TOP,
                        GroundTileVisual::Static {
                            sprite_sheet: SpriteSheetId::Outside,
                            atlas_idx: 705,
                        },
                    );
                }
            }
            GroundTile::WaterCalm => {
                let sprite_sheet = SpriteSheetId::WaterCalm;
                let is_water = |t: GroundTile| t.is_water();
                let not_water = |t: GroundTile| !t.is_water();
                let index = if self.top.is_none_or(is_water)
                    && self.left.is_none_or(is_water)
                    && self.right.is_none_or(is_water)
                {
                    7 // no edge
                } else if self.left.is_some_and(not_water)
                    && self.top.is_some_and(not_water)
                    && self.right.is_none_or(is_water)
                {
                    3 // top left corner
                } else if self.top.is_some_and(not_water)
                    && self.left.is_none_or(is_water)
                    && self.right.is_none_or(is_water)
                {
                    4 // top edge
                } else if self.top.is_some_and(not_water)
                    && self.left.is_none_or(is_water)
                    && self.right.is_some_and(not_water)
                {
                    5 // top right corner
                } else if self.left.is_some_and(not_water)
                    && self.top.is_none_or(is_water)
                    && self.right.is_none_or(is_water)
                {
                    6 // left edge
                } else if self.top.is_none_or(is_water)
                    && self.left.is_none_or(is_water)
                    && self.right.is_some_and(not_water)
                {
                    8 // right edge
                } else if self.top.is_some_and(not_water)
                    && self.left.is_some_and(not_water)
                    && self.right.is_some_and(not_water)
                {
                    0 // all edges
                } else {
                    7

                    // panic!(
                    //     "unhandled constellation: top: {:?}, left: {:?}, right: {:?}, bottom: {:?}",
                    //     self.top, self.left, self.right, self.bot
                    // )
                };
                result.insert(
                    TILE_LAYER_MIDDLE,
                    GroundTileVisual::Static {
                        sprite_sheet,
                        atlas_idx: index,
                    },
                );
            }
            GroundTile::WaterDeep => {
                let is_water = |t: GroundTile| t.is_water();
                let not_water = |t: GroundTile| !t.is_water();
                let edge = if self.top.is_none_or(is_water)
                    && self.left.is_none_or(is_water)
                    && self.right.is_none_or(is_water)
                {
                    GroundTileEdge::None
                } else if self.left.is_some_and(not_water)
                    && self.top.is_some_and(not_water)
                    && self.right.is_none_or(is_water)
                {
                    GroundTileEdge::TopLeft
                } else if self.top.is_some_and(not_water)
                    && self.left.is_none_or(is_water)
                    && self.right.is_none_or(is_water)
                {
                    GroundTileEdge::Top
                } else if self.top.is_some_and(not_water)
                    && self.left.is_none_or(is_water)
                    && self.right.is_some_and(not_water)
                {
                    GroundTileEdge::TopRight
                } else if self.left.is_some_and(not_water)
                    && self.top.is_none_or(is_water)
                    && self.right.is_none_or(is_water)
                {
                    GroundTileEdge::Left
                } else if self.top.is_none_or(is_water)
                    && self.left.is_none_or(is_water)
                    && self.right.is_some_and(not_water)
                {
                    GroundTileEdge::Right
                } else if self.top.is_some_and(not_water)
                    && self.left.is_some_and(not_water)
                    && self.right.is_some_and(not_water)
                {
                    GroundTileEdge::LeftTopRight
                } else {
                    // bottom edge is rendered on land tile
                    GroundTileEdge::None
                    // panic!(
                    //     "Unhandled constellation: top: {:?}, left: {:?}, right: {:?}, bottom: {:?}",
                    //     self.top, self.left, self.right, self.bot
                    // )
                };
                result.insert(
                    TILE_LAYER_MIDDLE,
                    GroundTileVisual::Animated(GroundTileAnimationId {
                        tile_kind: self.tile_kind,
                        edge,
                    }),
                );
            }
        }
        result
    }
}

enum GroundTileVisual {
    Static {
        sprite_sheet: SpriteSheetId,
        atlas_idx: usize,
    },
    Animated(GroundTileAnimationId),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, EnumIter)]
enum GroundTileEdge {
    None,
    TopLeft,
    Top,
    TopRight,
    Left,
    Right,
    BotLeft,
    Bottom,
    BotRight,
    LeftTopRight,
}

#[derive(PartialEq, Eq, Hash, Debug)]
struct GroundTileAnimationId {
    tile_kind: GroundTile,
    edge: GroundTileEdge,
}
