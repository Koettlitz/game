use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    render::render_resource::Extent3d,
    window::WindowResized,
};

use bevy_elf::{AssetResolver, HasResolver};
use engine::{
    asset::AssetsExt,
    overworld::{
        CHARACTER_LAYER,
        camera::{
            CameraAttached, RenderDimensions, ZoomWarp, attach_camera, ensure_pixel_perfect_size,
        },
        character::{Bobbing, Character, CharacterAsset, CharacterController, LoadingCharacter},
        lozo::{Lozo, LozoCommands},
        tile::GridSize,
    },
};
use input::InputPlugin;

mod input;

const CAMERA_Z: f32 = 100.0;
const LOZO_MARGIN: f32 = 4.0;

pub struct OverworldPlugin;
impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins((engine::overworld::OverworldPlugin, InputPlugin))
            .add_systems(
                Startup,
                (spawn_lozo, load_character_asset, setup_main_camera),
            )
            .add_systems(
                Update,
                (
                    resize_render_target,
                    spawn_character.run_if(resource_exists::<LoadingCharacter>),
                ),
            )
            .add_observer(setup_camera_sprite)
            .add_observer(camera_attached);
    }
}

fn setup_main_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn spawn_lozo(mut commands: Commands, asset_server: Res<AssetServer>) -> Result {
    commands.spawn(Lozo::from_id("world", &asset_server)?);
    Ok(())
}

fn load_character_asset(mut commands: Commands, asset_server: ResMut<AssetServer>) -> Result {
    let handle = asset_server.load(CharacterAsset::resolver().resolve("brendan")?);
    commands.insert_resource(LoadingCharacter(handle));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn spawn_character(
    mut commands: LozoCommands,
    asset_server: Res<AssetServer>,
    loading_character: Res<LoadingCharacter>,
    character_assets: Res<Assets<CharacterAsset>>,
    mut image_assets: ResMut<Assets<Image>>,
    lozo_query: Query<(Entity, &GridSize, &RenderLayers)>,
    window: Single<&Window>,
) -> Result {
    if !asset_server.is_loaded_with_dependencies(loading_character.id()) {
        return Ok(());
    }

    let Ok((lozo_entity, grid_size, render_layers)) = lozo_query.single() else {
        return Ok(());
    };

    let asset = character_assets.require_handle(&**loading_character)?;
    let position = grid_size.snap_to_tile((0.0, 0.0));

    let character_entity = commands
        .spawn_into_lozo(
            lozo_entity,
            (
                Character::new(loading_character.clone()),
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
                    Transform::from_translation(Vec3 {
                        x: 0.0,
                        y: 4.0,
                        z: 0.0
                    }),
                )],
            ),
        )?
        .id();

    let render_dimensions = RenderDimensions {
        width: window.width(),
        height: window.height(),
    };

    attach_camera(
        character_entity,
        render_dimensions,
        render_layers.clone(),
        &mut image_assets,
        &mut commands,
    );

    commands.remove_resource::<LoadingCharacter>();
    Ok(())
}

fn camera_attached(event: On<CameraAttached>, mut commands: Commands) {
    commands.trigger(ZoomWarp::reverse(event.entity()));
}

fn setup_camera_sprite(
    event: On<CameraAttached>,
    render_target: Query<&RenderTarget>,
    mut commands: Commands,
) -> Result {
    let image_handle = render_target
        .get(event.entity())?
        .as_image()
        .ok_or("expected lozo camera to render into image")?;

    commands.spawn((
        Sprite {
            image: image_handle.clone(),
            ..default()
        },
        CameraSprite,
        Transform::from_translation(Vec3::new(0.0, 0.0, CAMERA_Z)),
    ));

    Ok(())
}

#[derive(Component)]
struct CameraSprite;

fn resize_render_target(
    mut resize_events: MessageReader<WindowResized>,
    lozo_sprite: Query<&Sprite, With<CameraSprite>>,
    mut images: ResMut<Assets<Image>>,
) -> Result {
    let Some(event) = resize_events.read().last() else {
        return Ok(());
    };

    for sprite in lozo_sprite {
        let mut image = images.require_handle_mut(&sprite.image)?;
        image.resize(Extent3d {
            width: ensure_pixel_perfect_size(event.width / 2.0 - LOZO_MARGIN * 2.0),
            height: ensure_pixel_perfect_size(event.height),
            depth_or_array_layers: 1,
        });
    }

    Ok(())
}
