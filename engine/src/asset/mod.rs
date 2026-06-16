use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::io;
use std::marker::PhantomData;
use std::time::Duration;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AssetPath, LoadContext, ParseAssetPathError};
use bevy::prelude::*;
use macros::from_def_self;
use ron::de::SpannedError;
use serde::de::DeserializeOwned;
pub use set::{AssetMap, AssetSetPlugin, LoadState};
use thiserror::Error;

use set::InvalidAssetLinkError;

pub mod animation;
pub mod overworld;
pub mod set;
pub mod spritesheet;

pub type Phantom<L> = PhantomData<fn() -> L>;

pub trait AssetResolver {
    // TODO: error type sollte nicht FromDefError sein hier
    fn resolve(&self, asset_id: &str) -> Result<AssetPath<'static>, FromDefError>;
}

pub trait StaticAssetResolver {
    fn resolve(asset_id: &str) -> Result<AssetPath<'static>, FromDefError>;
}

pub struct PathResolver;
impl AssetResolver for PathResolver {
    fn resolve(&self, asset_path: &str) -> Result<AssetPath<'static>, FromDefError> {
        Ok(AssetPath::from(asset_path.to_string()))
    }
}

pub fn extract_id_from(asset_path: AssetPath) -> String {
    asset_path
        .path()
        .file_prefix()
        .unwrap_or_else(|| panic!("missing file name in asset path {asset_path}"))
        .to_string_lossy()
        .to_string()
}

#[derive(Error, Debug)]
pub enum FromDefError {
    #[error("{0}")]
    Parse(#[from] ParseAssetPathError),
    #[error("{0}")]
    InvalidAssetLink(#[from] InvalidAssetLinkError),
    #[error("{0}")]
    InvalidDef(String),
}

pub struct StaticResolverAdapter<S>(Phantom<S>);

impl<S> Default for StaticResolverAdapter<S> {
    fn default() -> Self {
        Self(PhantomData::default())
    }
}

impl<S: StaticAssetResolver> AssetResolver for StaticResolverAdapter<S> {
    fn resolve(&self, asset_id: &str) -> Result<AssetPath<'static>, FromDefError> {
        S::resolve(asset_id)
    }
}

pub trait HasResolver {
    type Resolver: AssetResolver;

    fn resolver() -> Self::Resolver;
}

impl<T: AssetResolver + Default> HasResolver for T {
    type Resolver = Self;

    fn resolver() -> Self::Resolver {
        Self::default()
    }
}

pub trait AssetPathSpec {
    const BASE_PATH: &'static str;
    const EXTENSION: Option<&'static str> = None;
}

pub struct ResolverSpec<S>(Phantom<S>);

impl<S> Default for ResolverSpec<S> {
    fn default() -> Self {
        Self(PhantomData::default())
    }
}

impl<S: AssetPathSpec> AssetPathSpecProvider for ResolverSpec<S> {
    fn base_path(&self) -> Cow<'static, str> {
        Cow::Borrowed(S::BASE_PATH)
    }

    fn extension(&self) -> Option<&'static str> {
        S::EXTENSION
    }
}

pub struct DynamicPathResolver {
    pub base_path: String,
    pub extension: Option<&'static str>,
}

impl DynamicPathResolver {
    pub fn resolve_sub_path(
        load_context: &mut LoadContext,
        sub_path: &str,
        extension: Option<&'static str>,
    ) -> Result<Self, ParseAssetPathError> {
        let base_path = load_context
            .path()
            .parent()
            .map(|p| p.resolve(sub_path))
            .transpose()?
            .unwrap_or_else(|| AssetPath::parse(sub_path))
            .to_string();
        Ok(Self {
            base_path,
            extension,
        })
    }
}

impl AssetPathSpecProvider for DynamicPathResolver {
    fn base_path(&self) -> Cow<'static, str> {
        Cow::Owned(self.base_path.clone())
    }

    fn extension(&self) -> Option<&'static str> {
        self.extension
    }
}

pub struct SpecResolver<S>(Phantom<S>);

impl<S> Default for SpecResolver<S> {
    fn default() -> Self {
        Self(PhantomData::default())
    }
}

impl<S: HasSpecProvider> AssetPathSpecProvider for SpecResolver<S> {
    fn base_path(&self) -> Cow<'static, str> {
        S::provider().base_path()
    }

    fn extension(&self) -> Option<&'static str> {
        S::provider().extension()
    }
}

pub trait AssetPathSpecProvider {
    fn base_path(&self) -> Cow<'static, str>;
    fn extension(&self) -> Option<&'static str> {
        None
    }
}

pub trait HasSpecProvider {
    type Provider: AssetPathSpecProvider;

