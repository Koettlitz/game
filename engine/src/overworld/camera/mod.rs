use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    image::ImageSampler,
    prelude::*,
    render::{
        extract_component::ExtractComponent,
        render_resource::{ShaderType, TextureFormat},
    },
};

use crate::{
    overworld::{character::PLAYER_SPEED, tile::TILE_SIZE},
    shader::{ShaderDescriptor, ShaderPlugin},
};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ShaderPlugin::<ZoomWarpShader>::default())
            .add_observer(init_zoom_warp)
            .add_systems(Update, (update_camera_positions, advance_zoom_warp));
    }
}

#[derive(Component)]
#[relationship_target(relationship = CameraOf, linked_spawn)]
pub struct HasCamera(Entity);

impl HasCamera {
    pub fn entity(&self) -> Entity {
        self.0
    }
}

#[derive(Component)]
#[relationship(relationship_target = HasCamera)]
pub struct CameraOf(Entity);

impl CameraOf {
    pub fn entity(&self) -> Entity {
        self.0
    }
}

#[derive(Component)]
#[require(Transform)]
pub struct CameraTarget;

fn update_camera_positions(
    cameras: Query<(&mut Transform, &CameraOf)>,
    target: Query<&Transform, (With<HasCamera>, Without<CameraOf>)>,
) {
    for (mut cam_transform, CameraOf(camera_entity)) in cameras {
        if let Ok(target_transform) = target.get(*camera_entity) {
            cam_transform.translation.x = target_transform.translation.x;
            cam_transform.translation.y = target_transform.translation.y;
        }
    }
}

#[derive(Event)]
pub struct ZoomWarp {
    pub camera_entity: Entity,
    pub reverse: bool,
}

impl ZoomWarp {
    pub fn new(camera: Entity) -> Self {
        Self {
            camera_entity: camera,
            reverse: false,
        }
    }

    pub fn reverse(camera: Entity) -> Self {
        Self {
            camera_entity: camera,
            reverse: true,
        }
    }
}

fn init_zoom_warp(event: On<ZoomWarp>, mut commands: Commands) -> Result {
    let mut zoom_warp_commands = commands.entity(event.camera_entity);

    zoom_warp_commands.insert(ZoomWarpUniform {
        progress: if event.reverse { 1.0 } else { 0.0 },
    });

    if event.reverse {
        zoom_warp_commands.insert(Reverse);
    }

    Ok(())
}

pub struct RenderDimensions {
    pub width: f32,
    pub height: f32,
}

#[derive(Event)]
pub struct CameraAttached(Entity);

impl CameraAttached {
    pub fn entity(&self) -> Entity {
        self.0
    }
}

pub fn attach_camera(
    target_entity: Entity,
    render_dimensions: RenderDimensions,
    render_layers: RenderLayers,
    images: &mut Assets<Image>,
    commands: &mut Commands,
) {
    let mut image = Image::new_target_texture(
        ensure_pixel_perfect_size(render_dimensions.width),
        ensure_pixel_perfect_size(render_dimensions.height),
        TextureFormat::Rgba8Unorm,
        Some(TextureFormat::Rgba8UnormSrgb),
    );
    image.sampler = ImageSampler::nearest();
    let image_handle = images.add(image);

    let entity = commands
        .spawn((
            Camera2d,
            Camera {
                order: -1,
                ..default()
            },
            RenderTarget::Image(image_handle.clone().into()),
            render_layers,
            CameraOf(target_entity),
        ))
        .id();

    commands.trigger(CameraAttached(entity));
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
