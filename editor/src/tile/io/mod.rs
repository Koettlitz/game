use bevy::prelude::*;

pub use export::{TileGridExport, TileGridExported};
pub use save::{SavedTileGrid, SavedTileGridDef, TileGridSaved};

mod export;
mod save;

pub struct IoPlugin;

impl Plugin for IoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((export::ExportPlugin, save::TileGridSavePlugin));
    }
}
