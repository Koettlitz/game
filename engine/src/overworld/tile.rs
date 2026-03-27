use std::{fmt::Debug, iter, ops};

use bevy::prelude::*;
use macros::FromDef;
use serde::{Deserialize, Serialize};

pub const TILE_SIZE: UVec2 = UVec2::splat(32);

#[derive(
    FromDef, Component, PartialEq, Eq, Debug, Clone, Copy, Default, Serialize, Deserialize,
)]
pub enum Passability {
    #[default]
    Always,
    Never,
    Bike,
    Surf,
    Waterfall,
}

impl ops::BitAnd for Passability {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        match self {
            Self::Always => rhs,
            Self::Bike => match rhs {
                Self::Always | Self::Bike => Self::Bike,
                other => other,
            },
            Self::Surf => match rhs {
                Self::Always | Self::Bike | Self::Surf => Self::Surf,
                other => other,
            },
            Self::Waterfall => match rhs {
                Self::Always | Self::Bike | Self::Surf | Self::Waterfall => Self::Waterfall,
                other => other,
            },
            Self::Never => Self::Never,
        }
    }
}

impl ops::BitAndAssign for Passability {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

#[derive(Resource, Copy, Clone, Debug)]
pub struct GridSize(UVec2);
impl From<UVec2> for GridSize {
    fn from(value: UVec2) -> Self {
        Self(value)
    }
}
impl GridSize {
    pub fn width(&self) -> u32 {
        self.0.x
    }

    pub fn height(&self) -> u32 {
        self.0.y
    }

    pub fn as_uvec2(&self) -> UVec2 {
        self.0
    }

    pub fn as_vec2(&self) -> Vec2 {
        self.0.as_vec2()
    }

    pub fn to_grid_pos(&self, world_position: impl Into<Vec2>) -> Option<GridPosition> {
        GridPosition::new(self.to_grid_space(world_position), self)
    }

    fn to_grid_space(&self, world_position: impl Into<Vec2>) -> Vec2 {
        let mut grid_position = world_position.into();
        let half_size = self.0.as_vec2() * TILE_SIZE.as_vec2() / 2.0;
        grid_position.x += half_size.x;
        grid_position.y = half_size.y - grid_position.y;
        grid_position / TILE_SIZE.as_vec2()
    }

    pub fn to_world_pos(&self, grid_position: &GridPosition) -> Vec2 {
        self.to_world_space(grid_position.as_vec2())
    }

    fn to_world_space(&self, grid_position: impl Into<Vec2>) -> Vec2 {
        let half_size = self.0.as_vec2() * TILE_SIZE.as_vec2() / 2.0;
        let mut world_position = grid_position.into() * TILE_SIZE.as_vec2();
        world_position += TILE_SIZE.as_vec2() / 2.0;
        world_position.x -= half_size.x;
        world_position.y = half_size.y - world_position.y;
        world_position
    }

    pub fn center_on_tile(&self, world_position: impl Into<Vec2>) -> Vec2 {
        self.to_world_space(self.to_grid_space(world_position).as_uvec2().as_vec2())
    }

    pub fn iter<'a>(&'a self) -> GridIterator<'a> {
        GridIterator::new(self)
    }

    pub fn contains(&self, position: impl Into<Vec2>) -> bool {
        let position = position.into();
        position.x >= 0.0
            && position.x < self.0.x as f32
            && position.y >= 0.0
            && position.y < self.0.y as f32
    }
}

pub struct GridIterator<'a> {
    grid_size: &'a GridSize,
    current_pos: Option<UVec2>,
}
impl<'a> GridIterator<'a> {
    fn new(grid_size: &'a GridSize) -> Self {
        Self {
            grid_size,
            current_pos: Some(UVec2::splat(0)),
        }
    }
}
impl<'a> Iterator for GridIterator<'a> {
    type Item = GridPosition;
    fn next(&mut self) -> Option<Self::Item> {
        let Some(current_pos) = self.current_pos.as_mut() else {
            return None;
        };
        let result = Some(GridPosition(*current_pos));
        if current_pos.x < self.grid_size.0.x - 1 {
            current_pos.x += 1;
        } else if current_pos.y < self.grid_size.0.y - 1 {
            current_pos.x = 0;
            current_pos.y += 1;
        } else {
            self.current_pos = None;
        }
        return result;
    }
}

