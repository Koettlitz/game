use ron::ser::PrettyConfig;
use serde::Serialize;
use std::{collections::HashMap, fs};

use bevy::{
    asset::{
        AssetPath,
        io::{AssetSourceId, file::FileAssetReader},
    },
    log,
    prelude::*,
    tasks::IoTaskPool,
};
use bevy_elf::{AssetResolver, HasResolver};
use engine::{
    asset::AssetsExt,
    overworld::{
        lozo::{LozoAsset, LozoDef},
        object::{GameObjectSpriteDef, SpriteKindDef, TextureAtlasDataDef},
        tile::{
            CameraAnimationDef, Grid, GridPosition, GridSize, Passability, TileDef, TileEdge,
            TileEventActionDef, TileVisualKindDef, TileVisualsAsset, TileVisualsDef,
        },
    },
};

use crate::{
    asset::{
        object::{Door, GameObjectKindAsset},
        tile::TileKindAsset,
    },
    object::{GameObject, GameObjectSprite},
    tile::{
        Tile,
        edge::{AnimationId, TileSprite},
    },
};

pub struct ExportPlugin;
impl Plugin for ExportPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(create_grid.pipe(add_objects.map(|result: Result| {
            if let Err(e) = result {
                log::error!("{e}");
            }
        })));
    }
}

#[derive(Event)]
pub struct ExportLozo;

fn create_grid(
    _: On<ExportLozo>,
    tile_grid: Single<(&Grid<Option<Tile>>, &GridSize)>,
    tile_kinds: Res<Assets<TileKindAsset>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
    sprites_query: Query<(&TileSprite, &Sprite, Option<&AnimationId>, &Transform)>,
) -> Result<Vec<Option<TileDef>>> {
    let (tile_grid, grid_size) = tile_grid.into_inner();
    let mut grid = Vec::new();
    let mut layout_map = HashMap::new();

    for pos in grid_size.iter_all() {
        let Some(tile) = &tile_grid[pos] else {
            grid.push(None);
            continue;
        };
        let tile_kind_handle = tile.kind.handle();
        let tile_kind = tile_kinds.require_handle(tile_kind_handle)?;
        let mut visuals = Vec::new();
        for tile_sprite in &tile.sprite_stack {
            let (sprite_tag, sprite, animated, transform) = sprites_query.get(*tile_sprite)?;
            let atlas = sprite
                .texture_atlas
                .as_ref()
                .ok_or("missing texture atlas on tile sprite")?;
            let layout = layouts.require_handle(&atlas.layout)?;
            let layout_id = sprite_tag.id().to_string();
            layout_map
                .entry(layout_id.clone())
                .or_insert_with(|| layout.clone());
            let kind = match animated {
                Some(animation_id) => TileVisualKindDef::Animated {
                    animation: animation_id.0.clone(),
                },
                None => TileVisualKindDef::Static { idx: atlas.index },
            };
            let visual = TileVisualsDef {
                spritesheet: sprite_tag.id().to_string(),
                layout: layout_id,
                kind,
                z: transform.translation.z,
            };
            visuals.push(visual);
        }

        grid.push(Some(TileDef {
            passability: tile_kind.passability,
            sprite_stack: visuals,
        }));
    }

    flush_assets(layout_map, TileVisualsAsset::layout_resolver());
    Ok(grid)
}

fn add_objects(
    grid: In<Result<Vec<Option<TileDef>>>>,
    grid_size: Single<&GridSize>,
    game_objects: Res<Assets<GameObjectKindAsset>>,
    object_query: Query<(&GameObject, &Transform, &Children)>,
    object_sprite_query: Query<(&GameObjectSprite, &Sprite, &GlobalTransform)>,
) -> Result {
    let mut grid = grid.0?;
    let mut object_sprite_map = HashMap::new();
    let mut char_left_events: HashMap<TileEdge, Vec<TileEventActionDef>> = HashMap::new();
    let mut char_entered_events: HashMap<TileEdge, Vec<TileEventActionDef>> = HashMap::new();
    let mut char_reached_events: HashMap<TileEdge, Vec<TileEventActionDef>> = HashMap::new();

    for (game_object, transform, children) in &object_query {
        let object_kind_id = game_object.kind_ref().id();
        let object_kind = game_objects.require_handle(game_object.kind_ref().handle())?;

        if let Some(ref collision_box) = object_kind.collision_box() {
            let object_pos = grid_size
                .world_to_grid(transform.translation.truncate())
                .ok_or_else(|| {
                    format!(
                        "position out of bounds: {}",
                        transform.translation.truncate()
                    )
                })?;
            for pos in CollisionBoxIter::from(collision_box) {
                let pos = object_pos.as_ivec2() + pos;
                let pos = GridPosition::new(UVec2::new(pos.x as u32, pos.y as u32), &grid_size)
                    .ok_or_else(|| format!("position out of bounds: {}", pos.as_vec2()))?;
                let tile_def = grid[*pos.as_index()].get_or_insert_with(TileDef::default);
                tile_def.passability &= Passability::Never;
            }
        }

        for child in children {
            let (sprite_tag, sprite, sprite_transform) = object_sprite_query.get(*child)?;
            let sprite_kind = sprite
                .texture_atlas
                .as_ref()
                .map(|atlas| TextureAtlasDataDef {
                    layout: object_kind_id.to_string(),
                    kind: SpriteKindDef::Static { idx: atlas.index },
                });

            let sprite_def = GameObjectSpriteDef {
                image: object_kind_id.to_string(),
                sprite_kind,
                world_position: sprite_transform.translation(),
            };
            match sprite_tag {
                GameObjectSprite::Main { id } => {
                    object_sprite_map.entry(id.clone()).or_insert(sprite_def);
                }
                GameObjectSprite::Door { id, door } => {
                    object_sprite_map.entry(id.clone()).or_insert(sprite_def);

                    let door_pos = grid_size
                        .world_to_grid(transform.translation.truncate() + door.offset().as_vec2())
                        .ok_or("door position out of bounds")?;
                    let door_tile = grid[*door_pos.as_index()].get_or_insert_with(TileDef::default);

                    door_tile.passability = Passability::Always;
                    register_door_events(
                        id,
                        door,
                        &door_pos,
                        &mut char_left_events,
                        &mut char_entered_events,
                        &mut char_reached_events,
                    )?;
                }
            }
        }
    }

    let object_ids = object_sprite_map.keys().cloned().collect();
    flush_assets(object_sprite_map, LozoAsset::objects_resolver());

    let lozo_def = LozoDef {
        width: grid_size.width(),
        height: grid_size.height(),
        tile_grid: grid,
        char_left_events,
        char_entered_events,
        char_reached_events,
        objects: object_ids,
    };
    flush_assets(vec![("world".to_string(), lozo_def)], LozoAsset::resolver());

    Ok(())
}

