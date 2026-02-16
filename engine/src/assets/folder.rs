use std::{borrow::Cow, hash::Hash, marker::PhantomData};

use bevy::{asset::AssetPath, platform::collections::HashMap, prelude::*};

use crate::{
    assets::Phantom,
    progress::{Progress, ProgressPanel},
};

pub struct AssetSetPlugin<S> {
    show_progress: bool,
    _marker: Phantom<S>,
}
impl<S> Default for AssetSetPlugin<S> {
    fn default() -> Self {
        Self {
            show_progress: true,
            _marker: PhantomData::default(),
        }
    }
}

impl<S: AssetSet> Plugin for AssetSetPlugin<S> {
    fn build(&self, app: &mut bevy::app::App) {
        app.init_state::<LoadState<S>>()
            .init_resource::<AssetMap<S>>()
            .add_systems(OnEnter(LoadState::<S>::loading()), load_asset_folder::<S>)
            .add_systems(
                Update,
                fill_asset_map::<S>
                    .run_if(in_state(LoadState::<S>::loading()))
                    .in_set(FillAssetMap),
            )
            .add_systems(
                PostUpdate,
                check_load_state::<S>.run_if(in_state(LoadState::<S>::loading())),
            )
            .add_systems(OnEnter(LoadState::<S>::finished()), cleanup::<S::Asset>);

        if self.show_progress {
            app.add_systems(
                Update,
                init_progress::<S>
                    .before(fill_asset_map::<S>)
                    .run_if(resource_added::<LoadingFolder<S::Asset>>)
                    .run_if(in_state(LoadState::<S>::loading())),
            );
        }
    }
}

#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FillAssetMap;

pub trait AssetSet: 'static {
    type Asset: FileAsset;
    fn paths() -> Vec<impl Into<AssetPath<'static>>>;
    fn name() -> Option<&'static str> {
        None
    }
}

pub trait FileAsset: Asset {
    fn id(&self) -> Option<&str> {
        None
    }
}

impl FileAsset for Image {}

fn load_asset_folder<S: AssetSet>(asset_server: Res<AssetServer>, mut commands: Commands) {
    let handles = S::paths()
        .into_iter()
        .map(|path| asset_server.load(path))
        .collect();
    commands.insert_resource(LoadingFolder::<S::Asset>::new(handles));
}

fn init_progress<S: AssetSet>(
    loading_folder: Res<LoadingFolder<S::Asset>>,
    mut commands: Commands,
) {
    commands.spawn((
        Progress::new(0, loading_folder.0.len()),
        ProgressPanel::new(set_name::<S>().into_owned()),
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

fn fill_asset_map<S: AssetSet>(
    asset_server: Res<AssetServer>,
    loading_folder: Option<ResMut<LoadingFolder<S::Asset>>>,
    assets: Res<Assets<S::Asset>>,
    mut asset_map: ResMut<AssetMap<S>>,
    mut progress: Query<&mut Progress, With<FolderProgress<S>>>,
) {
    let Some(mut loading_folder) = loading_folder else {
        return;
    };
    loading_folder.0.retain(|handle| {
        let Some(asset) = assets.get(handle.id()) else {
            return if let Some(bevy::asset::LoadState::Failed(e)) =
                asset_server.get_load_state(handle.id())
            {
                error!(
                    "failed to load asset {} from folder {} - {e}",
                    handle
                        .path()
                        .map(|p| Cow::Owned(format!("{p}")))
                        .unwrap_or_else(|| Cow::Borrowed("with no path")),
                    set_name::<S>()
                );
                for mut progress in progress.iter_mut() {
                    progress.add(1);
                }
                false
            } else {
                true
            };
        };
        let id = asset.id().map(|id| id.to_string()).unwrap_or_else(|| {
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
        });
        asset_map.0.insert(id, handle.clone());
        for mut progress in progress.iter_mut() {
            progress.add(1);
        }
        false
    });
}

fn check_load_state<S: AssetSet>(
    loading_folder: Option<Res<LoadingFolder<S::Asset>>>,
    mut next_state: ResMut<NextState<LoadState<S>>>,
) {
    let Some(loading_folder) = loading_folder else {
        return;
    };
    if loading_folder.0.is_empty() {
        next_state.set(LoadState::finished());
    }
}

fn cleanup<A: FileAsset>(mut commands: Commands) {
    commands.remove_resource::<LoadingFolder<A>>();
}

#[derive(Resource)]
struct LoadingFolder<A: Asset>(Vec<Handle<A>>, Phantom<A>);
impl<A: Asset> LoadingFolder<A> {
    fn new(loading_folder: Vec<Handle<A>>) -> Self {
        Self(loading_folder, PhantomData::default())
    }
}

#[derive(Resource)]
pub struct AssetMap<S: AssetSet>(pub HashMap<String, Handle<S::Asset>>);
impl<F: AssetSet> Default for AssetMap<F> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

#[derive(States, Copy)]
pub enum LoadState<S: 'static> {
    Loading(Phantom<S>),
    Finished(Phantom<S>),
}

impl<S: 'static> Default for LoadState<S> {
    fn default() -> Self {
        Self::loading()
    }
}

impl<S> LoadState<S> {
    pub fn loading() -> Self {
        Self::Loading(PhantomData::default())
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
            Self::Loading(_) => Self::Loading(PhantomData::default()),
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

fn set_name<'a, S: AssetSet>() -> Cow<'a, str> {
    S::name().map(|n| Cow::Borrowed(n)).unwrap_or_else(|| {
        Cow::Owned(format!(
            "{}",
            S::paths()
                .into_iter()
                .next()
                .unwrap()
                .into()
                .parent()
                .unwrap()
        ))
    })
}