impl Into<UVec2> for GridSize {
    fn into(self) -> UVec2 {
        self.0
    }
}

#[derive(Resource, Default)]
pub struct TileGrid<T>(Vec<T>);

impl<T: Default> TileGrid<T> {
    pub fn with_size(size: &GridSize) -> Self {
        Self::from_fn(size, |_| T::default())
    }
}

impl<T: Copy> TileGrid<T> {
    pub fn with_tile(size: &GridSize, tile: T) -> Self {
        Self(
            iter::repeat(tile)
                .take((size.width() * size.height()) as usize)
                .collect(),
        )
    }
}

impl<T> TileGrid<T> {
    pub fn from_fn(size: &GridSize, mut constructor: impl FnMut(GridPosition) -> T) -> Self {
        let mut tiles = Vec::with_capacity((size.width() * size.height()) as usize);
        for pos in size.iter() {
            tiles.push(constructor(pos));
        }
        Self(tiles)
    }

    pub fn view_of<'a>(&'a self, adjacent: Adjacent<'a>) -> GridView<'a, T> {
        GridView {
            adjacent,
            grid: self,
        }
    }
}

impl<T> ops::Index<&GridIndex> for TileGrid<T> {
    type Output = T;
    fn index(&self, index: &GridIndex) -> &Self::Output {
        &self.0[index.0]
    }
}

impl<T> ops::IndexMut<&GridIndex> for TileGrid<T> {
    fn index_mut(&mut self, index: &GridIndex) -> &mut Self::Output {
        &mut self.0[index.0]
    }
}

pub struct GridView<'a, T> {
    adjacent: Adjacent<'a>,
    grid: &'a TileGrid<T>,
}

impl<'a, T: Debug> Debug for GridView<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{\n\ttop_left: {:?},\n\ttop: {:?},\n\ttop_right: {:?},\n\tleft: {:?},\n\tself: {:?},\n\tright: {:?},\n\t, bottom_left: {:?},\n\t, bottom: {:?},\n\t, bottom_right: {:?},\n}}",
            self.top_left(),
            self.top(),
            self.top_right(),
            self.left(),
            self.center(),
            self.right(),
            self.bottom_left(),
            self.bottom(),
            self.bottom_right()
        )
    }
}

impl<'a, T> GridView<'a, T> {
    pub fn iter_exclusive(&self) -> [Option<&T>; 8] {
        [
            self.top_left(),
            self.top(),
            self.top_right(),
            self.left(),
            self.right(),
            self.bottom_left(),
            self.bottom(),
            self.bottom_right(),
        ]
    }

    pub fn iter_inclusive(&self) -> [Option<&T>; 9] {
        [
            self.top_left(),
            self.top(),
            self.top_right(),
            self.left(),
            Some(self.center()),
            self.right(),
            self.bottom_left(),
            self.bottom(),
            self.bottom_right(),
        ]
    }

    pub fn neighbor(&self, neighbor: &Neighbor) -> Option<&T> {
        self.adjacent
            .neighbor(neighbor)
            .map(|n| &self.grid[&n.as_index(self.adjacent.grid_size)])
    }

    pub fn top_left(&self) -> Option<&T> {
        self.adjacent
            .top_left()
            .map(|p| &self.grid[&p.as_index(self.adjacent.grid_size)])
    }

    pub fn top(&self) -> Option<&T> {
        self.adjacent
            .top()
            .map(|p| &self.grid[&p.as_index(self.adjacent.grid_size)])
    }

