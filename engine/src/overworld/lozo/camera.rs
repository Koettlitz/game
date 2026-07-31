use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    image::ImageSampler,
    prelude::*,
    render::{
        extract_component::ExtractComponent,
        render_resource::{ShaderType, TextureFormat},
    },
};
use std::collections::HashSet;

use crate::{
    overworld::{
        character::PLAYER_SPEED,
        lozo::{InLozo, LozoSpawned},
        tile::TILE_SIZE,
    },
    shader::{ShaderDescriptor, ShaderPlugin},
};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ShaderPlugin::<ZoomWarpShader>::default())
            .add_observer(init_zoom_warp)
            .add_observer(attach_camera)
            .add_systems(Update, (update_camera_position, advance_zoom_warp));
    }
}

#[derive(Component)]
#[relationship_target(relationship = CameraOf, linked_spawn)]
pub struct LozoCamera(Entity);

impl LozoCamera {
    pub fn entity(&self) -> Entity {
        self.0
    }
}

#[derive(Component)]
#[relationship(relationship_target = LozoCamera)]
pub struct CameraOf(Entity);

impl CameraOf {
    pub fn entity(&self) -> Entity {
        self.0
    }
}

#[derive(Component)]
#[require(Transform)]
pub struct CameraTarget;

fn update_camera_position(
    cameras: Query<(&mut Transform, &CameraOf), Without<InLozo>>,
    target: Query<(&Transform, &InLozo), With<CameraTarget>>,
) {
    for (mut cam_transform, CameraOf(lozo_entity)) in cameras {
        if let Some((target_transform, _)) = target
            .iter()
            .find(|(_, in_lozo)| in_lozo.entity() == *lozo_entity)
        {
            cam_transform.translation.x = target_transform.translation.x;
            cam_transform.translation.y = target_transform.translation.y;
        }
    }
}

#[derive(Event)]
pub struct ZoomWarp {
    pub entity: Entity,
    pub reverse: bool,
}

impl ZoomWarp {
    pub fn new(lozo_entity: Entity) -> Self {
        Self {
            entity: lozo_entity,
            reverse: false,
        }
    }

    pub fn reverse(lozo_entity: Entity) -> Self {
        Self {
            entity: lozo_entity,
            reverse: true,
        }
    }
}

fn init_zoom_warp(event: On<ZoomWarp>, lozo: Query<&LozoCamera>, mut commands: Commands) -> Result {
    let LozoCamera(camera_entity) = lozo.get(event.entity)?;
    let mut zoom_warp_commands = commands.entity(*camera_entity);
    zoom_warp_commands.insert(ZoomWarpUniform {
        progress: if event.reverse { 1.0 } else { 0.0 },
        ..Default::default()
    });
    if event.reverse {
        zoom_warp_commands.insert(Reverse);
    }

    Ok(())
}

#[derive(Event)]
pub struct LozoCamAttached {
    pub lozo_entity: Entity,
    pub camera_entity: Entity,
}

fn attach_camera(
    event: On<LozoSpawned>,
    lozo_query: Query<&LozoCamera>,
    camera_query: Query<&RenderLayers>,
    mut images: ResMut<Assets<Image>>,
    window: Query<&Window>,
    mut commands: Commands,
) {
    let camera_entity = if let Ok(LozoCamera(camera_entity)) = lozo_query.get(event.entity()) {
        *camera_entity
    } else {
        let window = window.single().expect("single window");

        let mut image = Image::new_target_texture(
            ensure_pixel_perfect_size(window.width()),
            ensure_pixel_perfect_size(window.height()),
            TextureFormat::Rgba8Unorm,
            Some(TextureFormat::Rgba8UnormSrgb),
        );
        image.sampler = ImageSampler::nearest();
        let image_handle = images.add(image);
        let render_layer =
            find_free_lozo_render_layer(camera_query.iter().flat_map(RenderLayers::iter));

        commands
            .spawn((
                Camera2d,
                Camera {
                    order: -1,
                    ..default()
                },
                RenderTarget::Image(image_handle.clone().into()),
                RenderLayers::layer(render_layer),
                CameraOf(event.entity()),
            ))
            .id()
    };

    commands.trigger(LozoCamAttached {
        lozo_entity: event.entity(),
        camera_entity,
    });
}

fn find_free_lozo_render_layer(layers: impl Iterator<Item = usize>) -> usize {
    let taken: HashSet<usize> = layers.collect();
    (1..).find(|l| !taken.contains(l)).unwrap()
}

pub fn ensure_pixel_perfect_size(size: f32) -> u32 {
    let size = size.round().max(2.0) as u32;
    size - size % 2
}

#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
struct ZoomWarpUniform {
    pub progress: f32,
}

struct ZoomWarpShader;

impl Default for ZoomWarpUniform {
    fn default() -> Self {
        Self { progress: 0.0 }
    }
}

impl ShaderDescriptor for ZoomWarpShader {
    type Uniform = ZoomWarpUniform;

    fn name() -> &'static str {
        "zoom_warp"
    }

    fn shader_path() -> &'static str {
        "shaders/zoom_warp.wgsl"
    }
}

#[derive(Component)]
pub struct Reverse;

fn advance_zoom_warp(
    zoom_warp: Query<(Entity, &mut ZoomWarpUniform, Option<&Reverse>)>,
    mut commands: Commands,
) {
    for (entity, mut settings, reverse) in zoom_warp {
        let progress = if reverse.is_some() {
            1.0 - settings.progress
        } else {
            settings.progress
        };
        if progress >= 1.0 {
            let mut zoom_warp_commands = commands.entity(entity);
            zoom_warp_commands.remove::<ZoomWarpUniform>();
            if reverse.is_some() {
                zoom_warp_commands.remove::<Reverse>();
            }
        } else {
            let mut advance = PLAYER_SPEED as f32 / TILE_SIZE as f32;
            if reverse.is_some() {
                advance = -advance;
            }
            settings.progress = (settings.progress + advance).clamp(0.0, 1.0);
        }
    }
}
