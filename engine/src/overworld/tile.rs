use std::{
    fmt::{Debug, Display},
    iter,
    ops::{self, Deref},
};

use bevy::{ecs::system::SystemParam, prelude::*};
use macros::FromDef;
use serde::{Deserialize, Serialize};

pub const TILE_SIZE: UVec2 = UVec2::splat(32);

#[derive(SystemParam)]
pub struct GridCommands<'w, 's>(Commands<'w, 's>);

impl<'w, 's> GridCommands<'w, 's> {
    pub fn spawn_default<T>(&mut self, size: impl Into<UVec2>) -> EntityCommands<'_>
    where
        T: Default + Send + Sync + 'static,
    {
        let size = GridSize(size.into());
        let grid = Grid::<T>::with_size(&size);
        self.0.spawn((grid, size))
    }

    pub fn spawn_with_tile<T>(&mut self, size: impl Into<UVec2>, tile: T) -> EntityCommands<'_>
    where
        T: Copy + Send + Sync + 'static,
    {
        let size = GridSize(size.into());
        let grid = Grid::<T>::with_tile(&size, tile);
        self.0.spawn((grid, size))
    }

    pub fn spawn_from_fn<T>(
        &mut self,
        size: impl Into<UVec2>,
        constructor: impl FnMut(GridPosition) -> T,
    ) -> EntityCommands<'_>
    where
        T: Send + Sync + 'static,
    {
        let size = GridSize(size.into());
        let grid = Grid::from_fn(&size, constructor);
        self.0.spawn((grid, size))
    }

    pub fn spawn_from_fn_result<T>(
        &mut self,
        size: impl Into<UVec2>,
        constructor: impl FnMut(GridPosition) -> Result<T>,
    ) -> Result<EntityCommands<'_>>
    where
        T: Send + Sync + 'static,
    {
        let size = GridSize(size.into());
        let grid = Grid::from_fn_result(&size, constructor)?;
        Ok(self.0.spawn((grid, size)))
    }
}

pub fn create_grid_bundle<T>(
    size: impl Into<UVec2>,
    constructor: impl FnMut(GridPosition) -> Result<T>,
) -> Result<impl Bundle>
where
    T: Send + Sync + 'static,
{
    let size = GridSize(size.into());
    let grid = Grid::from_fn_result(&size, constructor)?;
    Ok((grid, size))
}

#[derive(Component, Copy, Clone, Debug)]
pub struct GridSize(UVec2);
impl GridSize {
    pub fn new(width: u32, height: u32) -> Self {
        Self(UVec2::new(width, height))
    }

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

    pub fn tile_count(&self) -> u32 {
        self.width() * self.height()
    }

    pub fn to_grid_pos<'a>(&'a self, world_position: impl Into<Vec2>) -> Option<GridPosition<'a>> {
        let grid_position = self.to_grid_space(world_position);
        if self.contains_arbitrary(grid_position) {
            Some(GridPosition {
                pos: grid_position.as_uvec2(),
                grid_size: self,
            })
        } else {
            None
        }
    }

    fn to_grid_space(&self, world_position: impl Into<Vec2>) -> Vec2 {
        let mut grid_position = world_position.into();
        let half_size = self.0.as_vec2() * TILE_SIZE.as_vec2() / 2.0;
        grid_position.x += half_size.x;
        grid_position.y = half_size.y - grid_position.y;
        grid_position / TILE_SIZE.as_vec2()
    }

    pub fn to_world_pos(&self, grid_position: impl Into<UVec2>) -> Vec2 {
        let half_size = self.0.as_vec2() * TILE_SIZE.as_vec2() / 2.0;
        let mut world_position = grid_position.into().as_vec2() * TILE_SIZE.as_vec2();
        world_position += TILE_SIZE.as_vec2() / 2.0;
        world_position.x -= half_size.x;
        world_position.y = half_size.y - world_position.y;
        world_position
    }

    pub fn center_on_tile(&self, world_position: impl Into<Vec2>) -> Vec2 {
        self.to_world_pos(self.to_grid_space(world_position).as_uvec2())
    }

    pub fn iter<'a>(&'a self) -> GridPosIterator<'a> {
        GridPosIterator::new(self)
    }

    pub fn contains(&self, position: impl Into<UVec2>) -> bool {
        let position = position.into();
        position.x < self.width() && position.y < self.height()
    }

    pub fn contains_arbitrary(&self, position: impl Into<Vec2>) -> bool {
        let position = position.into();
        position.x >= 0.0
            && position.x < self.0.x as f32
            && position.y >= 0.0
            && position.y < self.0.y as f32
    }
}

