use std::ops::Deref;

use bevy::{asset::RecursiveDependencyLoadState, ecs::system::SystemParam, prelude::*};
use engine::{
    animation::Animated,
    asset::{
        AssetResolver, AssetsExt, HasResolver,
        overworld::{
            lozo::LozoAsset,
            object::{GameObjectSpriteAsset, TextureAtlasData},
            tile::TileVisualKind,
        },
        spritesheet::SpriteKind,
    },
    overworld::tile::create_grid_bundle,
};

use crate::overworld::tile::Tile;

pub struct LozoPlugin;
impl Plugin for LozoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NextLozo>()
            .init_state::<LozoState>()
            .add_systems(
                FixedPostUpdate,
                detect_lozo_transition
                    .run_if(resource_changed::<NextLozo>)
                    .run_if(in_state(LozoState::Default)),
            )
            .add_systems(
                FixedPostUpdate,
                detect_lozo_loaded.run_if(in_state(LozoState::LoadingLozoAsset)),
            )
            .add_systems(
                OnEnter(LozoState::Switching),
                (despawn_lozo_entities, spawn_next_lozo, spawn_lozo_entities).chain(),
            );
    }
}

#[derive(Component, Default)]
#[require(Visibility, Transform)]
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
pub struct NextLozo(Option<String>);

impl NextLozo {
    pub fn set(&mut self, target: String) {
        self.0 = Some(target);
    }
}

#[derive(Resource)]
struct LozoTransition {
    next_lozo: String,
    asset_handle: Handle<LozoAsset>,
}

fn detect_lozo_transition(
    mut next_lozo: ResMut<NextLozo>,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<LozoState>>,
    mut commands: Commands,
) -> Result<()> {
    let Some(next_lozo) = next_lozo.0.take() else {
        return Ok(());
    };

    let asset_path = <LozoAsset as HasResolver>::resolver().resolve(&next_lozo)?;
    commands.insert_resource(LozoTransition {
        next_lozo: next_lozo,
        asset_handle: asset_server.load(asset_path),
    });
    next_state.set(LozoState::LoadingLozoAsset);
    Ok(())
}

fn detect_lozo_loaded(
    asset_server: Res<AssetServer>,
    transition: Res<LozoTransition>,
    mut next_state: ResMut<NextState<LozoState>>,

    mut commands: Commands,
) {
    match asset_server.recursive_dependency_load_state(transition.asset_handle.id()) {
        RecursiveDependencyLoadState::Loaded => next_state.set(LozoState::Switching),
        RecursiveDependencyLoadState::Failed(e) => {
            error!("failed to load lozo: \"{e}\"");
            commands.remove_resource::<LozoTransition>();
            next_state.set(LozoState::Default);
        }
        _ => {}
    }
}

fn despawn_lozo_entities(mut commands: Commands, current: Query<Entity, With<CurrentLozo>>) {
    if let Ok(current) = current.single() {
        commands.entity(current).despawn_children();
    }
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
    mut commands: LozoCommands,
    transition: Res<LozoTransition>,
    lozo_assets: Res<Assets<LozoAsset>>,
    object_assets: Res<Assets<GameObjectSpriteAsset>>,
    mut next_state: ResMut<NextState<LozoState>>,
) -> Result<()> {
    let lozo_asset = lozo_assets.require_handle(&transition.asset_handle)?;

    let grid_bundle = create_grid_bundle(lozo_asset.grid_size(), |pos| {
        let Some(tile_asset) = &lozo_asset.tile_grid[*pos.as_index()] else {
            return Ok(None);
        };
        let mut sprite_stack = Vec::new();
        for (i, visual) in tile_asset.sprite_stack.iter().enumerate() {
            let spritesheet = &visual.spritesheet;
            let entity = spawn_tile_sprite(
                pos.to_world_pos(),
                &visual.kind,
                spritesheet.clone(),
                Some(visual.layout.clone()),
                i as f32,
                &mut commands.commands,
            )?;
            sprite_stack.push(entity);
        }

        let entity = commands
            .spawn(Tile::new(tile_asset.passability, sprite_stack))
            .id();
        Ok(Some(entity))
    })?;

    for handle in &lozo_asset.objects {
        let object_asset = object_assets.require_handle(handle)?;
        println!("spawning object sprite at {}", object_asset.world_position);
        let transform = Transform::from_translation(object_asset.world_position);
        if let Some(TextureAtlasData { layout, kind }) = &object_asset.sprite_kind {
            match kind {
                SpriteKind::Static { idx } => {
                    commands.spawn((
                        Sprite::from_atlas_image(
                            object_asset.image.clone(),
                            TextureAtlas {
                                layout: layout.clone(),
                                index: *idx,
                            },
                        ),
                        transform,
                    ));
                }
                SpriteKind::Animated { animation } => {
                    commands.spawn((
                        Sprite::from_atlas_image(
                            object_asset.image.clone(),
                            TextureAtlas {
                                layout: layout.clone(),
                                ..Default::default()
                            },
                        ),
                        Animated::by(animation.clone()),
                        transform,
                    ));
                }
            }
        }
    }
    commands.spawn(grid_bundle);
    commands.commands.remove_resource::<LozoTransition>();
    next_state.set(LozoState::Default);
    Ok(())
}

fn spawn_tile_sprite(
    world_pos: Vec2,
    visual: &TileVisualKind,
    image_handle: Handle<Image>,
    layout_handle: Option<Handle<TextureAtlasLayout>>,
    z: f32,
    commands: &mut Commands,
) -> Result<Entity> {
    let transform = Transform::from_translation(world_pos.extend(z));
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
    Switching,
}
