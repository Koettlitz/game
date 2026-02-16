use std::{collections::HashSet, fmt::Debug};

use bevy::prelude::*;
use engine::{
    animation::{Animated, SpriteAnimation},
    assets::tile::{SpriteSheet, TILE_SIZE},
    overworld::tile::{GridPosition, GridSize, GridView, Neighbor, TileGrid},
    progress::ProgressState,
};
use tracing::instrument;

use super::spawn_ground_tile_grid;
use crate::{
    assets::tile::{
        AdjacentRequirementConfig, AdjacentRequirementsConfig, GroundTileVisualLayersConfig,
        TileKindVisualConfig,
    },
    tile::{GroundTileGrid, GroundTileKind, GroundTilesChanged, TileKindLoadingError},
};

type Result<T> = std::result::Result<T, TileKindLoadingError>;

pub struct TileVisualsPlugin;
impl Plugin for TileVisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<UpdateTileSprites>()
            .add_systems(
                Update,
                init_sprite_grid
                    .after(spawn_ground_tile_grid)
                    .run_if(in_state(ProgressState::Loading)),
            )
            .add_observer(on_ground_tile_changed)
            .add_systems(
                PostUpdate,
                update_sprites.run_if(in_state(ProgressState::Finished)),
            );
    }
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
    commands.insert_resource(TileSpriteGrid(TileGrid::with_size(&grid_size)));
    for pos in grid_size.iter() {
        message_writer.write(UpdateTileSprites(pos));
    }
    *initialized = true;
}

fn on_ground_tile_changed(
    event: On<GroundTilesChanged>,
    mut message_writer: MessageWriter<UpdateTileSprites>,
    grid_size: Res<GridSize>,
) {
    let mut sprites_to_update: HashSet<GridPosition> = HashSet::new();
    for position in &event.0 {
        for neighbor in position
            .adjacent(&grid_size)
            .iter_inclusive()
            .into_iter()
            .filter_map(|p| p)
        {
            sprites_to_update.insert(neighbor);
        }
    }
    for sprite_to_update in sprites_to_update {
        message_writer.write(UpdateTileSprites(sprite_to_update));
    }
}

fn update_sprites(
    mut commands: Commands,
    mut message_reader: MessageReader<UpdateTileSprites>,
    ground_tile_grid: Res<GroundTileGrid>,
    grid_size: Res<GridSize>,
    visuals_query: Query<(&GroundTileVisuals, &SpriteSheet), With<GroundTileKind>>,
    animations: Query<&SpriteAnimation>,
    mut sprites_grid: ResMut<TileSpriteGrid>,
) {
    for UpdateTileSprites(position) in message_reader.read() {
        let surroundings = ground_tile_grid.0.view_of(position.adjacent(&grid_size));
        let (visuals, sprite_sheet) = visuals_query
            .get(*surroundings.center())
            .expect("missing visual for ground tile");
        let layers = &visuals
            .0
            .iter()
            .find(|(req, _)| req.matches(&surroundings))
            .expect("no adjacent requirement matched the current surroundings")
            .1;
        let old_sprites = &sprites_grid.0[&position.as_index(&grid_size)];
        for old_sprite in old_sprites {
            commands.entity(*old_sprite).despawn();
        }
        let mut new_sprites = Vec::new();
        for (z, layer) in layers.iter() {
            let entity = spawn_tile_visuals(
                position,
                layer,
                z,
                sprite_sheet,
                &mut commands,
                animations,
                visuals_query,
                &grid_size,
                &ground_tile_grid.0,
            );
            if let Some(entity) = entity {
                new_sprites.push(entity);
            }
        }
        sprites_grid.0[&position.as_index(&grid_size)] = new_sprites;
    }
}

fn spawn_tile_visuals(
    position: &GridPosition,
    layer: &GroundTileVisual,
    z: f32,
    sprite_sheet: &SpriteSheet,
    commands: &mut Commands,
    animations: Query<&SpriteAnimation>,
    visuals_query: Query<(&GroundTileVisuals, &SpriteSheet), With<GroundTileKind>>,
    grid_size: &GridSize,
    tile_grid: &TileGrid<Entity>,
) -> Option<Entity> {
    let transform = Transform::from_translation(
        grid_pos_to_sprite_pos((*position).into(), grid_size.as_uvec2()).extend(z),
    );
    match layer {
        GroundTileVisual::Static(idx) => {
            let atlas = TextureAtlas {
                layout: sprite_sheet.layout.clone(),
                index: *idx,
            };
            let entity = commands
                .spawn((
                    Sprite {
                        image: sprite_sheet.image.clone(),
                        texture_atlas: Some(atlas),
                        ..Default::default()
                    },
                    transform,
                ))
                .id();
            Some(entity)
        }
        GroundTileVisual::Animated(animation_entity) => {
            let animation = animations
                .get(*animation_entity)
                .expect("missing tile sprite animation");
            let atlas = TextureAtlas {
                layout: sprite_sheet.layout.clone(),
                index: animation.current_idx(),
            };
            let sprite = Sprite {
                image: sprite_sheet.image.clone(),
                texture_atlas: Some(atlas),
                ..Default::default()
            };
            let entity = commands
                .spawn((sprite, Animated::by(*animation_entity), transform))
                .id();
            Some(entity)
        }
        GroundTileVisual::Neighbor(neighbor) => {
            let Some(neighbor_position) = position.neighbor(&neighbor, &grid_size) else {
                return None;
            };
            let neighbor_visual_entity = tile_grid[&neighbor_position.as_index(&grid_size)];
            let (neighbor_visuals, sprite_sheet) = visuals_query
                .get(neighbor_visual_entity)
                .expect("invalid entity in tile grid");
            return spawn_tile_visuals(
                &position,
                &neighbor_visuals.get_default().base,
                z,
                sprite_sheet,
                commands,
                animations,
                visuals_query,
                grid_size,
                tile_grid,
            );
        }
    }
}

