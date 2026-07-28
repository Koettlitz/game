//! Renders the game world into an offscreen texture sized to the window's
//! *logical* pixels (1 game-pixel = 1 logical pixel), then displays that
//! texture on a sprite scaled to fill the window. The only place the OS's
//! fractional scale factor (e.g. 1.3125) ever applies is the single, final
//! GPU upscale of that sprite -- it never touches individual sprites.
//! Without this there are visual lines between tile sprites showing because
//! of rounding stuff while rendering the sprites.

use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureFormat},
    window::WindowResized,
};

use crate::ui::camera::CameraMovement;

/// All entities related to the blit (the "display this texture" pass)
/// live on this layer, separate from your game world's default layer (0),
/// so you don't have to touch any of your existing game-world spawns.
const BLIT_LAYER: usize = 1;

/// Renders the game world into an offscreen texture first, which is then rendered to the window
/// to make the sprites look clean without any visual artifacts.
pub struct PixelPerfectRenderPlugin;

impl Plugin for PixelPerfectRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_render_target)
            .add_systems(Update, resize_render_target);
    }
}

#[derive(Component)]
pub struct WorldCamera;

#[derive(Component)]
pub struct FinalCamera;

#[derive(Resource)]
struct GameTexture {
    image: Handle<Image>,
}

#[derive(Component)]
struct BlitSprite;

fn setup_render_target(
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

    // Game-world camera: renders your existing scene (default layer 0,
    // untouched) into the texture instead of straight to the window.
    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            ..default()
        },
        RenderTarget::Image(image_handle.clone().into()),
        CameraMovement::default(),
        WorldCamera,
    ));

    // The sprite that displays the rendered texture, scaled to fill the window.
    commands.spawn((
        Sprite {
            image: image_handle.clone(),
            ..default()
        },
        RenderLayers::layer(BLIT_LAYER),
        BlitSprite,
    ));

    // The camera that actually renders to the window. It only sees the
    // blit sprite (layer 1), never the game world (layer 0) directly.
    commands.spawn((Camera2d, RenderLayers::layer(BLIT_LAYER), FinalCamera));

    commands.insert_resource(GameTexture {
        image: image_handle,
    });
}

fn resize_render_target(
    mut resize_events: MessageReader<WindowResized>,
    game_texture: Res<GameTexture>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(event) = resize_events.read().last() else {
        return;
    };

    if let Some(mut image) = images.get_mut(&game_texture.image) {
        image.resize(Extent3d {
            width: ensure_pixel_perfect_size(event.width),
            height: ensure_pixel_perfect_size(event.height),
            depth_or_array_layers: 1,
        });
    }
}

fn ensure_pixel_perfect_size(size: f32) -> u32 {
    let size = size.round().max(2.0) as u32;
    size - size % 2
}
