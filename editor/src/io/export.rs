use bevy_entity_lookup::EntityId;
use bevy_spawn_phase_events::{InSpawnPhase, LozoAppExt, SpawnPhase, SpawnPhaseCompleted};
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
use bevy_elf::{AssetResolver, HasResolver};
use engine::{
    asset::AssetsExt,
    overworld::{
        character::{
            Orientation,
            asset::{CharacterBehaviourDef, CharacterDef, Route},
        },
        event::{CameraAnimationDef, TileEventActionDef},
        lozo::{LozoAsset, LozoDef},
        object::{GameObjectSpriteDef, SpriteKindDef, TextureAtlasDataDef},
        tile::{
            Grid, GridPosition, GridSize, Passability, TileDef, TileEdge, TileVisualKindDef,
            TileVisualsAsset, TileVisualsDef,
        },
    },
};

use crate::{
    character::{Character, asset::CharacterKindAsset},
    object::{
        GameObject, GameObjectSprite,
        asset::{Door, GameObjectKindAsset},
    },
    tile::{
        Tile,
        asset::TileKindAsset,
        edge::{AnimationId, TileSprite},
    },
};

pub struct ExportPlugin;
impl Plugin for ExportPlugin {
    fn build(&self, app: &mut App) {
        app.register_spawn_event::<CharactersExported>()
            .register_spawn_event::<GameObjectsExported>()
            .add_observer(init_lozo_export)
            .add_observer(export_grid)
            .add_observer(export_objects)
            .add_observer(export_characters)
            .add_observer(commit_lozo_export);
    }
}

#[derive(Event)]
pub struct ExportLozo;

#[derive(Event)]
struct LozoExportInitialized(Entity);

#[derive(Component)]
struct LozoExport;

#[derive(Component, Default)]
struct EventsExport {
    char_left_events: HashMap<TileEdge, Vec<TileEventActionDef>>,
    char_entered_events: HashMap<TileEdge, Vec<TileEventActionDef>>,
    char_reached_events: HashMap<TileEdge, Vec<TileEventActionDef>>,
}

fn init_lozo_export(_: On<ExportLozo>, mut commands: Commands) {
    let entity = commands.spawn((LozoExport, EventsExport::default())).id();
    commands.trigger(LozoExportInitialized(entity));
}

#[derive(Event)]
pub struct TileGridExported(Entity);

#[derive(Component)]
struct TileGridExport {
    width: u32,
    height: u32,
    grid: Vec<Option<TileDef>>,
}

fn export_grid(
    event: On<LozoExportInitialized>,
    tile_grid: Single<(&Grid<Option<Tile>>, &GridSize)>,
    tile_kinds: Res<Assets<TileKindAsset>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
    sprites_query: Query<(&TileSprite, &Sprite, Option<&AnimationId>, &Transform)>,
    mut commands: Commands,
) -> Result {
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
            blocked: false,
            sprite_stack: visuals,
        }));
    }

    flush_assets(layout_map, TileVisualsAsset::layout_resolver());
    commands.entity(event.0).insert(TileGridExport {
        width: grid_size.width(),
        height: grid_size.height(),
        grid,
    });

    commands.trigger(TileGridExported(event.0));
    Ok(())
}

struct ExportLozoObjects;

impl SpawnPhase for ExportLozoObjects {
    type InitialComponent = LozoExport;
}

#[derive(Event)]
struct GameObjectsExported(Entity);

impl InSpawnPhase for GameObjectsExported {
    type SpawnPhase = ExportLozoObjects;

    fn entity(&self) -> Entity {
        self.0
    }
}

#[derive(Component)]
struct GameObjectExport {
    pub objects: Vec<String>,
}

