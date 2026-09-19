use crate::{
    animation::{Animated, AnimationAdvanced, AnimationUpdate},
    asset::AssetsExt,
    overworld::{
        character::asset::{CharacterAsset, CharacterVisual, Route},
        event::TileEdgeEvent,
        input::InputSystems,
        lozo::{InLozo, InitLozo, Lozo, LozoAsset, LozoCommands, SpawnOverworldObjects},
        tile::{Grid, GridSize, Neighbor, Passability, TILE_SIZE, Tile, TileEdge},
    },
};
use bevy::prelude::*;
use bevy_elf::{AppExt, FromDef};
use bevy_spawn_phase_events::{InSpawnPhase, LozoAppExt};
use serde::{Deserialize, Serialize};
use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};
use thiserror::Error;

pub mod asset;

pub const CHARACTER_SPRITE_SCALE: Vec3 = Vec3::new(2.0, 2.0, 1.0);
pub const PLAYER_SPEED: u32 = 2;
const TURNING_DELAY_MILLIS: u64 = 64;
const BOBBING_OFFSET: f32 = 2.0;

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.init_ron_asset::<CharacterAsset>()
            .register_spawn_event::<CharactersSpawned>()
            .add_systems(
                PreUpdate,
                (
                    apply_behaviours.in_set(InputSystems),
                    (update_character_state, update_turning_delay)
                        .chain()
                        .after(InputSystems),
                ),
            )
            .add_systems(FixedUpdate, move_character)
            .add_systems(PostUpdate, update_visuals.before(AnimationUpdate))
            .add_observer(register_bobbing_observer)
            .add_observer(spawn_characters)
            .add_observer(start_tile_transition)
            .add_observer(update_behaviours);
    }
}

#[derive(Event)]
struct CharactersSpawned(Entity);

impl InSpawnPhase for CharactersSpawned {
    type SpawnPhase = SpawnOverworldObjects;

    fn entity(&self) -> Entity {
        self.0
    }
}

fn spawn_characters(
    event: On<InitLozo>,
    lozo_query: Query<&Lozo>,
    lozo_assets: Res<Assets<LozoAsset>>,
    character_assets: Res<Assets<CharacterAsset>>,
    mut commands: LozoCommands,
) -> Result {
    let lozo_asset = lozo_assets.require_handle(lozo_query.get(event.entity())?.handle())?;

    for character in &lozo_asset.characters {
        let character_asset = character_assets.require_handle(character.handle())?;

        let mut character_commands = commands.spawn_into_lozo(
            event.entity(),
            (
                Character(character.handle().clone()),
                Transform::from_translation(character_asset.position),
                character_asset.orientation,
                CharacterController::default(),
                children![(
                    Sprite {
                        image: character_asset.spritesheet.image.handle().clone(),
                        texture_atlas: Some(TextureAtlas {
                            index: 0,
                            layout: character_asset.spritesheet.layout.handle().clone(),
                        }),
                        ..Default::default()
                    },
                    Bobbing::default(),
                    Transform::from_translation(Vec3 {
                        x: 0.0,
                        y: 4.0,
                        z: 0.0,
                    })
                    .with_scale(CHARACTER_SPRITE_SCALE),
                )],
            ),
        )?;

        if let Some(ref behaviour) = character_asset.behaviour {
            character_commands.insert(CharacterBehaviour::from(behaviour.clone()));
        }
    }

    commands.trigger(CharactersSpawned(event.entity()));
    Ok(())
}

#[derive(Component)]
#[require(Orientation, CharacterState, Visibility)]
pub struct Character(Handle<CharacterAsset>);

impl Character {
    pub fn new(handle: Handle<CharacterAsset>) -> Self {
        Self(handle)
    }
}

impl Deref for Character {
    type Target = Handle<CharacterAsset>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Character {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Component)]
pub struct Player;

#[derive(
    FromDef, Serialize, Deserialize, Component, Default, PartialEq, Eq, Clone, Copy, Debug, Hash,
)]
#[elf(def_type(Self))]
pub enum Orientation {
    Up,
    Left,
    Right,
    #[default]
    Down,
}

impl Orientation {
    fn as_vec2(&self) -> Vec2 {
        match self {
            Self::Up => Vec2::Y,
            Self::Left => -Vec2::X,
            Self::Right => Vec2::X,
            Self::Down => -Vec2::Y,
        }
    }

    pub fn grid_direction(&self) -> IVec2 {
        match self {
            Self::Up => -IVec2::Y,
            Self::Left => -IVec2::X,
            Self::Right => IVec2::X,
            Self::Down => IVec2::Y,
        }
    }

