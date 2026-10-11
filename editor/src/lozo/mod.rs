use std::iter;

use bevy::{
    asset::RecursiveDependencyLoadState, camera::visibility::RenderLayers, log, prelude::*,
};

use bevy_elf::{AssetRef, AssetResolver, HasResolver};
use bevy_event_cascade::StageCompleted;
use engine::{
    overworld::lozo::{LozoPluginWithSource, SpawnOverworldObjects},
    progress::ProgressState,
};
pub use export::*;
pub use save::*;

use crate::{
    tile::{DEFAULT_TILE_GRID_SIZE, DEFAULT_TILE_KIND, asset::TileKindAsset, io::SavedTileGrid},
    ui::CurrentLozo,
};

mod export;
mod save;

pub const DEFAULT_LOZO_ID: &str = "world";

pub struct LozoPlugin;

impl Plugin for LozoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            LozoPluginWithSource::<SavedLozoSource>::default(),
            LozoExportPlugin,
            LozoSavePlugin,
        ))
        .add_systems(Startup, load_initial_lozo)
        .add_systems(
            PreUpdate,
            spawn_initial_lozo
                .run_if(resource_exists::<InitialLozo>)
                .run_if(in_state(ProgressState::Finished)),
        );
    }
}

#[derive(Resource)]
struct InitialLozo(AssetRef<SavedLozo>);

fn load_initial_lozo(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut assets: ResMut<Assets<SavedLozo>>,
) -> Result {
    let id = "world";

    let handle = match SavedLozo::resolver().resolve(id) {
        Ok(asset_path) => {
            println!("Resolving id {id} resulted in AssetPath: {asset_path}");
            asset_server.load(asset_path)
        }
        Err(e) => {
            debug!("lozo \"{id}\" not found: {e} - constructing default lozo.");

            let default_tile_kind = AssetRef::<TileKindAsset>::new(
                DEFAULT_TILE_KIND.to_owned(),
                asset_server.load(TileKindAsset::resolver().resolve(DEFAULT_TILE_KIND)?),
            );

            assets.add(SavedLozo {
                tile_grid: SavedTileGrid {
                    width: DEFAULT_TILE_GRID_SIZE.x,
                    height: DEFAULT_TILE_GRID_SIZE.y,
                    grid: iter::repeat_n(
                        Some(default_tile_kind),
                        (DEFAULT_TILE_GRID_SIZE.x * DEFAULT_TILE_GRID_SIZE.y) as usize,
                    )
                    .collect(),
                },
                objects: Vec::new(),
                characters: Vec::new(),
            })
        }
    };

    commands.insert_resource(InitialLozo(AssetRef::new(
        DEFAULT_LOZO_ID.to_owned(),
        handle,
    )));

    Ok(())
}

fn spawn_initial_lozo(
    mut commands: Commands,
    initial_lozo: Res<InitialLozo>,
    mut saved_lozo_assets: ResMut<Assets<SavedLozo>>,
    asset_server: Res<AssetServer>,
) -> Result {
    match asset_server.get_recursive_dependency_load_state(initial_lozo.0.handle().id()) {
        None | Some(RecursiveDependencyLoadState::Loaded) => {
            log::info!("Spawning loaded lozo {}", initial_lozo.0.id());
            commands
                .spawn((
                    SavedLozoSource::from(initial_lozo.0.clone()),
                    RenderLayers::default(),
                ))
                .observe(insert_current_lozo);
            commands.remove_resource::<InitialLozo>();
        }
        Some(RecursiveDependencyLoadState::Failed(e)) => {
            log::error!("Failed to load lozo {} - {e}", initial_lozo.0.id());
            let grid = std::iter::repeat_with(|| {
                TileKindAsset::resolver()
                    .resolve(DEFAULT_TILE_KIND)
                    .map(|path| {
                        Some(AssetRef::new(
                            DEFAULT_TILE_KIND.to_owned(),
                            asset_server.load::<TileKindAsset>(path),
                        ))
                    })
            })
            .take((DEFAULT_TILE_GRID_SIZE.x * DEFAULT_TILE_GRID_SIZE.y) as usize)
            .collect::<Result<_, _>>()?;

            log::info!("Spawning default lozo from memory");
            commands
                .spawn((
                    SavedLozoSource::from(AssetRef::new(
                        DEFAULT_LOZO_ID.to_owned(),
                        saved_lozo_assets.add(SavedLozo {
                            tile_grid: SavedTileGrid {
                                width: DEFAULT_TILE_GRID_SIZE.x,
                                height: DEFAULT_TILE_GRID_SIZE.y,
                                grid,
                            },
                            objects: Vec::new(),
                            characters: Vec::new(),
                        }),
                    )),
                    RenderLayers::default(),
                ))
                .observe(insert_current_lozo);
            commands.remove_resource::<InitialLozo>();
        }
        _ => {}
    }

    Ok(())
}

fn insert_current_lozo(event: On<StageCompleted<SpawnOverworldObjects>>, mut commands: Commands) {
    commands.insert_resource(CurrentLozo::new(event.event_target()));
}
