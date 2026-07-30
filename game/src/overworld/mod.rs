use bevy::{
    camera::RenderTarget, prelude::*, render::render_resource::Extent3d, window::WindowResized,
};

use bevy_elf::{AssetResolver, HasResolver};
use engine::{
    asset::AssetsExt,
    overworld::{
        CHARACTER_LAYER,
        character::{Character, CharacterAsset, LoadingCharacter},
        lozo::{Lozo, LozoCommands, LozoSpawned, ensure_pixel_perfect_size},
        tile::{GridSize, TileGridSpawned},
    },
};
use input::InputPlugin;

mod input;

pub struct OverworldPlugin;
impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins((engine::overworld::OverworldPlugin, InputPlugin))
            .add_systems(Startup, (spawn_lozo, load_character_asset))
            .add_systems(
                Update,
                resize_render_target.run_if(resource_exists::<LozoRenderTarget>),
            )
            .add_observer(setup_lozo_render_target)
            .add_observer(on_tile_grid_spawned);
    }
}

fn spawn_lozo(mut commands: LozoCommands) {
    commands.spawn_lozo("world".to_owned());
}

fn load_character_asset(mut commands: Commands, asset_server: ResMut<AssetServer>) -> Result {
    let handle = asset_server.load(CharacterAsset::resolver().resolve("brendan")?);
    commands.insert_resource(LoadingCharacter(handle));
    Ok(())
}

fn on_tile_grid_spawned(
    event: On<TileGridSpawned>,
    grid_size: Query<&GridSize>,
    character: Option<Single<&mut Transform, With<Character>>>,
) -> Result {
    if let Some(mut character) = character {
        character.translation = grid_size
            .get(event.entity())?
            .snap_to_tile(Vec2::new(0.0, 0.0))
            .extend(CHARACTER_LAYER);
    }

    Ok(())
}

fn setup_lozo_render_target(
    event: On<LozoSpawned>,
    mut commands: Commands,
    lozo: Query<&RenderTarget, With<Lozo>>,
    mut initialized: Local<bool>,
) -> Result {
    if *initialized {
        return Ok(());
    }
    *initialized = true;

    let image_handle = lozo
        .get(event.entity())?
        .as_image()
        .ok_or("expected lozo to render into image")?;
    commands.spawn((Sprite {
        image: image_handle.clone(),
        ..default()
    },));

    commands.spawn(Camera2d);

    Ok(())
}

#[derive(Resource)]
struct LozoRenderTarget(Handle<Image>);

fn resize_render_target(
    mut resize_events: MessageReader<WindowResized>,
    render_target: Res<LozoRenderTarget>,
    mut images: ResMut<Assets<Image>>,
) -> Result {
    let Some(event) = resize_events.read().last() else {
        return Ok(());
    };
    let mut image = images.require_handle_mut(&render_target.0)?;
    image.resize(Extent3d {
        width: ensure_pixel_perfect_size(event.width),
        height: ensure_pixel_perfect_size(event.height),
        depth_or_array_layers: 1,
    });

    Ok(())
}