    fn as_neighbor(&self) -> Neighbor {
        match self {
            Self::Up => Neighbor::Top,
            Self::Left => Neighbor::Left,
            Self::Right => Neighbor::Right,
            Self::Down => Neighbor::Bottom,
        }
    }

    pub fn rotate(self) -> Self {
        match self {
            Self::Up => Self::Right,
            Self::Right => Self::Down,
            Self::Down => Self::Left,
            Self::Left => Self::Up,
        }
    }
}

#[derive(Component, Default, PartialEq, Eq, Clone, Copy, Debug)]
pub enum CharacterState {
    #[default]
    Standing,
    Walking,
}

impl CharacterState {
    fn is_moving(&self) -> bool {
        matches!(self, Self::Walking)
    }
}

#[derive(Component)]
enum CharacterBehaviour {
    Walking { route: Route, next: usize },
}

impl From<asset::CharacterBehaviour> for CharacterBehaviour {
    fn from(value: asset::CharacterBehaviour) -> Self {
        match value {
            asset::CharacterBehaviour::Walking(route) => Self::Walking { route, next: 0 },
        }
    }
}

fn apply_behaviours(
    characters: Query<(
        &CharacterBehaviour,
        &Transform,
        &mut CharacterController,
        &InLozo,
    )>,
    grid_size: Query<&GridSize>,
) -> Result {
    for (behaviour, transform, mut controller, in_lozo) in characters {
        match &*behaviour {
            CharacterBehaviour::Walking { route, next } => {
                let grid_size = grid_size.get(in_lozo.entity())?;
                let current_pos = grid_size
                    .world_to_grid(transform.translation.truncate())
                    .ok_or_else(|| {
                        format!(
                            "character is at invalid grid position: {}",
                            transform.translation
                        )
                    })?;
                let next_pos = route.targets[*next];

                let direction = if next_pos.x != current_pos.x && next_pos.y != current_pos.y {
                    let previous_pos = route.targets[if *next == 0 {
                        route.targets.len() - 1
                    } else {
                        *next - 1
                    }];

                    let original_diff = next_pos.as_ivec2() - previous_pos.as_ivec2();
                    if original_diff.y > original_diff.x {
                        IVec2::new(0, next_pos.y as i32 - current_pos.y as i32)
                    } else {
                        IVec2::new(next_pos.x as i32 - current_pos.x as i32, 0)
                    }
                } else {
                    next_pos.as_ivec2() - current_pos.as_ivec2()
                };

                controller.set_direction(direction);
            }
        }
    }

    Ok(())
}

fn update_behaviours(
    event: On<CharReachedTile>,
    mut characters: Query<(
        Entity,
        &mut CharacterBehaviour,
        &Transform,
        &mut CharacterController,
        &InLozo,
    )>,
    grid_size: Query<&GridSize>,
    mut commands: Commands,
) -> Result {
    let Ok((entity, mut behaviour, transform, mut controller, in_lozo)) =
        characters.get_mut(event.character)
    else {
        return Ok(());
    };

    match &mut *behaviour {
        CharacterBehaviour::Walking { route, next } => {
            let world_pos = transform.translation.truncate();
            let current_pos = grid_size
                .get(in_lozo.entity())?
                .world_to_grid(world_pos)
                .ok_or_else(|| CharacterPositionOutOfGridBounds(world_pos))?;
            let next_pos = route.targets[*next];

            if *current_pos == next_pos {
                if route.cycle {
                    if *next == route.targets.len() - 1 {
                        if route.cycle {
                            *next = 0;
                        } else {
                            controller.reset();
                            commands.entity(entity).remove::<CharacterBehaviour>();
                        }
                    } else {
                        *next += 1;
                    }
                } else {
                    controller.reset();
                    commands.entity(entity).remove::<CharacterBehaviour>();
                }
            }
        }
    }

    Ok(())
}

#[derive(Component)]
struct TurningDelay {
    timer: Timer,
    just_inserted: bool,
}

impl Default for TurningDelay {
    fn default() -> Self {
        Self {
            timer: Timer::new(Duration::from_millis(TURNING_DELAY_MILLIS), TimerMode::Once),
            just_inserted: true,
        }
    }
}

#[derive(Component)]
struct TileTransition {
    from: UVec2,
    to: UVec2,
    state: TileTransitionState,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum TileTransitionState {
    LeavingTile,
    EnteringTile,
}

#[derive(EntityEvent)]
struct StartTileTransition(Entity);

#[derive(Resource)]
pub struct LoadingCharacter(pub Handle<CharacterAsset>);

impl Deref for LoadingCharacter {
    type Target = Handle<CharacterAsset>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Component, Default)]