fn grid_pos_to_sprite_pos(grid_pos: UVec2, grid_size: UVec2) -> Vec2 {
    let half_grid_size = grid_size.as_vec2() / 2.0;
    let sprite_pos = grid_pos.as_vec2() - half_grid_size;
    sprite_pos.with_y(-sprite_pos.y) * TILE_SIZE.as_vec2()
}

#[derive(Message)]
struct UpdateTileSprites(GridPosition);

#[derive(Resource)]
struct TileSpriteGrid(TileGrid<Vec<Entity>>);

#[derive(Component, Default)]
pub struct GroundTileVisuals(pub Vec<(AdjacentRequirements, GroundTileVisualLayers)>);

impl GroundTileVisuals {
    #[cfg_attr(
        debug_assertions,
        instrument(
            level = "trace",
            skip(ground_tile_kind_lookup, sprite_animation_lookup)
        )
    )]
    pub fn from_config<'a>(
        config: impl Iterator<
            Item = (
                &'a AdjacentRequirementsConfig,
                &'a GroundTileVisualLayersConfig,
            ),
        > + Debug,
        ground_tile_kind_lookup: impl Fn(&str) -> Result<Entity>,
        sprite_animation_lookup: impl Fn(&str) -> Result<Entity>,
    ) -> Option<Self> {
        let mut parsed_visuals = Vec::new();
        for (req, visuals) in config {
            let req = match AdjacentRequirements::from_config(req, &ground_tile_kind_lookup) {
                Ok(req) => req,
                Err(e) => {
                    if req.is_default() {
                        error!("could not link default tile kind visuals: {e}");
                        return None;
                    } else {
                        error!("could not link adjacentrequirements {req:?}: {e}");
                        continue;
                    }
                }
            };
            let visuals =
                match GroundTileVisualLayers::from_config(visuals, &sprite_animation_lookup) {
                    Ok(v) => v,
                    Err(e) => {
                        error!("could not link visual layers {visuals:?}: {e}");
                        continue;
                    }
                };
            parsed_visuals.push((req, visuals));
        }
        parsed_visuals.sort_by(|a, b| a.0.cmp(&b.0));
        Some(Self(parsed_visuals))
    }

    fn get_default(&self) -> &GroundTileVisualLayers {
        &self.0.last().expect("empty config").1
    }
}

#[derive(Debug)]
pub struct GroundTileVisualLayers {
    below: Vec<GroundTileVisual>,
    base: GroundTileVisual,
    above: Vec<GroundTileVisual>,
}

impl GroundTileVisualLayers {
    fn from_config(
        config: &GroundTileVisualLayersConfig,
        animation_lookup: impl Fn(&str) -> Result<Entity>,
    ) -> Result<Self> {
        Ok(Self {
            below: config
                .below
                .iter()
                .map(|c| GroundTileVisual::from_config(c, &animation_lookup))
                .filter_map(|result| result.inspect_err(|e| bevy::log::error!("{e}")).ok())
                .collect(),
            base: GroundTileVisual::from_config(&config.base, &animation_lookup)?,
            above: config
                .above
                .iter()
                .map(|c| GroundTileVisual::from_config(c, &animation_lookup))
                .filter_map(|result| result.inspect_err(|e| bevy::log::error!("{e}")).ok())
                .collect(),
        })
    }

    pub fn iter<'a>(&'a self) -> LayerIterator<'a> {
        LayerIterator::from(self)
    }
}

