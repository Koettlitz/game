use engine::{
    assets::{
        AssetMap, AssetSetPlugin, LoadState, one_or_many,
        sprite_sheet::{SpriteSheet, SpriteSheetMap},
    },
    overworld::tile::{Neighbor, Passability},
};
use macros::{FromDef, asset_set};
use ron::de::SpannedError;
use std::{collections::HashMap, fmt::Debug, io};
use thiserror::Error;

use bevy::{asset::AssetLoader, prelude::*};
use engine::assets::overworld::tile;
use serde::{Deserialize, Serialize, Serializer};

pub struct GroundTileAssetsPlugin;
impl Plugin for GroundTileAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TileSpriteSheetMap>()
            .init_asset::<GroundTileAsset>()
            .init_asset_loader::<GroundTileAssetLoader>()
            .add_plugins((
                AssetSetPlugin::<TileSpriteAssets>::default(),
                AssetSetPlugin::<GroundTileAsset>::default(),
            ))
            .add_systems(
                OnEnter(LoadState::<TileSpriteAssets>::finished()),
                tile::derive_texture_atlas_layouts::<TileSpriteAssets, TileSpriteSheetMap>,
            )
            .add_systems(
                OnEnter(LoadState::<TileSpriteAssets>::finished()),
                cleanup.after(
                    tile::derive_texture_atlas_layouts::<TileSpriteAssets, TileSpriteSheetMap>,
                ),
            );
    }
}

#[asset_set(
    name = "tile_sprite_folder",
    base_path = "tiles/spritesheets",
    extension = "png",
    asset_registry(crate::asset_registry),
    asset_type(Image)
)]
pub struct TileSpriteAssets;

#[derive(Resource, Default)]
pub struct TileSpriteSheetMap(pub HashMap<String, SpriteSheet>);
impl SpriteSheetMap for TileSpriteSheetMap {
    fn insert(&mut self, id: String, value: SpriteSheet) {
        self.0.insert(id, value);
    }

    fn get(&self, id: &str) -> Option<&SpriteSheet> {
        self.0.get(id)
    }

    fn remove(&mut self, id: &str) -> Option<SpriteSheet> {
        self.0.remove(id)
    }
}

fn cleanup(mut commands: Commands) {
    commands.remove_resource::<AssetMap<TileSpriteAssets>>();
}

#[derive(TypePath, Default)]
struct GroundTileAssetLoader;
impl AssetLoader for GroundTileAssetLoader {
    type Asset = GroundTileAsset;
    type Error = GroundTileAssetLoadError;
    type Settings = ();
    async fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _: &Self::Settings,
        load_context: &mut bevy::asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let asset: GroundTileAsset = ron::de::from_bytes(&mut bytes)?;
        if !asset
            .visuals
            .contains_key(&AdjacentRequirementsConfig::default())
        {
            return Err(GroundTileAssetLoadError::MissingDefaultVisuals(
                load_context.path().path().to_string_lossy().into_owned(),
            ));
        }
        Ok(asset)
    }

    fn extensions(&self) -> &[&str] {
        &["tile.ron"]
    }
}

#[derive(Error, Debug)]
enum GroundTileAssetLoadError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Ron(#[from] SpannedError),
    #[error("missing default visuals for tile {0}")]
    MissingDefaultVisuals(String),
}

fn serialize_some_unwrapped<T, S>(opt: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: Serializer,
{
    if let Some(value) = opt {
        value.serialize(serializer)
    } else {
        serializer.serialize_none()
    }
}

#[derive(FromDef, Debug, Serialize, Deserialize, Asset, TypePath)]
#[asset_set(
    base_path = "editor://tiles/config",
    extension = "tile.ron",
    asset_registry(crate::asset_registry),
    asset_type(Self)
)]
pub struct GroundTileAsset {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_some_unwrapped"
    )]
    pub id: Option<String>,
    pub passability: Passability,
    pub visuals: HashMap<AdjacentRequirementsConfig, GroundTileVisualLayersConfig>,
}

