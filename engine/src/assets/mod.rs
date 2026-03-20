use std::marker::PhantomData;

use bevy::{
    asset::Handle,
    ecs::{component::Component, resource::Resource},
    image::{Image, TextureAtlasLayout},
};
pub use folder::{AssetMap, AssetSet, AssetSetPlugin, FileAsset, LoadState};
pub use spawn::{EntityFolderPlugin, EntityLookupMap};

pub mod animations;
pub mod folder;
pub mod object;
pub mod overworld;
mod spawn;
pub mod tile;

type Phantom<L> = PhantomData<fn() -> L>;

#[derive(Component)]
pub struct SpriteSheet {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

pub trait SpriteSheetMap: Resource {
    fn insert(&mut self, id: String, value: SpriteSheet);
    fn get(&self, id: &str) -> Option<&SpriteSheet>;
    fn remove(&mut self, id: &str) -> Option<SpriteSheet>;
}