    fn provider() -> Self::Provider;
}

impl<T> HasSpecProvider for T
where
    T: AssetPathSpecProvider + Default,
{
    type Provider = Self;

    fn provider() -> Self::Provider {
        Self::default()
    }
}

impl<T> AssetResolver for T
where
    T: AssetPathSpecProvider,
{
    fn resolve(&self, asset_id: &str) -> Result<AssetPath<'static>, FromDefError> {
        let file_name = if let Some(ext) = self.extension() {
            Cow::Owned(asset_id.to_string() + "." + ext)
        } else {
            Cow::Borrowed(asset_id)
        };
        Ok(AssetPath::from(self.base_path().to_string()).resolve(file_name.as_ref())?)
    }
}

pub trait FromDef {
    type Def: DeserializeOwned;
    type Error: Into<FromDefError>;

    fn from_def(def: Self::Def, ctx: &mut LoadContext) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

pub trait FromDefWithResolver {
    type Def: DeserializeOwned;
    type Error: Into<FromDefError>;

    fn from_def_with_resolver<R: AssetResolver>(
        def: Self::Def,
        resolver: &R,
        ctx: &mut LoadContext,
    ) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

#[derive(Debug, Eq)]
pub struct AssetRef<A: Asset> {
    id: String,
    handle: Handle<A>,
}

impl<A: Asset> AssetRef<A> {
    pub fn new(id: String, handle: Handle<A>) -> Self {
        Self { id, handle }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn handle(&self) -> &Handle<A> {
        &self.handle
    }
}

impl<A: Asset + HasResolver> FromDef for AssetRef<A> {
    type Def = String;
    type Error = FromDefError;

    fn from_def(def: Self::Def, ctx: &mut LoadContext) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let handle = ctx.load(A::resolver().resolve(&def)?);
        Ok(Self { id: def, handle })
    }
}

impl<A: Asset> FromDefWithResolver for AssetRef<A> {
    type Def = String;
    type Error = FromDefError;

    fn from_def_with_resolver<R: AssetResolver>(
        def: Self::Def,
        resolver: &R,
        ctx: &mut LoadContext,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            handle: ctx.load(resolver.resolve(&def)?),
            id: def,
        })
    }
}

impl<A: Asset + HasResolver> FromDef for Handle<A> {
    type Def = String;
    type Error = FromDefError;

    fn from_def(def: Self::Def, ctx: &mut LoadContext) -> Result<Self, Self::Error> {
        Ok(ctx.load(A::resolver().resolve(&def)?))
    }
}

impl<A: Asset> FromDefWithResolver for Handle<A> {
    type Def = String;
    type Error = FromDefError;

    fn from_def_with_resolver<R: AssetResolver>(
        def: Self::Def,
        resolver: &R,
        ctx: &mut LoadContext,
    ) -> Result<Self, Self::Error> {
        Ok(ctx.load(resolver.resolve(&def)?))
    }
}

impl<T, D> FromDef for Option<T>
where
    T: FromDef<Def = D>,
    D: DeserializeOwned,
{
    type Def = Option<D>;
    type Error = T::Error;

    fn from_def(def: Self::Def, ctx: &mut LoadContext) -> Result<Self, Self::Error> {
        Ok(def.map(|d| T::from_def(d, ctx)).transpose()?)
    }
}

impl<T, D> FromDefWithResolver for Option<T>
where
    T: FromDefWithResolver<Def = D>,
    D: DeserializeOwned,
{
    type Def = Option<D>;
    type Error = T::Error;

    fn from_def_with_resolver<R: AssetResolver>(
        def: Self::Def,
        resolver: &R,
        ctx: &mut LoadContext,
    ) -> Result<Self, Self::Error> {
        Ok(def
            .map(|d| T::from_def_with_resolver(d, resolver, ctx))
            .transpose()?)
    }
}

impl<T, D> FromDef for Vec<T>
where
    T: FromDef<Def = D>,
    D: DeserializeOwned,
{
    type Def = Vec<D>;
    type Error = T::Error;

    fn from_def(def: Self::Def, ctx: &mut LoadContext) -> Result<Self, Self::Error> {
        def.into_iter().map(|d| T::from_def(d, ctx)).collect()
    }
}