pub struct Bobbing(bool);

impl Bobbing {
    fn up(&self) -> bool {
        self.0
    }
}

#[derive(Component, Default)]
pub struct CharacterController {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

impl CharacterController {
    fn state(&self) -> CharacterState {
        if self.up || self.down || self.left || self.right {
            CharacterState::Walking
        } else {
            CharacterState::Standing
        }
    }

    fn orientation(&self) -> Option<Orientation> {
        if self.up {
            Some(Orientation::Up)
        } else if self.left {
            Some(Orientation::Left)
        } else if self.right {
            Some(Orientation::Right)
        } else if self.down {
            Some(Orientation::Down)
        } else {
            None
        }
    }

    fn set_direction(&mut self, direction: IVec2) {
        if direction.x < 0 {
            self.left = true;
            self.right = false;
        } else if direction.x == 0 {
            self.left = false;
            self.right = false;
        } else if direction.x > 0 {
            self.left = false;
            self.right = true;
        }

        if direction.y < 0 {
            self.down = false;
            self.up = true;
        } else if direction.y == 0 {
            self.down = false;
            self.up = false;
        } else if direction.y > 0 {
            self.down = true;
            self.up = false;
        }
    }

    fn reset(&mut self) {
        *self = Self::default()
    }
}

#[allow(clippy::type_complexity)]
fn update_character_state(
    mut query: Query<
        (
            Entity,
            &mut Orientation,
            &mut CharacterState,
            &CharacterController,
            Option<&TurningDelay>,
        ),
        Without<TileTransition>,
    >,
    mut commands: Commands,
) -> Result {
    for (entity, mut orientation, mut state, controller, delay) in &mut query {
        let mut orientation_changed = false;
        if let Some(new_orientation) = controller.orientation()
            && new_orientation != *orientation
        {
            *orientation = new_orientation;
            if !state.is_moving() {
                commands.entity(entity).insert(TurningDelay::default());
            }
            orientation_changed = true;
        }

        let new_state = controller.state();
        if new_state != *state {
            *state = new_state;
        }

        if new_state.is_moving() {
            if delay.is_none() && !orientation_changed {
                commands.trigger(StartTileTransition(entity));
            }
        } else if delay.is_some() {
            commands.entity(entity).remove::<TurningDelay>();
        }
    }
    Ok(())
}

fn update_turning_delay(
    mut query: Query<(Entity, &mut TurningDelay)>,
    time: Res<Time>,
    mut commands: Commands,
) -> Result {
    for (entity, mut delay) in &mut query {
        if delay.just_inserted {
            delay.just_inserted = false;
            continue;
        }
        if delay.timer.tick(time.delta()).is_finished() {
            commands.entity(entity).remove::<TurningDelay>();
            commands.trigger(StartTileTransition(entity));
        }
    }

    Ok(())
}

#[derive(Event)]
pub struct CharLeftTile {
    character: Entity,
    edge: TileEdge,
}

impl TileEdgeEvent for CharLeftTile {
    fn trigger_entity(&self) -> Entity {
        self.character
    }

    fn edge(&self) -> &TileEdge {
        &self.edge
    }
}

#[derive(Event)]
pub struct CharEnteredTile {
    character: Entity,
    edge: TileEdge,
}

impl TileEdgeEvent for CharEnteredTile {
    fn trigger_entity(&self) -> Entity {
        self.character
    }

    fn edge(&self) -> &TileEdge {
        &self.edge
    }
}

#[derive(Event)]
pub struct CharReachedTile {
    character: Entity,
    edge: TileEdge,
}

impl TileEdgeEvent for CharReachedTile {
    fn trigger_entity(&self) -> Entity {
        self.character
    }

