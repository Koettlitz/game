use std::ops::Deref;

use crate::{
    animation::Animated,
    asset::{
        AssetResolver, AssetsExt, HasResolver,
        overworld::{
            CHARACTER_LAYER,
            lozo::LozoAsset,
            object::{GameObjectSpriteAsset, TextureAtlasData},
            tile::TileVisualKind,
        },
        spritesheet::SpriteKind,
    },
    overworld::{ObjectSpriteLookup, character::Character, tile::create_grid_bundle},
};
use bevy::{asset::RecursiveDependencyLoadState, ecs::system::SystemParam, log, prelude::*};

use crate::overworld::tile::Tile;

pub struct LozoPlugin;
impl Plugin for LozoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NextLozo>()
            .init_state::<LozoState>()
            .add_systems(
                PostUpdate,
                (
                    detect_lozo_transition
                        .run_if(resource_changed::<NextLozo>)
                        .run_if(in_state(LozoState::Default)),
                    (abort_transition, change_transition_target)
                        .before(detect_lozo_loaded)
                        .run_if(
                            resource_changed::<NextLozo>.and(
                                in_state(LozoState::LoadingLozoAsset)
                                    .or(in_state(LozoState::NextReady)),
                            ),
                        ),
                    detect_lozo_loaded.run_if(in_state(LozoState::LoadingLozoAsset)),
                    activate_switch.after(detect_lozo_loaded).run_if(
                        in_state(LozoState::LoadingLozoAsset).or(in_state(LozoState::NextReady)),
                    ),
                ),
            )
            .add_systems(
                OnEnter(LozoState::Switching),
                (despawn_lozo_entities, spawn_next_lozo, spawn_lozo_entities).chain(),
            );
    }
}

#[derive(Component, Default)]
#[require(Visibility, Transform, ObjectSpriteLookup)]
pub struct CurrentLozo(String);

impl Deref for CurrentLozo {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(SystemParam)]
pub struct LozoCommands<'w, 's> {
    commands: Commands<'w, 's>,
    query: Single<'w, 's, Entity, With<CurrentLozo>>,
}

impl<'w, 's> LozoCommands<'w, 's> {
    pub fn spawn(&mut self, bundle: impl Bundle) -> EntityCommands<'_> {
        let entity = self.commands.spawn(bundle).id();
        self.commands.entity(*self.query).add_child(entity);
        self.commands.entity(entity)
    }
}

#[derive(Resource, Default)]
pub struct NextLozo {
    id: Option<String>,
    ready: Option<ReadyNextLozo>,
    pub auto_activate: bool,
}

impl NextLozo {
    pub fn set(&mut self, target: String) {
        if let Some(id) = self.id.as_ref() {
            if *id == target {
                return;
            }
        }
        self.reset();
        self.id = Some(target);
    }

    pub fn ready(&mut self) -> Option<&mut ReadyNextLozo> {
        self.ready.as_mut()
    }

    pub fn reset(&mut self) {
        self.id = None;
        self.ready = None;
        self.auto_activate = false;
    }
}

#[derive(Default)]
pub struct ReadyNextLozo {
    activate: bool,
}

impl ReadyNextLozo {
    pub fn activate(&mut self) {
        self.activate = true;
    }
}

#[derive(Resource)]
struct LozoTransition {
    next_lozo: String,
    asset_handle: Handle<LozoAsset>,
}

fn detect_lozo_transition(
    next_lozo: ResMut<NextLozo>,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<LozoState>>,
    mut commands: Commands,
) -> Result<()> {
    let Some(next_lozo) = next_lozo.id.as_ref() else {
        return Ok(());
    };

    log::info!("loading requested next lozo {next_lozo}");
    let asset_path = <LozoAsset as HasResolver>::resolver().resolve(next_lozo)?;
    commands.insert_resource(LozoTransition {
        next_lozo: next_lozo.to_string(),
        asset_handle: asset_server.load(asset_path),
    });
    next_state.set(LozoState::LoadingLozoAsset);
    Ok(())
}

fn change_transition_target(
    next_lozo: Res<NextLozo>,
    mut transition: ResMut<LozoTransition>,
    asset_server: Res<AssetServer>,
    current_state: Res<State<LozoState>>,
    mut next_state: ResMut<NextState<LozoState>>,
) -> Result<()> {
    let Some(id) = next_lozo.id.as_ref() else {
        return Ok(());
    };

    if id != &transition.next_lozo {
        log::debug!(
            "next lozo changed from {} to {id} - loading {id} now instead",
            transition.next_lozo
        );
        transition.next_lozo = id.to_string();
        let asset_path = <LozoAsset as HasResolver>::resolver().resolve(id)?;
        transition.asset_handle = asset_server.load(asset_path);
        if !matches!(current_state.get(), LozoState::LoadingLozoAsset) {
            next_state.set(LozoState::LoadingLozoAsset);
        }
    }

    Ok(())
}

fn abort_transition(
    next_lozo: Res<NextLozo>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<LozoState>>,
) {
    if next_lozo.id.is_none() {
        commands.remove_resource::<LozoTransition>();
        next_state.set(LozoState::Default);
    }
}

fn detect_lozo_loaded(
    asset_server: Res<AssetServer>,
    transition: Res<LozoTransition>,
    mut next_state: ResMut<NextState<LozoState>>,
    mut next_lozo: ResMut<NextLozo>,
    mut commands: Commands,
) {
    match asset_server.recursive_dependency_load_state(transition.asset_handle.id()) {
        RecursiveDependencyLoadState::Loaded => {
            next_state.set(LozoState::NextReady);
            next_lozo.ready = Some(ReadyNextLozo {
                activate: next_lozo.auto_activate,
            });
        }
        RecursiveDependencyLoadState::Failed(e) => {
            error!("failed to load lozo: \"{e}\"");
            commands.remove_resource::<LozoTransition>();
            next_lozo.reset();
            next_state.set(LozoState::Default);
        }
        _ => {}
    }
}

