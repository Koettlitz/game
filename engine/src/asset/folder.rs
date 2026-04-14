use std::{borrow::Cow, hash::Hash, marker::PhantomData, str::FromStr};
use strum::IntoEnumIterator;
use thiserror::Error;

use bevy::{
    asset::{AssetPath, RecursiveDependencyLoadState},
    prelude::*,
};
use std::collections::HashMap;
use std::collections::hash_map::Iter;

use crate::{
    asset::{AssetResolver, FromDefError, Phantom},
    progress::{Progress, ProgressPanel},
};

pub struct AssetSetResolver<T>(Phantom<T>);
impl<T> Default for AssetSetResolver<T> {
    fn default() -> Self {
        Self(PhantomData::default())
    }
}

pub trait Set {
    type Iter: Iterator<Item = Self>;

    fn iter() -> Self::Iter;
}

impl<T> Set for T
where
    T: IntoEnumIterator,
{
    type Iter = <Self as IntoEnumIterator>::Iterator;

    fn iter() -> Self::Iter {
        <Self as IntoEnumIterator>::iter()
    }
}

impl<A> AssetResolver for AssetSetResolver<A>
where
    A: AsAssetPath + FromStr,
    FromDefError: From<<A as FromStr>::Err>,
{
    fn resolve(asset_id: &str) -> Result<AssetPath<'static>, FromDefError> {
        Ok(A::from_str(asset_id)?.as_asset_path())
    }
}

pub trait AsAssetPath {
    fn as_asset_path(&self) -> AssetPath<'static>;
}

/// TODO: Where does this belong?
pub trait ProgressName {
    fn name<'a>() -> &'a str;
}

#[derive(Error, Debug)]
#[error("invalid asset link: \"{0}\"")]
pub struct InvalidAssetLinkError(pub String);

pub struct AssetSetPlugin<S, A> {
    load_on_startup: bool,
    show_progress: bool,
    _set_marker: Phantom<S>,
    _asset_marker: Phantom<A>,
}

impl<S, A> Default for AssetSetPlugin<S, A> {
    fn default() -> Self {
        Self {
            show_progress: true,
            load_on_startup: true,
            _set_marker: PhantomData::default(),
            _asset_marker: PhantomData::default(),
        }
    }
}

impl<S, A> Plugin for AssetSetPlugin<S, A>
where
    S: AsAssetPath + Set + ProgressName + 'static,
    A: Asset,
{
    fn build(&self, app: &mut bevy::app::App) {
        app.init_state::<LoadState<S>>()
            .init_resource::<AssetMap<S, A>>()
            .add_systems(
                OnEnter(LoadState::<S>::loading()),
                load_asset_folder::<S, A>,
            )
            .add_systems(
                Update,
                fill_asset_map::<S, A>
                    .run_if(in_state(LoadState::<S>::loading()))
                    .in_set(FillAssetMap),
            )
            .add_systems(
                PostUpdate,
                check_handles::<S, A>.run_if(in_state(LoadState::<S>::loading())),
            )
            .add_systems(
                OnEnter(LoadState::<S>::handles_present()),
                cleanup_loading_folder::<S, A>,
            )
            .add_systems(
                PostUpdate,
                check_finished::<S, A>.run_if(in_state(LoadState::<S>::handles_present())),
            )
            .add_systems(OnEnter(LoadState::<S>::finished()), cleanup_handles::<S, A>);

        if self.load_on_startup {
            app.add_systems(Startup, trigger_loading::<S>);
        }

        if self.show_progress {
            app.add_systems(
                Update,
                init_progress::<S, A>
                    .before(fill_asset_map::<S, A>)
                    .run_if(resource_added::<LoadingFolder<S, A>>)
                    .run_if(in_state(LoadState::<S>::loading())),
            );
        }
    }
}

#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FillAssetMap;

fn trigger_loading<S>(mut next_state: ResMut<NextState<LoadState<S>>>) {
    next_state.set(LoadState::loading());
}

fn load_asset_folder<S, A>(asset_server: Res<AssetServer>, mut commands: Commands)
where
    S: AsAssetPath + Set + 'static,
    A: Asset,
{
    let handles: Vec<Handle<A>> = S::iter()
        .map(|p| asset_server.load(p.as_asset_path()))
        .collect();
    commands.insert_resource(LoadingFolder::<S, A>::new(handles.clone()));
    commands.insert_resource(Handles::<S, A>::new(handles));
}

fn init_progress<S, A>(loading_folder: Res<LoadingFolder<S, A>>, mut commands: Commands)
where
    S: AsAssetPath + ProgressName + 'static,
    A: Asset,
{
    commands.spawn((
        Progress::new(0, loading_folder.0.len()),
        ProgressPanel::new(S::name().to_string()),
        FolderProgress::<S>::default(),
    ));
}

#[derive(Component)]
#[require(Progress)]
pub struct FolderProgress<F>(Phantom<F>);
impl<S> Default for FolderProgress<S> {
    fn default() -> Self {
        Self(PhantomData::default())
    }
}