pub struct GridPosIterator<'a> {
    grid_size: &'a GridSize,
    current_pos: Option<UVec2>,
}

impl<'a> GridPosIterator<'a> {
    fn new(grid_size: &'a GridSize) -> Self {
        Self {
            grid_size,
            current_pos: Some(UVec2::splat(0)),
        }
    }
}

impl<'a> Iterator for GridPosIterator<'a> {
    type Item = GridPosition<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(current_pos) = self.current_pos.as_mut() else {
            return None;
        };
        let result = Some(GridPosition {
            pos: *current_pos,
            grid_size: self.grid_size,
        });
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

#[derive(Component, Default)]
pub struct Grid<T>(Vec<T>);

impl<T: Default> Grid<T> {
    fn with_size(size: &GridSize) -> Self {
        Self::from_fn(size, |_| T::default())
    }
}

impl<T: Copy> Grid<T> {
    fn with_tile(size: &GridSize, tile: T) -> Self {
        Self(
            iter::repeat(tile)
                .take((size.width() * size.height()) as usize)
                .collect(),
        )
    }
}

impl<T> Grid<T> {
    fn from_fn(size: &GridSize, mut constructor: impl FnMut(GridPosition) -> T) -> Self {
        Self::from_fn_result(size, |pos| Ok(constructor(pos))).unwrap()
    }

    fn from_fn_result(
        size: &GridSize,
        mut constructor: impl FnMut(GridPosition) -> Result<T>,
    ) -> Result<Self> {
        let mut tiles = Vec::with_capacity((size.width() * size.height()) as usize);
        for pos in size.iter() {
            tiles.push(constructor(pos)?);
        }
        Ok(Self(tiles))
    }

    pub fn cursor_at<'a>(&'a mut self, position: GridPosition<'a>) -> GridCursor<'a, T> {
        GridCursor {
            position,
            grid: self,
        }
    }
}

impl<T> ops::Index<GridIndex> for Grid<T> {
    type Output = T;
    fn index(&self, index: GridIndex) -> &Self::Output {
        &self.0[index.0]
    }
}

impl<T> ops::IndexMut<GridIndex> for Grid<T> {
    fn index_mut(&mut self, index: GridIndex) -> &mut Self::Output {
        &mut self.0[index.0]
    }
}

impl<T> ops::Index<GridPosition<'_>> for Grid<T> {
    type Output = T;

    fn index(&self, position: GridPosition) -> &Self::Output {
        &self[GridIndex::from_position(position)]
    }
}

impl<T> ops::IndexMut<GridPosition<'_>> for Grid<T> {
    fn index_mut(&mut self, position: GridPosition) -> &mut Self::Output {
        &mut self[GridIndex::from_position(position)]
    }
}

pub struct GridCursor<'a, T> {
    position: GridPosition<'a>,
    grid: &'a Grid<T>,
}

impl<'a, T: Debug> Debug for GridCursor<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{\n\ttop_left: {:?},\n\ttop: {:?},\n\ttop_right: {:?},\n\tleft: {:?},\n\tself: {:?},\n\tright: {:?},\n\t, bottom_left: {:?},\n\t, bottom: {:?},\n\t, bottom_right: {:?},\n}}",
            self.top_left(),
            self.top(),
            self.top_right(),
            self.left(),
            self.get(),
            self.right(),
            self.bottom_left(),
            self.bottom(),
            self.bottom_right()
        )
    }
}