fn activate_switch(mut next_lozo: ResMut<NextLozo>, mut next_state: ResMut<NextState<LozoState>>) {
    if let Some(ready) = next_lozo.ready() {
        if ready.activate {
            next_state.set(LozoState::Switching);
            next_lozo.reset();
        }
    }
}

fn despawn_lozo_entities(mut commands: Commands, current: Single<Entity, With<CurrentLozo>>) {
    commands.entity(*current).despawn_children();
}

fn spawn_next_lozo(
    mut commands: Commands,
    transition: Option<Res<LozoTransition>>,
    mut current: Query<&mut CurrentLozo>,
) {
    let Some(transition) = transition else {
        return;
    };
    match current.single_mut() {
        Ok(mut current) => {
            current.0 = transition.next_lozo.clone();
        }
        Err(_) => {
            commands.spawn(CurrentLozo(transition.next_lozo.clone()));
        }
    }
}

fn spawn_lozo_entities(
    mut object_lookup: Single<&mut ObjectSpriteLookup, With<CurrentLozo>>,
    mut commands: LozoCommands,
    transition: Res<LozoTransition>,
    lozo_assets: Res<Assets<LozoAsset>>,
    object_assets: Res<Assets<GameObjectSpriteAsset>>,
    character: Option<Single<&mut Transform, With<Character>>>,
    mut next_state: ResMut<NextState<LozoState>>,
) -> Result<()> {
    let lozo_asset = lozo_assets.require_handle(&transition.asset_handle)?;

    let (grid, grid_size) = create_grid_bundle(lozo_asset.grid_size(), |pos| {
        let Some(tile_asset) = &lozo_asset.tile_grid[*pos.as_index()] else {
            return Ok(None);
        };
        let mut sprite_stack = Vec::new();
        for visual in tile_asset.sprite_stack.iter() {
            let spritesheet = &visual.spritesheet;
            let entity = spawn_tile_sprite(
                &visual.kind,
                spritesheet.clone(),
                Some(visual.layout.clone()),
                visual.z,
                &mut commands.commands,
            )?;
            sprite_stack.push(entity);
        }

        let tile_entity = commands
            .spawn((
                Tile::new(tile_asset.passability, tile_asset.events.clone()),
                Transform::from_translation(pos.to_world_pos().extend(0.0)),
            ))
            .id();
        commands
            .commands
            .entity(tile_entity)
            .add_children(&sprite_stack);
        Ok(Some(tile_entity))
    })?;

    for object in &lozo_asset.objects {
        let object_asset = object_assets.require_handle(object.handle())?;
        let transform = Transform::from_translation(object_asset.world_position);
        let entity = if let Some(TextureAtlasData { layout, kind }) = &object_asset.sprite_kind {
            match kind {
                SpriteKind::Static { idx } => commands.spawn((
                    Sprite::from_atlas_image(
                        object_asset.image.clone(),
                        TextureAtlas {
                            layout: layout.clone(),
                            index: *idx,
                        },
                    ),
                    transform,
                )),
                SpriteKind::Animated { animation } => commands.spawn((
                    Sprite::from_atlas_image(
                        object_asset.image.clone(),
                        TextureAtlas {
                            layout: layout.clone(),
                            ..Default::default()
                        },
                    ),
                    Animated::by(animation.clone()),
                    transform,
                )),
            }
        } else {
            commands.spawn((Sprite::from_image(object_asset.image.clone()), transform))
        }
        .id();
        object_lookup.insert(object.id().to_string(), entity);
    }

    if let Some(mut character) = character {
        character.translation = grid_size
            .snap_to_tile(Vec2::new(0.0, 0.0))
            .extend(CHARACTER_LAYER);
    }

    commands.spawn((grid, grid_size));

    commands.commands.remove_resource::<LozoTransition>();
    next_state.set(LozoState::Default);

    Ok(())
}

fn spawn_tile_sprite(
    visual: &TileVisualKind,
    image_handle: Handle<Image>,
    layout_handle: Option<Handle<TextureAtlasLayout>>,
    z: f32,
    commands: &mut Commands,
) -> Result<Entity> {
    let transform = Transform::from_translation(Vec3::new(0.0, 0.0, z));
    Ok(match &visual {
        TileVisualKind::Static { idx } => {
            let sprite = if let Some(layout_handle) = layout_handle {
                Sprite::from_atlas_image(
                    image_handle.clone(),
                    TextureAtlas {
                        layout: layout_handle,
                        index: *idx,
                    },
                )
            } else {
                Sprite::from_image(image_handle.clone())
            };
            commands.spawn((sprite, transform)).id()
        }
        TileVisualKind::Animated { animation } => {
            let sprite = if let Some(layout_handle) = layout_handle {
                Sprite::from_atlas_image(
                    image_handle.clone(),
                    TextureAtlas {
                        layout: layout_handle,
                        ..Default::default()
                    },
                )
            } else {
                Sprite::from_image(image_handle.clone())
            };
            commands
                .spawn((sprite, transform, Animated::by(animation.clone())))
                .id()
        }
    })
}

#[derive(States, Default, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum LozoState {
    #[default]
    Default,
    LoadingLozoAsset,
    NextReady,
    Switching,
}
