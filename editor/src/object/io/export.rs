use std::collections::HashMap;

use bevy::prelude::*;
use bevy_entity_lookup::EntityId;
use bevy_event_cascade::{EventCascadeAppExt, InStage};
use engine::{
    asset::AssetsExt,
    overworld::{
        event::{CameraAnimationDef, TileEventActionDef},
        lozo::LozoAsset,
        object::{GameObjectSpriteDef, SpriteKindDef, TextureAtlasDataDef},
        tile::{GridPosition, GridSize, Passability, TileDef, TileEdge},
    },
};

use crate::{
    io::resolve_and_flush,
    lozo::{EventsExport, ExportLozoObjects},
    object::{
        GameObject, GameObjectSprite,
        asset::{Door, GameObjectKindAsset},
    },
    tile::io::{TileGridExport, TileGridExported},
};

pub struct ExportPlugin;

impl Plugin for ExportPlugin {
    fn build(&self, app: &mut App) {
        app.register_event::<GameObjectsExported>()
            .add_observer(export_objects);
    }
}

#[derive(EntityEvent)]
struct GameObjectsExported(Entity);

impl InStage for GameObjectsExported {
    type Stage = ExportLozoObjects;
}

#[derive(Component)]
pub struct GameObjectExport {
    pub objects: Vec<String>,
}

fn export_objects(
    event: On<TileGridExported>,
    mut grid: Query<(&mut TileGridExport, &mut EventsExport)>,
    grid_size: Query<&GridSize>,
    game_objects: Res<Assets<GameObjectKindAsset>>,
    object_query: Query<(&GameObject, &Transform, &Children)>,
    object_sprite_query: Query<(&GameObjectSprite, &Sprite, &GlobalTransform)>,
    mut commands: Commands,
) -> Result {
    let grid_size = grid_size.get(event.lozo_entity)?;
    let (mut grid, mut events) = grid.get_mut(event.event_target())?;
    let mut object_sprite_map = HashMap::new();

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
                let pos = GridPosition::new(UVec2::new(pos.x as u32, pos.y as u32), grid_size)
                    .ok_or_else(|| format!("position out of bounds: {}", pos.as_vec2()))?;
                let tile_def = grid.grid[*pos.as_index()].get_or_insert_with(TileDef::default);
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

            if let GameObjectSprite::Door { id, door } = sprite_tag {
                let door_pos = grid_size
                    .world_to_grid(transform.translation.truncate() + door.offset().as_vec2())
                    .ok_or("door position out of bounds")?;
                let door_tile =
                    grid.grid[*door_pos.as_index()].get_or_insert_with(TileDef::default);

                door_tile.passability = Passability::Always;
                register_door_events(id, door, &door_pos, &mut events)?;
            }

            object_sprite_map
                .entry(sprite_tag.id().to_owned())
                .or_insert(GameObjectSpriteDef {
                    id: EntityId::new(sprite_tag.id().to_owned()),
                    image: object_kind_id.to_string(),
                    sprite_kind,
                    world_position: sprite_transform.translation(),
                });
        }
    }

    let object_ids = object_sprite_map.keys().cloned().collect();
    resolve_and_flush(object_sprite_map, LozoAsset::objects_resolver());
    commands
        .entity(event.event_target())
        .insert(GameObjectExport {
            objects: object_ids,
        });

    commands.trigger(GameObjectsExported(event.event_target()));
    Ok(())
}

fn register_door_events(
    sprite_id: &str,
    door: &Door,
    door_pos: &GridPosition,
    events: &mut EventsExport,
) -> Result {
    let Some(next_to_door) = door_pos.bottom() else {
        return Ok(());
    };
    let to_door_edge = TileEdge {
        from: next_to_door.as_uvec2(),
        to: door_pos.as_uvec2(),
    };
    events
        .char_left_events
        .entry(to_door_edge.clone())
        .or_default()
        .push(TileEventActionDef::CameraAnimation(
            CameraAnimationDef::ZoomWarp { reverse: false },
        ));
    events
        .char_reached_events
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

        events
            .char_left_events
            .entry(to_next_to_door_edge.clone())
            .or_default()
            .push(TileEventActionDef::LoadNextLozo {
                next_lozo_id: door.target_lozo().to_string(),
                after_animation: Some(CameraAnimationDef::ZoomWarp { reverse: true }),
            });
        events
            .char_left_events
            .entry(from_next_to_door_edge.clone())
            .or_default()
            .push(TileEventActionDef::UnloadNextLozo);

        events
            .char_entered_events
            .entry(to_next_to_door_edge)
            .or_default()
            .push(TileEventActionDef::SpriteAnimation {
                sprite_entity: bevy_entity_lookup::EntityRef::new(sprite_id.to_owned()),
                animation: door.open_animation_path()?,
            });
        events
            .char_entered_events
            .entry(from_next_to_door_edge)
            .or_default()
            .push(TileEventActionDef::SpriteAnimation {
                sprite_entity: bevy_entity_lookup::EntityRef::new(sprite_id.to_owned()),
                animation: door.close_animation_path()?,
            });
    }

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
