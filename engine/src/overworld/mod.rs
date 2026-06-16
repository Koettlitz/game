use bevy::prelude::*;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};
use thiserror::Error;

pub mod character;
pub mod input;
pub mod lozo;
pub mod tile;

#[derive(Component, Default)]
pub struct ObjectSpriteLookup(HashMap<String, Entity>);

impl Deref for ObjectSpriteLookup {
    type Target = HashMap<String, Entity>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ObjectSpriteLookup {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ObjectSpriteLookup {
    pub fn lookup(&self, id: &str) -> Result<Entity> {
        Ok(self
            .get(id)
            .ok_or_else(|| ObjectSpriteLookupFailed(id.to_string()))
            .map(|e| *e)?)
    }
}

#[derive(Error, Debug)]
#[error("missing object sprite \"{0}\" in ObjectSpriteLookup")]
pub struct ObjectSpriteLookupFailed(String);
