use bevy::prelude::*;
use macros::FromDef;
use serde::Deserialize;

#[derive(Component)]
pub struct SpriteSheet {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

#[derive(FromDef, Asset, Deserialize, TypePath)]
pub struct SpriteSheetAsset;

pub trait SpriteSheetMap: Resource {
    fn insert(&mut self, id: String, value: SpriteSheet);
    fn get(&self, id: &str) -> Option<&SpriteSheet>;
    fn remove(&mut self, id: &str) -> Option<SpriteSheet>;
}
