use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::time::Duration;

use bevy::asset::AssetMut;
use bevy::prelude::*;
pub use set::{AssetMap, AssetSetPlugin, LoadState};
use thiserror::Error;

pub mod set;
pub mod spritesheet;

pub type Phantom<L> = PhantomData<fn() -> L>;

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

pub trait AssetsExt<A: Asset> {
    fn require(&self, id: AssetId<A>) -> Result<&A>;
    fn require_handle(&self, handle: &Handle<A>) -> Result<&A> {
        self.require(handle.id())
    }

    fn require_mut<'a>(&'a mut self, id: AssetId<A>) -> Result<AssetMut<'a, A>>;
    fn require_handle_mut<'a>(&'a mut self, handle: &Handle<A>) -> Result<AssetMut<'a, A>> {
        self.require_mut(handle.id())
    }
}

impl<A: Asset> AssetsExt<A> for Assets<A> {
    fn require(&self, id: AssetId<A>) -> Result<&A> {
        Ok(self.get(id).ok_or_else(|| MissingAssetError::new(id))?)
    }

    fn require_mut<'a>(&'a mut self, id: AssetId<A>) -> Result<AssetMut<'a, A>> {
        Ok(self.get_mut(id).ok_or_else(|| MissingAssetError::new(id))?)
    }
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
