use std::iter;

use bevy::prelude::*;

#[derive(PartialEq, Eq, Debug, Clone, Copy, Default)]
pub enum Passability {
    #[default]
    Always,
    Never,
    Bike,
    Surf,
}

#[derive(Resource)]
pub struct TileGrid<T> {
    width: u32,
    height: u32,
    grid: Vec<T>,
}

#[derive(Resource, Copy, Clone)]
pub struct GridSize(pub UVec2);
impl GridSize {
    pub fn contains(&self, position: impl Into<Vec2>) -> bool {
        let position = position.into();
        position.x >= 0.0
            && position.x < self.0.x as f32
            && position.y >= 0.0
            && position.y < self.0.y as f32
    }
}

impl Into<UVec2> for GridSize {
    fn into(self) -> UVec2 {
        self.0
    }
}

impl<T: Default> TileGrid<T> {
    pub fn new(size: impl Into<UVec2>) -> Self {
        let size = size.into();
        let width = size.x;
        let height = size.y;
        let grid = iter::repeat_with(T::default)
            .take((width * height) as usize)
            .collect();
        Self {
            width,
            height,
            grid,
        }
    }
}

impl<T: Copy> TileGrid<T> {
    pub fn get(&self, coords: impl Into<UVec2>) -> Option<T> {
        let coords = coords.into();
        if coords.x < self.width && coords.y < self.height {
            self.grid
                .get((coords.y * self.width + coords.x) as usize)
                .copied()
        } else {
            None
        }
    }
}

impl<T> TileGrid<T> {
    pub fn get_mut(&mut self, coords: impl Into<UVec2>) -> Option<&mut T> {
        let coords = coords.into();
        if coords.x < self.width && coords.y < self.height {
            self.grid
                .get_mut((coords.y * self.width + coords.x) as usize)
        } else {
            None
        }
    }

    pub fn set(&mut self, coords: impl Into<UVec2>, tile: T) {
        let coords = coords.into();
        if coords.x < self.width && coords.y < self.height {
            self.grid[(coords.y * self.width + coords.x) as usize] = tile;
        }
    }

    pub fn contains(&self, coords: impl Into<Vec2>) -> bool {
        let coords = coords.into();
        coords.x >= 0.0
            && coords.x < self.width as f32
            && coords.y >= 0.0
            && coords.y < self.height as f32
    }
}
