use std::marker::PhantomData;

use bevy::{platform::collections::HashMap, prelude::*};

use crate::assets::{
    LoadState, Phantom,
    folder::{AssetMap, AssetSet, AssetSetPlugin, FillAssetMap},
};

pub struct EntityFolderPlugin<F, B>(Phantom<F>, Phantom<B>);
impl<F, B> Default for EntityFolderPlugin<F, B> {
    fn default() -> Self {
        Self(PhantomData::default(), PhantomData::default())
    }
}

impl<F, B> Plugin for EntityFolderPlugin<F, B>
where
    F: AssetSet,
    F::Asset: Into<B>,
    B: Bundle,
{
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins(AssetSetPlugin::<F>::default())
            .init_resource::<EntityLookupMap<B>>()
            .add_systems(
                Update,
                spawn_entites::<F, B>
                    .run_if(in_state(LoadState::<F>::loading()))
                    .in_set(SpawnEntities)
                    .after(FillAssetMap),
            )
            .add_systems(OnEnter(LoadState::<F>::finished()), cleanup::<F, B>);
    }
}

#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SpawnEntities;

fn spawn_entites<F, B>(
    mut commands: Commands,
    mut assets: ResMut<Assets<F::Asset>>,
    mut asset_map: ResMut<AssetMap<F>>,
    mut entity_map: ResMut<EntityLookupMap<B>>,
) where
    F: AssetSet,
    F::Asset: Into<B>,
    B: Bundle,
{
    for (id, handle) in asset_map.0.drain() {
        let Some(asset) = assets.remove(handle.id()) else {
            panic!("missing asset {id} even though, it was inside the asset map")
        };
        let entity = commands.spawn(asset.into()).id();
        entity_map.0.insert(id, entity);
    }
}

fn cleanup<F: AssetSet, B>(mut commands: Commands) {
    commands.remove_resource::<AssetMap<F>>();
}

#[derive(Resource)]
pub struct EntityLookupMap<B>(pub HashMap<String, Entity>, Phantom<B>);
impl<B> Default for EntityLookupMap<B> {
    fn default() -> Self {
        Self(HashMap::default(), PhantomData::default())
    }
}
