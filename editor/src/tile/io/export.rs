use std::collections::HashMap;

use bevy::prelude::*;
use engine::{
    asset::AssetsExt,
    overworld::tile::{
        Grid, GridSize, TileDef, TileVisualKindDef, TileVisualsAsset, TileVisualsDef,
    },
};

use crate::{
    io::resolve_and_flush,
    lozo::LozoExportInitialized,
    tile::{
        Tile,
        asset::TileKindAsset,
        edge::{AnimationId, TileSprite},
    },
};

pub struct ExportPlugin;

impl Plugin for ExportPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(export_grid);
    }
}

#[derive(EntityEvent)]
pub struct TileGridExported {
    pub lozo_entity: Entity,
    #[event_target]
    pub export_entity: Entity,
}

#[derive(Component)]
pub struct TileGridExport {
    pub width: u32,
    pub height: u32,
    pub grid: Vec<Option<TileDef>>,
}

fn export_grid(
    event: On<LozoExportInitialized>,
    tile_grid: Query<(&Grid<Option<Tile>>, &GridSize)>,
    tile_kinds: Res<Assets<TileKindAsset>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
    sprites_query: Query<(&TileSprite, &Sprite, Option<&AnimationId>, &Transform)>,
    mut commands: Commands,
) -> Result {
    let (tile_grid, grid_size) = tile_grid.get(event.lozo_entity)?;
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

            visuals.push(TileVisualsDef {
                spritesheet: sprite_tag.id().to_string(),
                layout: layout_id,
                kind,
                z: transform.translation.z,
            });
        }

        grid.push(Some(TileDef {
            passability: tile_kind.passability,
            blocked: false,
            sprite_stack: visuals,
        }));
    }

    resolve_and_flush(layout_map, TileVisualsAsset::layout_resolver());
    commands
        .entity(event.event_target())
        .insert(TileGridExport {
            width: grid_size.width(),
            height: grid_size.height(),
            grid,
        });

    commands.trigger(TileGridExported {
        lozo_entity: event.lozo_entity,
        export_entity: event.export_entity,
    });
    Ok(())
}
