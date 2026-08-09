use crate::{
    animation::{Animated, AnimationAdvanced, AnimationUpdate},
    asset::AssetsExt,
    overworld::{
        CHARACTER_LAYER,
        input::InputSystems,
        lozo::{CameraTarget, InLozo, LozoCamera, LozoCommands, SurviveLozoTransition},
        tile::{
            CharEnteredTile, CharLeftTile, CharReachedTile, Grid, GridSize, Neighbor, Passability,
            TILE_SIZE, Tile, TileEdge, TileEdgeEvents,
        },
    },
};
use bevy::{camera::visibility::RenderLayers, prelude::*};
use bevy_elf::AppExt;
use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

pub use asset::*;

mod asset;

pub const PLAYER_SPEED: u32 = 2;
const TURNING_DELAY_MILLIS: u64 = 64;
const BOBBING_OFFSET: f32 = 0.8;

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.init_ron_asset::<CharacterAsset>()
            .add_systems(
                PreUpdate,
                (update_character_state, update_turning_delay)
                    .chain()
                    .after(InputSystems),
            )
            .add_systems(FixedUpdate, move_character)
            .add_systems(
                Update,
                spawn_character.run_if(resource_exists::<LoadingCharacter>),
            )
            .add_systems(PostUpdate, update_visuals.before(AnimationUpdate))
            .add_observer(start_tile_transition)
            .add_observer(bobbing);
    }
}

#[derive(Component)]
#[require(Orientation, CharacterState, Visibility)]
pub struct Character(Handle<CharacterAsset>);

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

#[derive(Component, Default, PartialEq, Eq, Clone, Copy, Debug)]
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

    fn as_neighbor(&self) -> Neighbor {
        match self {
            Self::Up => Neighbor::Top,
            Self::Left => Neighbor::Left,
            Self::Right => Neighbor::Right,
            Self::Down => Neighbor::Bottom,
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
struct Bobbing(bool);

impl Bobbing {
    fn up(&self) -> bool {
        self.0
    }
}

fn spawn_character(
    mut commands: LozoCommands,
    asset_server: Res<AssetServer>,
    loading_character: Res<LoadingCharacter>,
    character_assets: Res<Assets<CharacterAsset>>,

    // TODO: This singleton assumption works for now, but has to be changed at some point
    lozo_query: Single<(Entity, &GridSize, &LozoCamera)>,
    render_layers: Query<&RenderLayers>,
) -> Result<()> {
    if !asset_server.is_loaded_with_dependencies(loading_character.id()) {
        return Ok(());
    }
    let asset = character_assets.require_handle(&**loading_character)?;
    let (lozo_entity, grid_size, lozo_camera) = lozo_query.into_inner();
    let position = grid_size.snap_to_tile((0.0, 0.0));
    commands.spawn_into_lozo(
        lozo_entity,
        (
            Character(loading_character.clone()),
            Transform {
                translation: position.extend(CHARACTER_LAYER),
                scale: Vec3::new(2.0, 2.0, 1.0),
                ..Default::default()
            },
            CharacterController::default(),
            children![(
                Sprite {
                    image: asset.spritesheet.image.clone(),
                    texture_atlas: Some(TextureAtlas {
                        index: 0,
                        layout: asset.spritesheet.layout.clone(),
                    }),
                    ..Default::default()
                },
                Bobbing::default(),
                render_layers.get(lozo_camera.entity())?.clone(),
                Transform::from_translation(Vec3 {
                    x: 0.0,
                    y: 4.0,
                    z: 0.0
                })
            )],
            CameraTarget,
            SurviveLozoTransition,
        ),
    )?;

    commands.remove_resource::<LoadingCharacter>();
    Ok(())
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
) -> Result<()> {
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
) -> Result<()> {
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

#[allow(clippy::type_complexity)]
fn start_tile_transition(
    event: On<StartTileTransition>,
    mut character: Query<(Entity, &Transform, &Orientation, &InLozo), With<Character>>,
    lozo_query: Query<(
        &GridSize,
        &Grid<Option<Entity>>,
        &TileEdgeEvents<CharLeftTile>,
    )>,
    tiles: Query<&Tile>,
    mut commands: Commands,
) -> Result<()> {
    let (entity, transform, orientation, in_lozo) = character.get_mut(event.0)?;
    let (grid_size, grid, tile_edge_events) = lozo_query.get(in_lozo.entity())?;
    let origin = grid_size
        .world_to_grid(transform.translation.truncate())
        .ok_or("character at invalid grid position")?;

    let Some(target) = origin.neighbor(&orientation.as_neighbor()) else {
        return Ok(());
    };
    let Some(ref target_tile) = grid[target] else {
        return Ok(());
    };
    let target_tile = tiles.get(*target_tile)?;
    if !matches!(target_tile.passability, Passability::Always) {
        return Ok(());
    }

    commands.entity(entity).insert(TileTransition {
        from: *origin,
        to: *target,
        state: TileTransitionState::LeavingTile,
    });

    tile_edge_events.trigger(
        &TileEdge {
            from: *origin,
            to: *target,
        },
        in_lozo.entity(),
        &mut commands,
    );

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
    lozo_query: Query<(
        &GridSize,
        &TileEdgeEvents<CharEnteredTile>,
        &TileEdgeEvents<CharReachedTile>,
    )>,
    mut commands: Commands,
) -> Result {
    for (entity, orientation, mut transform, mut tt, in_lozo) in &mut character {
        let mut new_translation =
            transform.translation + (orientation.as_vec2() * PLAYER_SPEED as f32).extend(0.0);

        let (grid_size, entered_events, reached_events) = lozo_query.get(in_lozo.entity())?;
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
                    entered_events.trigger(&edge, in_lozo.entity(), &mut commands);
                }
            }
            TileTransitionState::EnteringTile => {
                if distance_to_from >= TILE_SIZE as f32 {
                    commands.entity(entity).remove::<TileTransition>();
                    new_translation = grid_size
                        .grid_to_world(tt.to.as_vec2())
                        .extend(new_translation.z);

                    reached_events.trigger(&edge, in_lozo.entity(), &mut commands);
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
) -> Result<()> {
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
                        *animated = Animated::by(animation.clone());
                    } else {
                        commands
                            .entity(entity)
                            .insert(Animated::by(animation.clone()));
                    }
                }
            }

            sprite.flip_x = *orientation == Orientation::Right;
        }
    }
    Ok(())
}

fn bobbing(event: On<AnimationAdvanced>, mut sprites: Query<(&mut Transform, &mut Bobbing)>) {
    let Ok((mut transform, mut bobbing)) = sprites.get_mut(event.entity()) else {
        return;
    };

    if bobbing.up() {
        transform.translation.y += BOBBING_OFFSET;
    } else {
        transform.translation.y -= BOBBING_OFFSET;
    }

    bobbing.0 = !bobbing.0;
}
