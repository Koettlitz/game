use bevy::prelude::*;
use engine::overworld::tile::Passability;

#[derive(Component, Debug)]
pub struct Tile {
    pub passability: Passability,
    pub _sprite_stack: Vec<Entity>,
}

impl Tile {
    pub fn new(passability: Passability, sprite_stack: Vec<Entity>) -> Self {
        Self {
            passability,
            _sprite_stack: sprite_stack,
        }
    }
}
