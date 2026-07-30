use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    ecs::system::SystemParam,
    image::ImageSampler,
    prelude::*,
    render::render_resource::TextureFormat,
};

use crate::overworld::lozo::Lozo;

#[derive(SystemParam)]
pub struct LozoCamBuilder<'w, 's> {
    lozo_query: Query<'w, 's, &'static RenderLayers, With<Lozo>>,
    images: ResMut<'w, Assets<Image>>,
    windows: Query<'w, 's, &'static Window>,
}

impl<'w, 's> LozoCamBuilder<'w, 's> {
    pub fn create_camera(&mut self) -> impl Bundle {
        let window = self.windows.single().expect("single window");

        let mut image = Image::new_target_texture(
            ensure_pixel_perfect_size(window.width()),
            ensure_pixel_perfect_size(window.height()),
            TextureFormat::Rgba8Unorm,
            Some(TextureFormat::Rgba8UnormSrgb),
        );
        image.sampler = ImageSampler::nearest();
        let image_handle = self.images.add(image);
        let render_layer =
            find_free_lozo_render_layer(self.lozo_query.iter().flat_map(RenderLayers::iter));

        (
            Camera2d,
            Camera {
                order: -1,
                ..default()
            },
            RenderTarget::Image(image_handle.clone().into()),
            RenderLayers::layer(render_layer),
        )
    }
}

fn find_free_lozo_render_layer(mut layers: impl Iterator<Item = usize>) -> usize {
    let mut render_layer = 1;
    loop {
        if layers.any(|l| l == render_layer) {
            render_layer += 1;
        } else {
            return render_layer;
        }
    }
}

pub fn ensure_pixel_perfect_size(size: f32) -> u32 {
    let size = size.round().max(2.0) as u32;
    size - size % 2
}