    pub fn top_right(&self) -> Option<&T> {
        self.adjacent
            .top_right()
            .map(|p| &self.grid[&p.as_index(self.adjacent.grid_size)])
    }

    pub fn left(&self) -> Option<&T> {
        self.adjacent
            .left()
            .map(|p| &self.grid[&p.as_index(self.adjacent.grid_size)])
    }

    pub fn center(&self) -> &T {
        &self.grid[&self.adjacent.center.as_index(&self.adjacent.grid_size)]
    }

    pub fn right(&self) -> Option<&T> {
        self.adjacent
            .right()
            .map(|p| &self.grid[&p.as_index(self.adjacent.grid_size)])
    }

    pub fn bottom_left(&self) -> Option<&T> {
        self.adjacent
            .bottom_left()
            .map(|p| &self.grid[&p.as_index(self.adjacent.grid_size)])
    }

    pub fn bottom(&self) -> Option<&T> {
        self.adjacent
            .bottom()
            .map(|p| &self.grid[&p.as_index(self.adjacent.grid_size)])
    }

    pub fn bottom_right(&self) -> Option<&T> {
        self.adjacent
            .bottom_right()
            .map(|p| &self.grid[&p.as_index(self.adjacent.grid_size)])
    }
}

pub struct Adjacent<'a> {
    center: GridPosition,
    grid_size: &'a GridSize,
}

impl<'a> Adjacent<'a> {
    pub fn iter_exclusive(&self) -> [Option<GridPosition>; 8] {
        [
            self.top_left(),
            self.top(),
            self.top_right(),
            self.left(),
            self.right(),
            self.bottom_left(),
            self.bottom(),
            self.bottom_right(),
        ]
    }
    pub fn iter_inclusive(&self) -> [Option<GridPosition>; 9] {
        [
            self.top_left(),
            self.top(),
            self.top_right(),
            self.left(),
            Some(self.center),
            self.right(),
            self.bottom_left(),
            self.bottom(),
            self.bottom_right(),
        ]
    }

    pub fn neighbor(&self, neighbor: &Neighbor) -> Option<GridPosition> {
        self.center.neighbor(neighbor, self.grid_size)
    }

    pub fn top_left(&self) -> Option<GridPosition> {
        self.center.top_left(self.grid_size)
    }

    pub fn top(&self) -> Option<GridPosition> {
        self.center.top(self.grid_size)
    }

    pub fn top_right(&self) -> Option<GridPosition> {
        self.center.top_right(self.grid_size)
    }

    pub fn left(&self) -> Option<GridPosition> {
        self.center.left(self.grid_size)
    }

    pub fn center(&self) -> GridPosition {
        self.center
    }

    pub fn right(&self) -> Option<GridPosition> {
        self.center.right(self.grid_size)
    }

    pub fn bottom_left(&self) -> Option<GridPosition> {
        self.center.bottom_left(self.grid_size)
    }

    pub fn bottom(&self) -> Option<GridPosition> {
        self.center.bottom(self.grid_size)
    }

