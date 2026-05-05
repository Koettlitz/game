use ron::ser::PrettyConfig;
use serde::Serialize;
use std::{fs, io, path::Path};
use thiserror::Error;

use bevy::{prelude::*, tasks::IoTaskPool};
use engine::{
    animation::Animated,
    asset::{
        AssetPathSpec, MissingAssetError,
        overworld::{
            lozo::{LozoAsset, LozoDef},
            tile::{TileDef, TileVisualKindDef, TileVisualsDef},
        },
    },
    overworld::tile::{GridPosition, GridSize, Passability},
};

use crate::{
    asset::{object::GameObjectKindAsset, tile::TileKindAsset},
    object::GameObject,
    tile::{
        GroundTileGrid,
        visuals::{TileSprite, TileSpriteGrid},
    },
};

pub struct ExportPlugin;
impl Plugin for ExportPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(export_lozo);
    }
}

#[derive(Event)]
pub struct ExportLozo;

pub fn export_lozo(
    _: On<ExportLozo>,
    tile_grid: Res<GroundTileGrid>,
    grid_size: Res<GridSize>,
    tile_kinds: Res<Assets<TileKindAsset>>,
    game_objects: Res<Assets<GameObjectKindAsset>>,
    sprite_grid: Res<TileSpriteGrid>,
    sprites_query: Query<(&TileSprite, &Sprite, Option<&Animated>)>,
    object_query: Query<(&GameObject, &Transform)>,
) -> Result<()> {
    let mut grid = Vec::new();
    for pos in grid_size.iter() {
        let tile_kind_handle = tile_grid.0[&pos.as_index(&grid_size)].handle();
        let tile_kind = tile_kinds
            .get(tile_kind_handle.id())
            .ok_or_else(|| MissingAssetError::new(tile_kind_handle.id()))?;
        let tile_def = create_tile_def(&tile_kind, &pos, &grid_size, &sprite_grid, &sprites_query)?;
        grid.push(Some(tile_def));
    }
    let mut lozo_def = LozoDef {
        width: grid_size.width(),
        height: grid_size.height(),
        tile_grid: grid,
        game_object_ids: Vec::new(),
    };

    for (game_object, transform) in &object_query {
        let object_kind_id = game_object.kind_ref().handle().id();
        let object_kind = game_objects
            .get(object_kind_id)
            .ok_or_else(|| MissingAssetError::new(object_kind_id))?;
        let object_pos = grid_size
            .to_grid_pos(transform.translation.truncate())
            .ok_or_else(|| PositionOutOfGridBoundsError::new(transform.translation.truncate()))?;
        let tile_def = lozo_def.tile_grid[*object_pos.as_index(&grid_size)]
            .get_or_insert_with(|| TileDef::default());
        tile_def.sprite_stack.push(TileVisualsDef {
            kind: Default::default(),
            image: object_kind.sprite_sheet.id().to_string(),
        });
        if let Some(ref collision_box) = object_kind.collision_box {
            for pos in CollisionBoxIter::from(collision_box) {
                let pos = (object_pos.as_uvec2().as_ivec2() + pos).as_vec2();
                let pos = GridPosition::new(pos, &grid_size)
                    .ok_or_else(|| PositionOutOfGridBoundsError::new(pos))?;
                let tile_def = lozo_def.tile_grid[*pos.as_index(&grid_size)]
                    .get_or_insert_with(|| TileDef::default());
                tile_def.passability &= Passability::Never;
            }
        }
    }
    IoTaskPool::get()
        .spawn(async move {
            save_lozo("world", lozo_def).expect("failed to save lozo");
        })
        .detach();
    Ok(())
}

fn save_lozo(id: &str, lozo: LozoDef) -> Result<()> {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("crate root {} has no parent dir?", crate_root.display()),
        )
    })?;
    let game_asset_root = workspace_root.join("game").join("assets");
    let base_path = LozoAsset::BASE_PATH
        .split_once("://")
        .map(|(_, base_path)| base_path)
        .unwrap_or_else(|| LozoAsset::BASE_PATH);
    let file_name = match LozoAsset::EXTENSION {
        Some(ext) => &format!("{id}.{ext}"),
        None => id,
    };
    let dir_path = game_asset_root.join(base_path);
    fs::create_dir_all(&dir_path)?;
    let path = dir_path.join(file_name);
    info!("writing lozo to {}", path.display());
    let serialized = ron::ser::to_string_pretty(&lozo, PrettyConfig::default())?;
    fs::write(path, serialized)?;
    Ok(())
}

fn create_tile_def(
    tile_kind: &TileKindAsset,
    pos: &GridPosition,
    grid_size: &GridSize,
    sprite_grid: &TileSpriteGrid,
    sprites_query: &Query<(&TileSprite, &Sprite, Option<&Animated>)>,
) -> Result<TileDef> {
    let mut visuals = Vec::new();
    for tile_sprite in &sprite_grid[&pos.as_index(&grid_size)] {
        let (tile_sprite, sprite, animated) = sprites_query.get(*tile_sprite)?;
        let kind = match animated {
            Some(animated) => TileVisualKindDef::Animated {
                animation: animated.id().to_string(),
            },
            None => TileVisualKindDef::Static {
                idx: sprite
                    .texture_atlas
                    .as_ref()
                    .map(|atlas| atlas.index)
                    .unwrap_or(0),
            },
        };
        let visual = TileVisualsDef {
            kind,
            image: tile_sprite.id().to_string(),
        };
        visuals.push(visual);
    }
    let tile_def = TileDef {
        passability: tile_kind.passability,
        sprite_stack: visuals,
    };
    Ok(tile_def)
}

fn _save_def<D: Serialize, S: AssetPathSpec>(_id: &str, _def: &D) -> io::Result<()> {
    todo!()
}

struct CollisionBoxIter<'a> {
    collision_box: &'a IRect,
    current: Option<IVec2>,
}

impl<'a> From<&'a IRect> for CollisionBoxIter<'a> {
    fn from(collision_box: &'a IRect) -> Self {
        Self {
            collision_box,
            current: Some(collision_box.min),
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

#[derive(Error, Debug)]
#[error("position out of bounds: {0}")]
struct PositionOutOfGridBoundsError(Vec2);

impl PositionOutOfGridBoundsError {
    fn new(pos: impl Into<Vec2>) -> Self {
        Self(pos.into())
    }
}