pub struct LayerIterator<'a> {
    layers: &'a GroundTileVisualLayers,
    current_layer: VisualLayer,
    current_idx: usize,
}
impl<'a> From<&'a GroundTileVisualLayers> for LayerIterator<'a> {
    fn from(value: &'a GroundTileVisualLayers) -> Self {
        Self {
            layers: value,
            current_layer: VisualLayer::Below,
            current_idx: 0,
        }
    }
}
impl<'a> Iterator for LayerIterator<'a> {
    type Item = (f32, &'a GroundTileVisual);
    fn next(&mut self) -> Option<Self::Item> {
        if matches!(self.current_layer, VisualLayer::Below) {
            if self.current_idx == self.layers.below.len() {
                self.current_layer = VisualLayer::Above;
                self.current_idx = 0;
                Some((VisualLayer::Base.z(), &self.layers.base))
            } else {
                let visual = &self.layers.below[self.current_idx];
                let z = VisualLayer::Below.z() + self.current_idx as f32;
                self.current_idx += 1;
                Some((z, visual))
            }
        } else if self.current_idx < self.layers.above.len() {
            let visual = &self.layers.above[self.current_idx];
            let z = VisualLayer::Above.z() + self.current_idx as f32;
            self.current_idx += 1;
            Some((z, visual))
        } else {
            None
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct AdjacentRequirements {
    pub top_left: AdjacentRequirement,
    pub top: AdjacentRequirement,
    pub top_right: AdjacentRequirement,
    pub left: AdjacentRequirement,
    pub right: AdjacentRequirement,
    pub bottom_left: AdjacentRequirement,
    pub bottom: AdjacentRequirement,
    pub bottom_right: AdjacentRequirement,
}

impl AdjacentRequirements {
    fn from_config(
        config: &AdjacentRequirementsConfig,
        entity_lookup: impl Fn(&str) -> Result<Entity>,
    ) -> Result<Self> {
        Ok(Self {
            top_left: AdjacentRequirement::from_config(&config.top_left, &entity_lookup)?,
            top: AdjacentRequirement::from_config(&config.top, &entity_lookup)?,
            top_right: AdjacentRequirement::from_config(&config.top_right, &entity_lookup)?,
            left: AdjacentRequirement::from_config(&config.left, &entity_lookup)?,
            right: AdjacentRequirement::from_config(&config.right, &entity_lookup)?,
            bottom_left: AdjacentRequirement::from_config(&config.bottom_left, &entity_lookup)?,
            bottom: AdjacentRequirement::from_config(&config.bottom, &entity_lookup)?,
            bottom_right: AdjacentRequirement::from_config(&config.bottom_right, &entity_lookup)?,
        })
    }

    fn matches(&self, surroundings: &GridView<Entity>) -> bool {
        let center = *surroundings.center();
        let result = self
            .all()
            .iter()
            .zip(surroundings.iter_exclusive())
            .all(|(req, neighbor)| req.matches(center, neighbor));
        result
    }

    pub fn all(&self) -> [&AdjacentRequirement; 8] {
        [
            &self.top_left,
            &self.top,
            &self.top_right,
            &self.left,
            &self.right,
            &self.bottom_left,
            &self.bottom,
            &self.bottom_right,
        ]
    }

    fn prio(&self) -> usize {
        self.all().into_iter().map(|n| n.prio()).sum()
    }
}

impl Ord for AdjacentRequirements {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.prio().cmp(&self.prio())
    }
}

impl PartialOrd for AdjacentRequirements {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Default, Debug, Eq, PartialEq, Clone)]
pub enum AdjacentRequirement {
    #[default]
    Any,
    Same,
    Other,
    Either(Vec<Entity>),
}

impl AdjacentRequirement {
    fn from_config(
        config: &AdjacentRequirementConfig,
        entity_lookup: impl Fn(&str) -> Result<Entity>,
    ) -> Result<Self> {
        Ok(match config {
            AdjacentRequirementConfig::Any => Self::Any,
            AdjacentRequirementConfig::Same => Self::Same,
            AdjacentRequirementConfig::Other => Self::Other,
            AdjacentRequirementConfig::Either(ids) => {
                let mut entities = Vec::with_capacity(ids.len());
                for id in ids {
                    entities.push(entity_lookup(id)?);
                }
                Self::Either(entities)
            }
        })
    }

    fn matches(&self, identity: Entity, other: Option<&Entity>) -> bool {
        match self {
            Self::Any => true,
            Self::Same => other.map(|n| *n == identity).unwrap_or(false),
            Self::Other => other.map(|n| *n != identity).unwrap_or(false),
            Self::Either(e) => other.map(|o| e.contains(o)).unwrap_or(false),
        }
    }

    fn prio(&self) -> usize {
        match self {
            Self::Any => 1,
            Self::Same | Self::Other => 10,
            Self::Either(_) => 100,
        }
    }
}

impl Ord for AdjacentRequirement {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.prio().cmp(&other.prio())
    }
}

impl PartialOrd for AdjacentRequirement {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
pub enum GroundTileVisual {
    Static(usize),
    Animated(Entity),
    Neighbor(Neighbor),
}

impl GroundTileVisual {
    fn from_config(
        config: &TileKindVisualConfig,
        animation_lookup: impl Fn(&str) -> Result<Entity>,
    ) -> Result<Self> {
        Ok(match config {
            TileKindVisualConfig::Static(idx) => Self::Static(*idx),
            TileKindVisualConfig::Animated { animation_id } => {
                Self::Animated(animation_lookup(&animation_id)?)
            }
            TileKindVisualConfig::Neighbor(neighbor) => Self::Neighbor(*neighbor),
        })
    }
}

enum VisualLayer {
    Below,
    Base,
    Above,
}

impl VisualLayer {
    fn z(&self) -> f32 {
        match self {
            Self::Below => 1.0,
            Self::Base => 10.0,
            Self::Above => 100.0,
        }
    }
}
