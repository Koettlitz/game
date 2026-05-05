use bevy::prelude::*;

use crate::io::export::ExportPlugin;
pub mod export;

pub struct IoPlugin;
impl Plugin for IoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExportPlugin);
    }
}
