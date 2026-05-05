use std::marker::PhantomData;

use bevy::{platform::collections::HashMap, prelude::*};

use crate::asset::{
    AssetRef, LoadState, Phantom,
    folder::{AsAssetPath, AssetMap, AssetSetPlugin, FillAssetMap, ProgressName, Set},
};

pub trait Spawn: Asset {
    type B: Bundle;
    fn spawn(&self, asset_ref: AssetRef<Self>) -> Self::B
    where
        Self: Sized;
}

pub struct EntityFolderPlugin<S, A, B>(Phantom<S>, Phantom<A>, Phantom<B>);
impl<S, A, B> Default for EntityFolderPlugin<S, A, B> {
    fn default() -> Self {
        Self(
            PhantomData::default(),
            PhantomData::default(),
            PhantomData::default(),
        )
    }
}

impl<S, A, B> Plugin for EntityFolderPlugin<S, A, B>
where
    S: AsAssetPath + Set + ProgressName + 'static,
    A: Asset + Spawn<B = B>,
    B: Bundle + 'static,
{
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins(AssetSetPlugin::<S, A>::default())
            .init_resource::<EntityLookupMap<B>>()
            .add_systems(
                Update,
                spawn_entites::<S, A, B>
                    .run_if(in_state(LoadState::<S>::loading()))
                    .in_set(SpawnEntities)
                    .after(FillAssetMap),
            )
            .add_systems(OnEnter(LoadState::<S>::finished()), cleanup::<S, A>);
    }
}

#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SpawnEntities;

fn spawn_entites<S, A, B>(
    mut commands: Commands,
    assets: ResMut<Assets<A>>,
    mut asset_map: ResMut<AssetMap<S, A>>,
    mut entity_map: ResMut<EntityLookupMap<B>>,
) where
    S: AsAssetPath + 'static,
    A: Asset + Spawn<B = B>,
    B: Bundle + 'static,
{
    for (id, handle) in asset_map.0.drain() {
        let Some(asset) = assets.get(handle.id()) else {
            panic!("missing asset {id} even though, it was inside the asset map")
        };
        let entity = commands
            .spawn(asset.spawn(AssetRef::new(id.to_string(), handle)))
            .id();
        entity_map.0.insert(id, entity);
    }
}

fn cleanup<S: AsAssetPath + 'static, A: Asset>(mut commands: Commands) {
    commands.remove_resource::<AssetMap<S, A>>();
}

#[derive(Resource)]
pub struct EntityLookupMap<B>(pub HashMap<String, Entity>, Phantom<B>);
impl<B> Default for EntityLookupMap<B> {
    fn default() -> Self {
        Self(HashMap::default(), PhantomData::default())
    }
}
