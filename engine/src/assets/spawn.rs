use std::marker::PhantomData;

use bevy::{platform::collections::HashMap, prelude::*};

use crate::assets::{
    AssetResolver, LoadState, Phantom,
    folder::{AssetMap, AssetSet, AssetSetPlugin, FillAssetMap},
};

pub trait Spawn: Asset {
    type B: Bundle;
    fn spawn(&self, handle: Handle<Self>) -> Self::B
    where
        Self: Sized;
}

pub struct EntityFolderPlugin<F, B>(Phantom<F>, Phantom<B>);
impl<F, B> Default for EntityFolderPlugin<F, B> {
    fn default() -> Self {
        Self(PhantomData::default(), PhantomData::default())
    }
}

impl<S, B> Plugin for EntityFolderPlugin<S, B>
where
    S: AssetSet + 'static,
    B: Bundle + 'static,
    <S::Resolver as AssetResolver>::Asset: Spawn<B = B>,
{
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins(AssetSetPlugin::<S>::default())
            .init_resource::<EntityLookupMap<B>>()
            .add_systems(
                Update,
                spawn_entites::<S, B>
                    .run_if(in_state(LoadState::<S>::loading()))
                    .in_set(SpawnEntities)
                    .after(FillAssetMap),
            )
            .add_systems(OnEnter(LoadState::<S>::finished()), cleanup::<S, B>);
    }
}

#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SpawnEntities;

fn spawn_entites<S, B>(
    mut commands: Commands,
    assets: ResMut<Assets<<S::Resolver as AssetResolver>::Asset>>,
    mut asset_map: ResMut<AssetMap<S>>,
    mut entity_map: ResMut<EntityLookupMap<B>>,
) where
    S: AssetSet + 'static,
    B: Bundle + 'static,
    <S::Resolver as AssetResolver>::Asset: Spawn<B = B>,
{
    for (id, handle) in asset_map.0.drain() {
        let Some(asset) = assets.get(handle.id()) else {
            panic!("missing asset {id} even though, it was inside the asset map")
        };
        let entity = commands.spawn(asset.spawn(handle)).id();
        entity_map.0.insert(id, entity);
    }
}

fn cleanup<S: AssetSet + 'static, B>(mut commands: Commands) {
    commands.remove_resource::<AssetMap<S>>();
}

#[derive(Resource)]
pub struct EntityLookupMap<B>(pub HashMap<String, Entity>, Phantom<B>);
impl<B> Default for EntityLookupMap<B> {
    fn default() -> Self {
        Self(HashMap::default(), PhantomData::default())
    }
}