#[derive(Serialize, Deserialize)]
pub struct GroundTileVisualLayersConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub below: Vec<TileKindVisualConfig>,
    pub base: TileKindVisualConfig,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub above: Vec<TileKindVisualConfig>,
}

impl From<TileKindVisualConfig> for GroundTileVisualLayersConfig {
    fn from(value: TileKindVisualConfig) -> Self {
        Self {
            below: Vec::new(),
            base: value,
            above: Vec::new(),
        }
    }
}

impl Debug for GroundTileVisualLayersConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.below.is_empty() {
            if self.above.is_empty() {
                write!(
                    f,
                    "GroundTileVisualLayersConfig {{ base: {:?} }}",
                    self.base
                )
            } else {
                write!(
                    f,
                    "GroundTileVisualLayersConfig {{ base: {:?}, above: {:?}",
                    self.base, self.above
                )
            }
        } else {
            if self.above.is_empty() {
                write!(
                    f,
                    "GroundTileVisualLayersConfig {{ below: {:?}, base: {:?}",
                    self.below, self.base
                )
            } else {
                write!(
                    f,
                    "GroundTileVisualLayersConfig {{ below: {:?}, base: {:?}, above: {:?}",
                    self.below, self.base, self.above
                )
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TileKindVisualConfig {
    Static(usize),
    Animated { animation_id: String },
    Neighbor(Neighbor),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub struct AdjacentRequirementsConfig {
    #[serde(default, skip_serializing_if = "AdjacentRequirementConfig::is_any")]
    pub top_left: AdjacentRequirementConfig,
    #[serde(default, skip_serializing_if = "AdjacentRequirementConfig::is_any")]
    pub top: AdjacentRequirementConfig,
    #[serde(default, skip_serializing_if = "AdjacentRequirementConfig::is_any")]
    pub top_right: AdjacentRequirementConfig,
    #[serde(default, skip_serializing_if = "AdjacentRequirementConfig::is_any")]
    pub left: AdjacentRequirementConfig,
    #[serde(default, skip_serializing_if = "AdjacentRequirementConfig::is_any")]
    pub right: AdjacentRequirementConfig,
    #[serde(default, skip_serializing_if = "AdjacentRequirementConfig::is_any")]
    pub bottom_left: AdjacentRequirementConfig,
    #[serde(default, skip_serializing_if = "AdjacentRequirementConfig::is_any")]
    pub bottom: AdjacentRequirementConfig,
    #[serde(default, skip_serializing_if = "AdjacentRequirementConfig::is_any")]
    pub bottom_right: AdjacentRequirementConfig,
}

impl AdjacentRequirementsConfig {
    fn all(&self) -> [&AdjacentRequirementConfig; 8] {
        [
            &self.top_left,
            &self.top,
            &self.top_right,
            &self.left,
            &self.right,
            &self.bottom_left,
            &self.bottom,
            &self.bottom_right,
        ]
    }

    pub fn is_default(&self) -> bool {
        self.all().into_iter().all(|req| req.is_any())
    }

    fn prio(&self) -> usize {
        self.all().into_iter().map(|n| n.prio()).sum()
    }
}

impl Ord for AdjacentRequirementsConfig {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.prio().cmp(&other.prio())
    }
}

impl PartialOrd for AdjacentRequirementsConfig {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum AdjacentRequirementConfig {
    #[default]
    Any,
    Same,
    Other,
    #[serde(with = "one_or_many")]
    Either(Vec<String>),
}

impl AdjacentRequirementConfig {
    fn prio(&self) -> usize {
        match self {
            Self::Any => 1,
            Self::Same | Self::Other { .. } => 10,
            Self::Either(_) => 100,
        }
    }
}

impl Ord for AdjacentRequirementConfig {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.prio().cmp(&other.prio())
    }
}

impl PartialOrd for AdjacentRequirementConfig {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl AdjacentRequirementConfig {
    fn is_any(&self) -> bool {
        matches!(self, Self::Any)
    }
}
