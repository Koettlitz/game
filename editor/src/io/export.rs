use std::{io, path::Path};

use bevy::prelude::*;

use crate::tile::GroundTileGrid;

#[derive(Event)]
pub struct ExportLozoCommand;

pub fn export_lozo(_: On<ExportLozoCommand>, tile_grid: Res<GroundTileGrid>) -> io::Result<()> {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("crate root {} has no parent dir?", crate_root.display()),
        )
    })?;
    let game_asset_root = workspace_root.join("game").join("assets");

    Ok(())
}