    pub fn bottom_right(&self) -> Option<GridPosition> {
        self.center.bottom_right(self.grid_size)
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct GridIndex(usize);
impl GridIndex {
    pub fn from_position(position: &GridPosition, grid_size: &GridSize) -> Self {
        Self((position.0.y * grid_size.0.x + position.0.x) as usize)
    }
}

impl ops::Deref for GridIndex {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct GridPosition(UVec2);
impl Into<UVec2> for GridPosition {
    fn into(self) -> UVec2 {
        self.0
    }
}

impl GridPosition {
    pub fn new(position: impl Into<Vec2>, grid_size: &GridSize) -> Option<Self> {
        let position = position.into();
        if grid_size.contains(position) {
            Some(Self(position.as_uvec2()))
        } else {
            None
        }
    }

    pub fn as_uvec2(self) -> UVec2 {
        self.0
    }

    pub fn as_vec2(self) -> Vec2 {
        self.0.as_vec2()
    }

    pub fn as_index(&self, grid_size: &GridSize) -> GridIndex {
        GridIndex::from_position(self, grid_size)
    }

    pub fn adjacent<'a>(self, grid_size: &'a GridSize) -> Adjacent<'a> {
        Adjacent {
            center: self,
            grid_size,
        }
    }

    pub fn neighbor(&self, neighbor: &Neighbor, grid_size: &GridSize) -> Option<Self> {
        self.0
            .checked_add_signed(neighbor.as_ivec2())
            .and_then(|p| Self::new(p.as_vec2(), grid_size))
    }

    pub fn top_left(&self, grid_size: &GridSize) -> Option<Self> {
        self.0
            .checked_sub(UVec2::splat(1))
            .and_then(|p| Self::new(p.as_vec2(), grid_size))
    }

    pub fn top(&self, grid_size: &GridSize) -> Option<Self> {
        self.0
            .checked_sub(UVec2::Y)
            .and_then(|p| Self::new(p.as_vec2(), grid_size))
    }

    pub fn top_right(&self, grid_size: &GridSize) -> Option<Self> {
        self.0
            .checked_sub(UVec2::Y)
            .map(|p| p + UVec2::X)
            .and_then(|p| Self::new(p.as_vec2(), grid_size))
    }

    pub fn left(&self, grid_size: &GridSize) -> Option<Self> {
        self.0
            .checked_sub(UVec2::X)
            .and_then(|p| Self::new(p.as_vec2(), grid_size))
    }

    pub fn right(&self, grid_size: &GridSize) -> Option<Self> {
        Self::new((self.0 + UVec2::X).as_vec2(), grid_size)
    }

    pub fn bottom_left(&self, grid_size: &GridSize) -> Option<Self> {
        self.0
            .checked_sub(UVec2::X)
            .map(|p| p + UVec2::Y)
            .and_then(|p| Self::new(p.as_vec2(), grid_size))
    }

    pub fn bottom(&self, grid_size: &GridSize) -> Option<Self> {
        Self::new((self.0 + UVec2::Y).as_vec2(), grid_size)
    }

    pub fn bottom_right(&self, grid_size: &GridSize) -> Option<Self> {
        Self::new((self.0 + UVec2::splat(1)).as_vec2(), grid_size)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Neighbor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Neighbor {
    pub fn as_ivec2(&self) -> IVec2 {
        match self {
            Self::TopLeft => IVec2::splat(-1),
            Self::Top => IVec2::new(0, -1),
            Self::TopRight => IVec2::new(1, -1),
            Self::Left => IVec2::new(-1, 0),
            Self::Right => IVec2::new(1, 0),
            Self::BottomLeft => IVec2::new(-1, 1),
            Self::Bottom => IVec2::new(0, 1),
            Self::BottomRight => IVec2::splat(1),
        }
    }
}

#[cfg(test)]
mod test {
    use bevy::math::{UVec2, Vec2};

    use crate::overworld::tile::GridSize;

    const GRID_SIZE: GridSize = GridSize(UVec2::splat(10));

    #[test]
    fn world_origin_maps_to_grid_center() {
        let half_grid_size = GRID_SIZE.0 / 2;
        test_to_grid_pos(Vec2::splat(0.0), half_grid_size);
    }

    #[test]
    fn left_of_world_origin_maps_to_left_of_grid_center() {
        let half_grid_size = GRID_SIZE.0 / 2;
        test_to_grid_pos(
            Vec2::new(-30.0, 0.0),
            half_grid_size.with_x(half_grid_size.x - 1),
        );
    }

    fn test_to_grid_pos(world_position: impl Into<Vec2>, expected: impl Into<UVec2>) {
        let Some(result) = GRID_SIZE.to_grid_pos(world_position.into()) else {
            panic!("expected grid center, but got None");
        };
        assert_eq!(result.as_uvec2(), expected.into());
    }
}