fn export_objects(
    event: On<TileGridExported>,
    mut grid: Query<(&mut TileGridExport, &mut EventsExport)>,
    grid_size: Single<&GridSize>,
    game_objects: Res<Assets<GameObjectKindAsset>>,
    object_query: Query<(&GameObject, &Transform, &Children)>,
    object_sprite_query: Query<(&GameObjectSprite, &Sprite, &GlobalTransform)>,
    mut commands: Commands,
) -> Result {
    let (mut grid, mut events) = grid.get_mut(event.0)?;
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
                let pos = GridPosition::new(UVec2::new(pos.x as u32, pos.y as u32), &grid_size)
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
    flush_assets(object_sprite_map, LozoAsset::objects_resolver());
    commands.entity(event.0).insert(GameObjectExport {
        objects: object_ids,
    });

    commands.trigger(GameObjectsExported(event.0));
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

#[derive(Event)]
struct CharactersExported(Entity);

impl InSpawnPhase for CharactersExported {
    type SpawnPhase = ExportLozoObjects;

    fn entity(&self) -> Entity {
        self.0
    }
}

#[derive(Component)]
struct CharacterExport {
    characters: Vec<String>,
}

fn export_characters(
    event: On<TileGridExported>,
    character_kinds: Res<Assets<CharacterKindAsset>>,
    character_query: Query<(&Character, &Transform)>,
    grid_size: Single<&GridSize>,
    mut grid_export: Query<&mut TileGridExport>,
    mut commands: Commands,
) -> Result {
    let mut characters = HashMap::new();
    for (i, (character, transform)) in character_query.iter().enumerate() {
        let character_kind = character_kinds.require_handle(character.asset_ref.handle())?;
        let Some(position) = grid_size.world_to_grid(transform.translation.truncate()) else {
            return Err(BevyError::from(format!(
                "character position out of grid bounds {}",
                transform.translation
            )));
        };

        if let Some(ref mut tile_def) = grid_export.get_mut(event.0)?.grid[*position.as_index()] {
            tile_def.blocked = true;
        }

        characters.insert(
            format!("{}_{i}", character.asset_ref.id()),
            CharacterDef {
                dialog: Some("Rück mir nich so auf die Pelle!".to_string()),
                spritesheet: character_kind.spritesheet.clone().into_def(),
                animations: character_kind
                    .animations
                    .iter()
                    .map(|(state, visual)| (state.clone().into(), visual.clone().into_def()))
                    .collect(),
                position: transform.translation,
                orientation: character.orientation,
                behaviour: default_route(*position, &character.orientation)
                    .map(|r| CharacterBehaviourDef::Walking(r)),
            },
        );
    }

    let character_ids = characters.keys().cloned().collect();
    flush_assets(characters, LozoAsset::characters_resolver());
    let entity = commands
        .entity(event.0)
        .insert(CharacterExport {
            characters: character_ids,
        })
        .id();

    commands.trigger(CharactersExported(entity));
    Ok(())
}

fn default_route(start: UVec2, orientation: &Orientation) -> Option<Route> {
    let first: UVec2 = (start.as_ivec2() + orientation.grid_direction() * 4)
        .try_into()
        .ok()?;
    let second: UVec2 = (first.as_ivec2() + orientation.rotate().grid_direction() * 4)
        .try_into()
        .ok()?;
    let third = (second.as_ivec2() + orientation.rotate().rotate().grid_direction() * 4)
        .try_into()
        .ok()?;

    Some(Route {
        targets: vec![first, second, third, start],
        cycle: true,
    })
}

fn commit_lozo_export(event: On<SpawnPhaseCompleted<ExportLozoObjects>>, mut commands: Commands) {
    commands
        .entity(event.entity())
        .queue(|mut entity: EntityWorldMut| {
            let Some((tile_grid, game_objects, characters, events)) = entity.take::<(
                TileGridExport,
                GameObjectExport,
                CharacterExport,
                EventsExport,
            )>() else {
                return Err(BevyError::from(
                    "commit_lozo_export triggered, but not all the export components were there.",
                ));
            };
            entity.despawn();

            let lozo_def = LozoDef {
                width: tile_grid.width,
                height: tile_grid.height,
                tile_grid: tile_grid.grid,
                char_left_events: events.char_left_events,
                char_entered_events: events.char_entered_events,
                char_reached_events: events.char_reached_events,
                objects: game_objects.objects,
                characters: characters.characters,
            };
            flush_assets(vec![("world".to_string(), lozo_def)], LozoAsset::resolver());
            Ok(())
        });
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
