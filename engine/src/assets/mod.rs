use std::marker::PhantomData;

pub use folder::{AssetMap, AssetSet, AssetSetPlugin, FileAsset, LoadState};
pub use spawn::{EntityFolderPlugin, EntityLookupMap};

pub mod animations;
pub mod folder;
mod spawn;
pub mod tile;

type Phantom<L> = PhantomData<fn() -> L>;