fn register_door_events(
    sprite_id: &str,
    door: &Door,
    door_pos: &GridPosition,
    char_left_events: &mut HashMap<TileEdge, Vec<TileEventActionDef>>,
    char_entered_events: &mut HashMap<TileEdge, Vec<TileEventActionDef>>,
    char_reached_events: &mut HashMap<TileEdge, Vec<TileEventActionDef>>,
) -> Result {
    let Some(next_to_door) = door_pos.bottom() else {
        return Ok(());
    };
    let to_door_edge = TileEdge {
        from: next_to_door.as_uvec2(),
        to: door_pos.as_uvec2(),
    };
    char_left_events
        .entry(to_door_edge.clone())
        .or_default()
        .push(TileEventActionDef::CameraAnimation(
            CameraAnimationDef::ZoomWarp { reverse: false },
        ));
    char_reached_events
        .entry(to_door_edge)
        .or_default()
        .push(TileEventActionDef::ActivateNextLozo);

    for next_to_door_neighbor in next_to_door
        .reachable_neigbors()
        .into_iter()
        .flatten()
        .map(|pos| pos.as_uvec2())
        .filter(|pos| *pos != door_pos.as_uvec2())
    {
        let to_next_to_door_edge = TileEdge {
            from: next_to_door_neighbor,
            to: next_to_door.as_uvec2(),
        };
        let from_next_to_door_edge = to_next_to_door_edge.reverse();

        char_left_events
            .entry(to_next_to_door_edge.clone())
            .or_default()
            .push(TileEventActionDef::LoadNextLozo {
                next_lozo_id: door.target_lozo().to_string(),
                after_animation: Some(CameraAnimationDef::ZoomWarp { reverse: true }),
            });
        char_left_events
            .entry(from_next_to_door_edge.clone())
            .or_default()
            .push(TileEventActionDef::UnloadNextLozo);

        char_entered_events
            .entry(to_next_to_door_edge)
            .or_default()
            .push(TileEventActionDef::SpriteAnimation {
                sprite_id: sprite_id.to_owned(),
                animation: door.open_animation_path()?,
            });
        char_entered_events
            .entry(from_next_to_door_edge)
            .or_default()
            .push(TileEventActionDef::SpriteAnimation {
                sprite_id: sprite_id.to_owned(),
                animation: door.close_animation_path()?,
            });
    }

    Ok(())
}

fn flush_assets<A: Serialize>(
    assets: impl IntoIterator<Item = (String, A)> + Send + 'static,
    resolver: impl AssetResolver + Send + 'static,
) {
    IoTaskPool::get()
        .spawn(async move {
            for (id, asset) in assets {
                let asset_path = resolver
                    .resolve(&id)
                    .unwrap_or_else(|e| panic!("failed to resolve asset path: {e}"));
                write_asset(asset_path, asset).expect("failed to save asset");
            }
        })
        .detach();
}

fn write_asset<A: Serialize>(asset_path: AssetPath, asset: A) -> Result<()> {
    let base_path = FileAssetReader::get_base_path();
    let source_folder = match asset_path.source() {
        AssetSourceId::Default => "assets",
        AssetSourceId::Name(name) => &format!("{}/assets", name.as_ref()),
    };
    let file_path = base_path.join(source_folder).join(asset_path.path());
    if let Some(dir_path) = file_path.parent()
        && !dir_path.exists()
    {
        info!("ensuring parent dirs exist for {}", dir_path.display());
        fs::create_dir_all(dir_path)?;
    }
    info!(
        "writing asset to {asset_path} => \"{}\"",
        file_path.display()
    );
    let serialized = ron::ser::to_string_pretty(&asset, PrettyConfig::default())?;
    fs::write(file_path, serialized)?;
    Ok(())
}

struct CollisionBoxIter<'a> {
    collision_box: &'a IRect,
    current: Option<IVec2>,
}

impl<'a> From<&'a IRect> for CollisionBoxIter<'a> {
    fn from(collision_box: &'a IRect) -> Self {
        Self {
            collision_box,
            current: (!collision_box.is_empty()).then_some(collision_box.min),
        }
    }
}

impl<'a> Iterator for CollisionBoxIter<'a> {
    type Item = IVec2;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current?;
        let next = current.with_x(current.x + 1);
        self.current = if self.collision_box.contains(next) {
            Some(next)
        } else {
            let next = IVec2::new(self.collision_box.min.x, current.y + 1);
            if self.collision_box.contains(next) {
                Some(next)
            } else {
                None
            }
        };
        Some(current)
    }
}