    fn edge(&self) -> &TileEdge {
        &self.edge
    }
}

#[allow(clippy::type_complexity)]
fn start_tile_transition(
    event: On<StartTileTransition>,
    mut character: Query<(Entity, &Transform, &Orientation, &InLozo), With<Character>>,
    lozo_query: Query<(&GridSize, &Grid<Option<Entity>>)>,
    mut tiles: Query<&mut Tile>,
    mut commands: Commands,
) -> Result {
    let (entity, transform, orientation, in_lozo) = character.get_mut(event.0)?;
    let (grid_size, grid) = lozo_query.get(in_lozo.entity())?;
    let world_pos = transform.translation.truncate();
    let origin = grid_size
        .world_to_grid(world_pos)
        .ok_or_else(|| CharacterPositionOutOfGridBounds(world_pos))?;

    let Some(target) = origin.neighbor(&orientation.as_neighbor()) else {
        return Ok(());
    };
    let Some(ref target_tile) = grid[target] else {
        return Ok(());
    };
    let mut target_tile = tiles.get_mut(*target_tile)?;
    if !matches!(target_tile.passability, Passability::Always) || target_tile.blocked {
        return Ok(());
    }

    target_tile.blocked = true;
    if let Some(origin_tile) = grid[origin] {
        tiles.get_mut(origin_tile)?.blocked = false;
    }

    commands.entity(entity).insert(TileTransition {
        from: *origin,
        to: *target,
        state: TileTransitionState::LeavingTile,
    });

    commands.trigger(CharLeftTile {
        character: entity,
        edge: TileEdge {
            from: *origin,
            to: *target,
        },
    });

    Ok(())
}

fn move_character(
    mut character: Query<
        (
            Entity,
            &Orientation,
            &mut Transform,
            &mut TileTransition,
            &InLozo,
        ),
        With<Character>,
    >,
    lozo_query: Query<&GridSize>,
    mut commands: Commands,
) -> Result {
    for (entity, orientation, mut transform, mut tt, in_lozo) in &mut character {
        let mut new_translation =
            transform.translation + (orientation.as_vec2() * PLAYER_SPEED as f32).extend(0.0);

        let grid_size = lozo_query.get(in_lozo.entity())?;
        let distance_to_from =
            (new_translation.truncate() - grid_size.grid_to_world(tt.from.as_vec2())).length();

        let edge = TileEdge {
            from: tt.from,
            to: tt.to,
        };
        match &tt.state {
            TileTransitionState::LeavingTile => {
                if distance_to_from >= TILE_SIZE as f32 / 2.0 {
                    tt.state = TileTransitionState::EnteringTile;
                    commands.trigger(CharEnteredTile {
                        character: entity,
                        edge,
                    });
                }
            }
            TileTransitionState::EnteringTile => {
                if distance_to_from >= TILE_SIZE as f32 {
                    commands.entity(entity).remove::<TileTransition>();
                    new_translation = grid_size
                        .grid_to_world(tt.to.as_vec2())
                        .extend(new_translation.z);

                    commands.trigger(CharReachedTile {
                        character: entity,
                        edge,
                    });
                }
            }
        }

        transform.translation = new_translation;
    }

    Ok(())
}

fn update_visuals(
    mut character: Query<(Ref<Orientation>, Ref<CharacterState>, &Character, &Children)>,
    mut sprites: Query<(Entity, &mut Sprite, Option<&mut Animated>)>,
    character_assets: Res<Assets<CharacterAsset>>,
    mut commands: Commands,
) -> Result {
    for (orientation, state, character, children) in &mut character {
        if !orientation.is_changed() && !state.is_changed() {
            continue;
        }

        for child in children {
            let (entity, mut sprite, animated) = sprites.get_mut(*child)?;
            let Some(ref mut atlas) = sprite.texture_atlas else {
                warn!("character sprite had no texture_atlas");
                continue;
            };
            let asset = character_assets.require_handle(character)?;
            let key = asset::CharacterState::from((*state, *orientation));
            let visual = &asset.animations[&key];

            match visual {
                CharacterVisual::Static(idx) => {
                    if animated.is_some() {
                        commands.entity(entity).remove::<Animated>();
                    }
                    atlas.index = *idx
                }
                CharacterVisual::Animated(animation) => {
                    if let Some(mut animated) = animated {
                        *animated = Animated::by(animation.handle().clone());
                    } else {
                        commands
                            .entity(entity)
                            .insert(Animated::by(animation.handle().clone()));
                    }
                }
            }

            sprite.flip_x = *orientation == Orientation::Right;
        }
    }
    Ok(())
}

fn register_bobbing_observer(
    event: On<Insert, Sprite>,
    sprites: Query<&ChildOf, (With<Bobbing>, With<Sprite>)>,
    characters: Query<(), With<Character>>,
    mut commands: Commands,
) {
    let Ok(child_of) = sprites.get(event.entity) else {
        return;
    };

    if characters.contains(child_of.parent()) {
        commands.entity(event.entity).observe(bobbing);
    }
}

fn bobbing(event: On<AnimationAdvanced>, mut sprites: Query<(&mut Transform, &mut Bobbing)>) {
    let Ok((mut transform, mut bobbing)) = sprites.get_mut(event.event_target()) else {
        return;
    };

    if bobbing.up() {
        transform.translation.y += BOBBING_OFFSET;
    } else {
        transform.translation.y -= BOBBING_OFFSET;
    }

    bobbing.0 = !bobbing.0;
}

#[derive(Error, Debug)]
#[error("character at invalid grid position: {0}")]
struct CharacterPositionOutOfGridBounds(Vec2);
