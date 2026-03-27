use std::collections::HashMap;
use std::hash::Hash;
use std::io;
use std::marker::PhantomData;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AssetPath, LoadContext, ParseAssetPathError};
use bevy::prelude::*;
pub use folder::{AssetMap, AssetSetPlugin, LoadState};
use macros::from_def_self;
use ron::de::SpannedError;
use serde::de::DeserializeOwned;
pub use spawn::{EntityFolderPlugin, EntityLookupMap};
use thiserror::Error;

pub mod animations;
pub mod folder;
pub mod overworld;
mod spawn;
pub mod sprite_sheet;

mod registry {
    include!(concat!(env!("OUT_DIR"), "/asset_registry.rs"));
}

pub type Phantom<L> = PhantomData<fn() -> L>;

pub trait AssetResolver {
    type Asset: Asset;

    const BASE_PATH: &'static str;
    const EXTENSION: &'static str;

    fn resolve(asset_id: &str) -> Result<AssetPath<'static>, ParseAssetPathError> {
        AssetPath::from(Self::BASE_PATH).resolve(&(asset_id.to_string() + "." + Self::EXTENSION))
    }
}

pub trait FromDef {
    type Def: DeserializeOwned;
    type Error: From<ParseAssetPathError>;

    fn from_def<R: AssetResolver>(
        def: Self::Def,
        ctx: &mut LoadContext,
    ) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

from_def_self![
    u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64, String
];

impl<A: Asset> FromDef for Handle<A> {
    type Def = String;
    type Error = ParseAssetPathError;

    fn from_def<R: AssetResolver>(
        def: Self::Def,
        ctx: &mut LoadContext,
    ) -> Result<Self, Self::Error> {
        Ok(ctx.load(R::resolve(&def)?))
    }
}

impl<A, D> FromDef for Option<A>
where
    A: FromDef<Def = D>,
    D: DeserializeOwned,
{
    type Def = Option<D>;
    type Error = A::Error;

    fn from_def<R: AssetResolver>(
        def: Self::Def,
        ctx: &mut LoadContext,
    ) -> Result<Self, Self::Error> {
        Ok(def.map(|d| A::from_def::<R>(d, ctx)).transpose()?)
    }
}

impl<A, D> FromDef for Vec<A>
where
    A: FromDef<Def = D>,
    D: DeserializeOwned,
{
    type Def = Vec<D>;
    type Error = A::Error;

    fn from_def<R: AssetResolver>(
        def: Self::Def,
        ctx: &mut LoadContext,
    ) -> Result<Self, Self::Error> {
        def.into_iter().map(|d| A::from_def::<R>(d, ctx)).collect()
    }
}

impl<A, K, D> FromDef for HashMap<K, A>
where
    A: FromDef<Def = D>,
    K: DeserializeOwned + Eq + Hash,
    D: DeserializeOwned,
{
    type Def = HashMap<K, D>;
    type Error = A::Error;

    fn from_def<R: AssetResolver>(
        def: Self::Def,
        ctx: &mut LoadContext,
    ) -> Result<Self, Self::Error> {
        def.into_iter()
            .map(|(k, d)| Ok((k, A::from_def::<R>(d, ctx)?)))
            .collect()
    }
}

// // Generic impl apparently "too generic" for rust to accept ;(
// // Maybe one day that's allowed...
// impl<A, S, T> GameAsset for T
// where
//     Self: FromIterator<A> + Sized,
//     A: GameAsset,
//     S: IntoIterator<Item = A::Def> + DeserializeOwned,
// {
//     type Def = S;
//     type Error = A::Error;
//     fn from_def<R: AssetResolver>(
//         def: Self::Def,
//         ctx: &mut LoadContext,
//     ) -> Result<Self, Self::Error> {
//         def.into_iter().map(|d| A::from_def(d, ctx)).collect()
//     }
// }

pub struct GameAssetPlugin<A, R>(Phantom<A>, Phantom<R>);
impl<A, R> Default for GameAssetPlugin<A, R> {
    fn default() -> Self {
        Self(PhantomData::default(), PhantomData::default())
    }
}

impl<A, R> Plugin for GameAssetPlugin<A, R>
where
    A: FromDef + Asset,
    R: AssetResolver + TypePath,
    GameAssetLoadError: From<<A as FromDef>::Error>,
{
    fn build(&self, app: &mut App) {
        app.init_asset::<A>()
            .init_asset_loader::<GameAssetLoader<A, R>>();
    }
}

#[derive(TypePath)]
pub struct GameAssetLoader<A, R>(Phantom<A>, Phantom<R>);
impl<A, R> AssetLoader for GameAssetLoader<A, R>
where
    A: FromDef + Asset + TypePath,
    R: AssetResolver + TypePath,
    GameAssetLoadError: From<<A as FromDef>::Error>,
{
    type Asset = A;
    type Error = GameAssetLoadError;
    type Settings = ();

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let def: A::Def = ron::de::from_bytes(&bytes)?;
        Ok(A::from_def::<R>(def, load_context)?)
    }
}

impl<A, R> Default for GameAssetLoader<A, R>
where
    A: FromDef,
    R: AssetResolver,
{
    fn default() -> Self {
        Self(PhantomData::default(), PhantomData::default())
    }
}

#[derive(Error, Debug)]
pub enum GameAssetLoadError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Spanned(#[from] SpannedError),
    #[error("{0}")]
    InvalidAssetLink(#[from] ParseAssetPathError),
}

pub mod one_or_many {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match OneOrMany::deserialize(deserializer)? {
            OneOrMany::One(s) => Ok(vec![s]),
            OneOrMany::Many(v) => Ok(v),
        }
    }

    pub fn serialize<S>(value: &Vec<String>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if value.len() == 1 {
            serializer.serialize_str(&value[0])
        } else {
            value.serialize(serializer)
        }
    }
}