fn fill_asset_map<S, A>(
    asset_server: Res<AssetServer>,
    loading_folder: Option<ResMut<LoadingFolder<S, A>>>,
    assets: Res<Assets<A>>,
    mut asset_map: ResMut<AssetMap<S, A>>,
    mut progress: Query<&mut Progress, With<FolderProgress<S>>>,
) where
    S: AsAssetPath + ProgressName + 'static,
    A: Asset,
{
    let Some(mut loading_folder) = loading_folder else {
        return;
    };
    loading_folder.0.retain(|handle: &Handle<A>| {
        if assets.get(handle.id()).is_some() {
            let id = {
                let asset_path = handle.path().expect("missing asset path");
                let id = asset_path
                    .path()
                    .file_stem()
                    .unwrap_or_else(|| panic!("missing file stem in asset path {}", asset_path));
                id.to_string_lossy()
                    .split('.')
                    .next()
                    .expect("split string was emtpy")
                    .to_string()
            };
            asset_map.0.insert(id, handle.clone());
            for mut progress in progress.iter_mut() {
                progress.add(1);
            }
            false
        } else {
            if let Some(bevy::asset::LoadState::Failed(e)) =
                asset_server.get_load_state(handle.id())
            {
                error!(
                    "failed to load asset {} from folder {} - {e}",
                    handle
                        .path()
                        .map(|p| Cow::Owned(format!("{p}")))
                        .unwrap_or_else(|| Cow::Borrowed("with no path")),
                    S::name()
                );
                for mut progress in progress.iter_mut() {
                    progress.add(1);
                }
                false
            } else {
                true
            }
        }
    });
}

fn check_handles<S, A>(
    loading_folder: Option<Res<LoadingFolder<S, A>>>,
    mut next_state: ResMut<NextState<LoadState<S>>>,
) where
    S: AsAssetPath,
    A: Asset,
{
    let Some(loading_folder) = loading_folder else {
        return;
    };
    if loading_folder.0.is_empty() {
        next_state.set(LoadState::handles_present());
    }
}

fn check_finished<S: 'static, A: Asset>(
    handles: ResMut<Handles<S, A>>,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<LoadState<S>>>,
) {
    let all_loaded = handles
        .0
        .iter()
        .map(|handle| asset_server.recursive_dependency_load_state(handle.id()))
        .all(|state| matches!(state, RecursiveDependencyLoadState::Loaded));
    if all_loaded {
        next_state.set(LoadState::<S>::finished());
    }
}

fn cleanup_loading_folder<S: AsAssetPath + 'static, A: Asset>(mut commands: Commands) {
    commands.remove_resource::<LoadingFolder<S, A>>();
}

fn cleanup_handles<S: 'static, A: Asset>(mut commands: Commands) {
    commands.remove_resource::<Handles<S, A>>();
}

#[derive(Resource)]
struct LoadingFolder<S: AsAssetPath, A: Asset>(Vec<Handle<A>>, Phantom<S>);
impl<S: AsAssetPath, A: Asset> LoadingFolder<S, A> {
    fn new(handles: impl IntoIterator<Item = Handle<A>>) -> Self {
        Self(handles.into_iter().collect(), PhantomData::default())
    }
}

#[derive(Resource)]
pub struct AssetMap<S: AsAssetPath, A: Asset>(pub HashMap<String, Handle<A>>, Phantom<S>);
impl<S: AsAssetPath, A: Asset> Default for AssetMap<S, A> {
    fn default() -> Self {
        Self(HashMap::new(), PhantomData::default())
    }
}
impl<S: AsAssetPath, A: Asset> AssetMap<S, A> {
    pub fn get(&self, id: &str) -> Option<&Handle<A>> {
        self.0.get(id)
    }

    pub fn iter(&self) -> Iter<'_, String, Handle<A>> {
        self.0.iter()
    }
}

#[derive(Resource)]
struct Handles<S, A: Asset>(Vec<Handle<A>>, Phantom<S>);
impl<S, A: Asset> Handles<S, A> {
    fn new(handles: impl IntoIterator<Item = Handle<A>>) -> Self {
        Self(handles.into_iter().collect(), PhantomData::default())
    }
}

#[derive(States, Copy)]
pub enum LoadState<S: 'static> {
    Idle(Phantom<S>),
    Loading(Phantom<S>),
    HandlesPresent(Phantom<S>),
    Finished(Phantom<S>),
}

impl<S: 'static> Default for LoadState<S> {
    fn default() -> Self {
        Self::idle()
    }
}

impl<S> LoadState<S> {
    pub fn idle() -> Self {
        Self::Idle(PhantomData::default())
    }

    pub fn loading() -> Self {
        Self::Loading(PhantomData::default())
    }

    pub fn handles_present() -> Self {
        Self::HandlesPresent(PhantomData::default())
    }

    pub fn finished() -> Self {
        Self::Finished(PhantomData::default())
    }

    pub fn is_finished(&self) -> bool {
        *self == Self::finished()
    }
}

impl<S> Clone for LoadState<S> {
    fn clone(&self) -> Self {
        match self {
            Self::Idle(_) => Self::Idle(PhantomData::default()),
            Self::Loading(_) => Self::Loading(PhantomData::default()),
            Self::HandlesPresent(_) => Self::HandlesPresent(PhantomData::default()),
            Self::Finished(_) => Self::Finished(PhantomData::default()),
        }
    }
}
impl<S> PartialEq for LoadState<S> {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}
impl<S> Eq for LoadState<S> {}
impl<S> Hash for LoadState<S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
    }
}
impl<S> std::fmt::Debug for LoadState<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::mem::discriminant(self).fmt(f)
    }
}
