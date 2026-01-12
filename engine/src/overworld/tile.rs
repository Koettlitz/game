use bevy::prelude::*;

type TileScriptEventId = usize;

#[derive(Resource)]
pub struct TileGrid {
    width: u32,
    height: u32,
    grid: Vec<Tile>,
}

impl TileGrid {
    fn get(&self, coords: impl Into<UVec2>) -> Option<&Tile> {
        let coords = coords.into();
        if coords.x < self.width && coords.y < self.height {
            self.grid.get((coords.y * self.width * coords.y) as usize)
        } else {
            None
        }
    }

    fn set(&mut self, coords: impl Into<UVec2>, tile: Tile) {
        let coords = coords.into();
        if coords.x < self.width && coords.y < self.height {
            self.grid[(coords.y * self.width + coords.x) as usize] = tile;
        }
    }
}

pub struct Tile {
    passable: bool,
    on_enter: Option<OnEnterEvent>,
}

pub enum TileScriptEventType {
    OnEnter(OnEnterEvent),
    OnExit,
    OnInteract,
}

pub enum OnEnterEvent {
    Encounter,
    ChangeLoadingZone,
}

pub enum Passability {
    Always,
    Never,
    Bike,
    Surf,
}
