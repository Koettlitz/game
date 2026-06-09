use ron::ser::PrettyConfig;
use serde::Serialize;
use std::{collections::HashMap, fs};

use bevy::{
    asset::{
        AssetPath,
        io::{AssetSourceId, file::FileAssetReader},
    },
    prelude::*,
    tasks::IoTaskPool,
};
use engine::{
    asset::{
        AssetResolver, HasResolver,
        overworld::{
            lozo::{LozoAsset, LozoDef},
            object::{GameObjectSpriteDef, TextureAtlasData, TextureAtlasDataDef},
            tile::{TileDef, TileVisualKindDef, TileVisualsAsset, TileVisualsDef},
        },
        spritesheet::SpriteKindDef,
    },
    overworld::tile::{Grid, GridPosition, GridSize, Passability},
};

use crate::{
    asset::{object::GameObjectKindAsset, tile::TileKindAsset},
    object::{GameObject, GameObjectSprite},
    tile::{
        Tile,
        edge::{AnimationId, TileSprite},
    },
};

pub struct ExportPlugin;
impl Plugin for ExportPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(create_grid.pipe(add_objects));
    }
}

trait AssetsExt<A: Asset> {
    fn require(&self, id: AssetId<A>) -> &A;
    fn require_handle(&self, handle: &Handle<A>) -> &A {
        self.require(handle.id())
    }

    // fn require_mut(&mut self, id: AssetId<A>) -> &mut A;
    // fn require_handle_mut(&mut self, handle: &Handle<A>) -> &mut A {
    //     self.require_mut(handle.id())
    // }
}

impl<A: Asset> AssetsExt<A> for Assets<A> {
    fn require(&self, id: AssetId<A>) -> &A {
        let asset_type = A::type_ident().unwrap_or_else(|| A::type_path());
        self.get(id)
            .unwrap_or_else(|| panic!("missing {asset_type}: \"{}\"", id))
    }

    // fn require_mut(&mut self, id: AssetId<A>) -> &mut A {
    //     let asset_type = A::type_ident().unwrap_or_else(|| A::type_path());
    //     self.get_mut(id)
    //         .unwrap_or_else(|| panic!("missing {asset_type}: \"{}\"", id))
    // }
}

#[derive(Event)]
pub struct ExportLozo;

fn create_grid(
    _: On<ExportLozo>,
    tile_grid: Single<(&Grid<Option<Tile>>, &GridSize)>,
    tile_kinds: Res<Assets<TileKindAsset>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
    sprites_query: Query<(&TileSprite, &Sprite, Option<&AnimationId>)>,
) -> Vec<Option<TileDef>> {
    let (tile_grid, grid_size) = tile_grid.into_inner();
    let mut grid = Vec::new();
    let mut layout_map = HashMap::new();

    for pos in grid_size.iter_all() {
        let Some(tile) = &tile_grid[pos] else {
            grid.push(None);
            continue;
        };
        let tile_kind_handle = tile.kind.handle();
        let tile_kind = tile_kinds.require_handle(tile_kind_handle);
        let mut visuals = Vec::new();
        for tile_sprite in &tile.sprite_stack {
            let (sprite_tag, sprite, animated) = sprites_query
                .get(*tile_sprite)
                .unwrap_or_else(|e| panic!("missing tile sprite {tile_sprite} - {e}"));
            let atlas = sprite
                .texture_atlas
                .as_ref()
                .unwrap_or_else(|| panic!("missing texture atlas on tile sprite"));
            let layout = layouts.require_handle(&atlas.layout);
            let layout_id = sprite_tag.id().to_string();
            layout_map
                .entry(layout_id.clone())
                .or_insert(layout.clone());
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
            };
            visuals.push(visual);
        }

        grid.push(Some(TileDef {
            passability: tile_kind.passability,
            sprite_stack: visuals,
        }));
    }

    flush_assets(layout_map, TileVisualsAsset::layout_resolver());
    grid
}

fn add_objects(
    mut grid: In<Vec<Option<TileDef>>>,
    grid_size: Single<&GridSize>,
    game_objects: Res<Assets<GameObjectKindAsset>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
    object_query: Query<(&GameObject, &Transform, &Children)>,
    object_sprite_query: Query<(&GameObjectSprite, &Sprite, &GlobalTransform)>,
) {
    let mut object_sprite_map = HashMap::new();
    let mut layout_map = HashMap::new();
    for (game_object, transform, children) in &object_query {
        let object_kind_id = game_object.kind_ref().id();
        let object_kind = game_objects.require_handle(game_object.kind_ref().handle());

        for child in children {
            let (sprite_tag, sprite, sprite_transform) = object_sprite_query
                .get(*child)
                .unwrap_or_else(|e| panic!("missing object sprite {child} - {e}"));
            let sprite_kind = sprite.texture_atlas.as_ref().map(|atlas| {
                layout_map
                    .entry(object_kind_id.to_string())
                    .or_insert_with(|| layouts.require_handle(&atlas.layout).clone());
                TextureAtlasDataDef {
                    layout: object_kind_id.to_string(),
                    kind: SpriteKindDef::Static {
                        idx: sprite.texture_atlas.as_ref().unwrap().index,
                    },
                }
            });

            let sprite_def = GameObjectSpriteDef {
                image: object_kind.spritesheet().image().id().to_string(),
                sprite_kind,
                world_position: sprite_transform.translation(),
            };
            object_sprite_map
                .entry(sprite_tag.id().to_string())
                .or_insert(sprite_def);
        }
        if let Some(ref collision_box) = object_kind.collision_box() {
            let object_pos = grid_size
                .world_to_grid(transform.translation.truncate())
                .unwrap_or_else(|| {
                    panic!(
                        "position out of bounds: {}",
                        transform.translation.truncate()
                    )
                });
            for pos in CollisionBoxIter::from(collision_box) {
                let pos = object_pos.as_ivec2() + pos;
                let pos = GridPosition::new(UVec2::new(pos.x as u32, pos.y as u32), &grid_size)
                    .unwrap_or_else(|| panic!("position out of bounds: {}", pos.as_vec2()));
                let tile_def = grid[*pos.as_index()].get_or_insert_with(|| TileDef::default());
                tile_def.passability &= Passability::Never;
            }
        }
    }

    flush_assets(layout_map, TextureAtlasData::layout_resolver());

    let object_ids = object_sprite_map.keys().cloned().collect();
    flush_assets(object_sprite_map, LozoAsset::objects_resolver());

    let lozo_def = LozoDef {
        width: grid_size.width(),
        height: grid_size.height(),
        tile_grid: grid.0,
        objects: object_ids,
    };
    flush_assets(vec![("world".to_string(), lozo_def)], LozoAsset::resolver());
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
