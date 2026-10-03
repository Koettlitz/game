use bevy::prelude::*;

pub use export::*;
pub use save::*;

mod export;
mod save;

pub struct LozoIoPlugin;

impl Plugin for LozoIoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((LozoExportPlugin, LozoSavePlugin));
    }
}