impl<T, D> FromDefWithResolver for Vec<T>
where
    T: FromDefWithResolver<Def = D>,
    D: DeserializeOwned,
{
    type Def = Vec<D>;
    type Error = T::Error;

    fn from_def_with_resolver<R: AssetResolver>(
        def: Self::Def,
        resolver: &R,
        ctx: &mut LoadContext,
    ) -> Result<Self, Self::Error> {
        def.into_iter()
            .map(|d| T::from_def_with_resolver(d, resolver, ctx))
            .collect()
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

    fn from_def(def: Self::Def, ctx: &mut LoadContext) -> Result<Self, Self::Error> {
        def.into_iter()
            .map(|(k, d)| Ok((k, A::from_def(d, ctx)?)))
            .collect()
    }
}

impl<A, K, D> FromDefWithResolver for HashMap<K, A>
where
    A: FromDefWithResolver<Def = D>,
    K: DeserializeOwned + Eq + Hash,
    D: DeserializeOwned,
{
    type Def = HashMap<K, D>;
    type Error = A::Error;

    fn from_def_with_resolver<R: AssetResolver>(
        def: Self::Def,
        resolver: &R,
        ctx: &mut LoadContext,
    ) -> Result<Self, Self::Error> {
        def.into_iter()
            .map(|(k, d)| Ok((k, A::from_def_with_resolver(d, resolver, ctx)?)))
            .collect()
    }
}

from_def_self![
    u8,
    u16,
    u32,
    u64,
    usize,
    i8,
    i16,
    i32,
    i64,
    isize,
    f32,
    f64,
    String,
    IRect,
    IVec2,
    Vec2,
    Vec3,
    UVec2,
    TextureAtlasLayout,
    Duration
];

/// Registers the asset type `A` and a `RonAssetLoader<A>`
pub struct RonAssetPlugin<A>(Phantom<A>);
impl<A> Default for RonAssetPlugin<A> {
    fn default() -> Self {
        Self(PhantomData::default())
    }
}

impl<A> Plugin for RonAssetPlugin<A>
where
    A: Asset + FromDef + 'static,
    RonAssetLoadError: From<<A as FromDef>::Error>,
{
    fn build(&self, app: &mut App) {
        app.init_asset::<A>()
            .init_asset_loader::<RonAssetLoader<A>>();
    }
}

#[derive(TypePath)]
pub struct RonAssetLoader<A>(Phantom<A>);
impl<A> AssetLoader for RonAssetLoader<A>
where
    A: FromDef + Asset,
    RonAssetLoadError: From<<A as FromDef>::Error>,
{
    type Asset = A;
    type Error = RonAssetLoadError;
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
        Ok(A::from_def(def, load_context)?)
    }
}

impl<A> Default for RonAssetLoader<A> {
    fn default() -> Self {
        Self(PhantomData::default())
    }
}

#[derive(Error)]
pub struct MissingAssetError<A: Asset>(AssetId<A>);

impl<A: Asset> MissingAssetError<A> {
    pub fn new(id: AssetId<A>) -> Self {
        Self(id)
    }
}

impl<A: Asset> Debug for MissingAssetError<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MissingAssetError({:?})", self.0)
    }
}

impl<A: Asset> Display for MissingAssetError<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let asset_type = A::type_ident().unwrap_or_else(|| A::type_path());
        write!(f, "missing {asset_type}: \"{}\"", self.0)
    }
}

#[derive(Error, Debug)]
pub enum RonAssetLoadError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Spanned(#[from] SpannedError),
    #[error("{0}")]
    FromDef(#[from] FromDefError),
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

pub mod implicit_option {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S, T>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        match value {
            Some(v) => v.serialize(serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        Ok(Some(T::deserialize(deserializer)?))
    }
}

impl<A: Asset> PartialEq for AssetRef<A> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.handle.id() == other.handle.id()
    }
}

impl<A: Asset> PartialEq<Handle<A>> for AssetRef<A> {
    fn eq(&self, other: &Handle<A>) -> bool {
        self.handle.id() == other.id()
    }
}

impl<A: Asset> Clone for AssetRef<A> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            handle: self.handle.clone(),
        }
    }
}

pub trait AssetsExt<A: Asset> {
    fn require(&self, id: AssetId<A>) -> Result<&A>;
    fn require_handle(&self, handle: &Handle<A>) -> Result<&A> {
        self.require(handle.id())
    }

    fn require_mut(&mut self, id: AssetId<A>) -> Result<&mut A>;
    fn require_handle_mut(&mut self, handle: &Handle<A>) -> Result<&mut A> {
        self.require_mut(handle.id())
    }
}

impl<A: Asset> AssetsExt<A> for Assets<A> {
    fn require(&self, id: AssetId<A>) -> Result<&A> {
        Ok(self.get(id).ok_or_else(|| MissingAssetError::new(id))?)
    }

    fn require_mut(&mut self, id: AssetId<A>) -> Result<&mut A> {
        Ok(self.get_mut(id).ok_or_else(|| MissingAssetError::new(id))?)
    }
}

pub mod duration_millis {
    use serde::{Deserialize, Deserializer, Serializer};

    use super::*;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}