impl<'a, T> GridCursor<'a, T> {
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
            Some(self.get()),
            self.right(),
            self.bottom_left(),
            self.bottom(),
            self.bottom_right(),
        ]
    }

    pub fn neighbor(&self, neighbor: &Neighbor) -> Option<&T> {
        self.position.neighbor(neighbor).map(|n| &self.grid[n])
    }

    pub fn top_left(&self) -> Option<&T> {
        self.position.top_left().map(|p| &self.grid[p])
    }

    pub fn top(&self) -> Option<&T> {
        self.position.top().map(|p| &self.grid[p])
    }

    pub fn top_right(&self) -> Option<&T> {
        self.position.top_right().map(|p| &self.grid[p])
    }

    pub fn left(&self) -> Option<&T> {
        self.position.left().map(|p| &self.grid[p])
    }

    pub fn get(&self) -> &T {
        &self.grid[self.position]
    }

    pub fn right(&self) -> Option<&T> {
        self.position.right().map(|p| &self.grid[p])
    }

    pub fn bottom_left(&self) -> Option<&T> {
        self.position.bottom_left().map(|p| &self.grid[p])
    }

    pub fn bottom(&self) -> Option<&T> {
        self.position.bottom().map(|p| &self.grid[p])
    }

    pub fn bottom_right(&self) -> Option<&T> {
        self.position.bottom_right().map(|p| &self.grid[p])
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GridPosition<'a> {
    pos: UVec2,
    grid_size: &'a GridSize,
}

impl<'a> Deref for GridPosition<'a> {
    type Target = UVec2;

    fn deref(&self) -> &Self::Target {
        &self.pos
    }
}

impl<'a> Display for GridPosition<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pos)
    }
}

impl<'a> GridPosition<'a> {
    pub fn new(position: impl Into<UVec2>, grid_size: &'a GridSize) -> Option<Self> {
        let position = position.into();
        if grid_size.contains(position) {
            Some(Self {
                pos: position,
                grid_size,
            })
        } else {
            None
        }
    }

    pub fn as_index(self) -> GridIndex {
        GridIndex::from_position(self)
    }

    pub fn to_world_pos(&self) -> Vec2 {
        self.grid_size.to_world_pos(**self)
    }

    pub fn around_exclusive(&self) -> [Option<Self>; 8] {
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
    pub fn around_inclusive(&self) -> [Option<Self>; 9] {
        [
            self.top_left(),
            self.top(),
            self.top_right(),
            self.left(),
            Some(*self),
            self.right(),
            self.bottom_left(),
            self.bottom(),
            self.bottom_right(),
        ]
    }

    pub fn neighbor(&self, neighbor: &Neighbor) -> Option<Self> {
        self.checked_add_signed(neighbor.as_ivec2())
            .and_then(|p| Self::new(p, self.grid_size))
    }

    pub fn top_left(&self) -> Option<Self> {
        self.checked_sub(UVec2::splat(1))
            .and_then(|p| Self::new(p, self.grid_size))
    }

    pub fn top(&self) -> Option<Self> {
        self.checked_sub(UVec2::Y)
            .and_then(|p| Self::new(p, self.grid_size))
    }

    pub fn top_right(&self) -> Option<Self> {
        self.checked_sub(UVec2::Y)
            .map(|p| p + UVec2::X)
            .and_then(|p| Self::new(p, self.grid_size))
    }

    pub fn left(&self) -> Option<Self> {
        self.checked_sub(UVec2::X)
            .and_then(|p| Self::new(p, self.grid_size))
    }

    pub fn right(&self) -> Option<Self> {
        Self::new(**self + UVec2::X, self.grid_size)
    }

    pub fn bottom_left(&self) -> Option<Self> {
        self.checked_sub(UVec2::X)
            .map(|p| p + UVec2::Y)
            .and_then(|p| Self::new(p, self.grid_size))
    }

    pub fn bottom(&self) -> Option<Self> {
        Self::new(**self + UVec2::Y, self.grid_size)
    }

    pub fn bottom_right(&self) -> Option<Self> {
        Self::new(**self + UVec2::splat(1), self.grid_size)
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct GridIndex(usize);
impl GridIndex {
    pub fn from_position(position: GridPosition) -> Self {
        Self((position.y * position.grid_size.width() + position.x) as usize)
    }
}

impl ops::Deref for GridIndex {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(FromDef, Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[def_type(Self)]
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

#[derive(FromDef, Component, PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize, Hash)]
#[def_type(Self)]
pub enum Passability {
    Always,
    Never,
    Bike,
    Surf,
    Waterfall,
}

impl Default for Passability {
    fn default() -> Self {
        Self::Always
    }
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
        assert_eq!(*result, expected.into());
    }
}
