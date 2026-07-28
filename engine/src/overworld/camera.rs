use bevy::{
    camera::RenderTarget, image::ImageSampler, prelude::*, render::render_resource::TextureFormat,
};

use crate::overworld::lozo::LozoSpawned;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_lozo_spawned);
    }
}

#[derive(EntityEvent)]
pub struct LozoCamSetup(#[event_target] Entity);
impl LozoCamSetup {
    pub fn entity(&self) -> Entity {
        self.0
    }
}

fn on_lozo_spawned(
    event: On<LozoSpawned>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let mut image = Image::new_target_texture(
        ensure_pixel_perfect_size(window.width()),
        ensure_pixel_perfect_size(window.height()),
        TextureFormat::Rgba8Unorm,
        Some(TextureFormat::Rgba8UnormSrgb),
    );
    image.sampler = ImageSampler::nearest();
    let image_handle = images.add(image);

    commands.entity(event.entity()).insert((
        Camera2d,
        Camera {
            order: -1,
            ..default()
        },
        RenderTarget::Image(image_handle.clone().into()),
    ));
    commands.trigger(LozoCamSetup(event.entity()));
}

pub fn ensure_pixel_perfect_size(size: f32) -> u32 {
    let size = size.round().max(2.0) as u32;
    size - size % 2
}
