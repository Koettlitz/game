use engine::{
    asset::{
        AssetMap, AssetRef, AssetSetPlugin, FromDef, FromDefError, LoadState, RonAssetPlugin,
        animations::sprite::SpriteAnimationAsset, one_or_many, overworld::tile::TileSpriteSheet,
    },
    overworld::tile::{GridView, Neighbor, Passability},
    progress::{Progress, ProgressPanel},
};
use macros::{FromDef, asset_set};
use std::{collections::HashMap, fmt::Debug, slice};

use bevy::{asset::LoadContext, prelude::*};
use serde::{Deserialize, Serialize};

pub type TileKindMap = AssetMap<Tile, TileKindAsset>;

pub struct GroundTileAssetsPlugin;
impl Plugin for GroundTileAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RonAssetPlugin::<TileKindAsset>::default(),
            AssetSetPlugin::<Tile, TileKindAsset>::default(),
        ))
        .add_systems(Startup, init_progress)
        .add_systems(OnEnter(LoadState::<Tile>::finished()), derive_layouts);
    }
}

#[derive(Component)]
struct DeriveLayoutsProgress;

fn init_progress(mut commands: Commands) {
    commands.spawn((
        Progress::new(0, 1),
        ProgressPanel::new("tile spritesheet layouts".to_string()),
        DeriveLayoutsProgress,
    ));
}

fn derive_layouts(
    mut tile_kinds: ResMut<Assets<TileKindAsset>>,
    images: Res<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut progress: Single<&mut Progress, With<DeriveLayoutsProgress>>,
) -> Result<()> {
    for spritesheet in tile_kinds
        .iter_mut()
        .map(|(_, tile_kind)| &mut tile_kind.visuals.spritesheet)
    {
        spritesheet.derive_layout(&images, &mut layouts)?;
    }
    progress.add(1);
    Ok(())
}

#[derive(Asset, TypePath, Debug)]
#[asset_set(base_path = "tiles", progress_name = "tiles")]
pub struct TileKindAsset {
    pub _passability: Passability,
    pub visuals: GroundTileVisuals,
}

#[derive(Serialize, Deserialize)]
pub struct TileKindDef {
    pub passability: Passability,
    pub visuals: GroundTileVisualsDef,
}

impl FromDef for TileKindAsset {
    type Def = TileKindDef;
    type Error = FromDefError;

    fn from_def(def: Self::Def, ctx: &mut bevy::asset::LoadContext) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        if !def.visuals.config.keys().any(|k| k.is_default()) {
            let path = ctx.path().path().to_string_lossy();
            Err(FromDefError::InvalidDef(format!(
                "missing default visuals for tile {path}"
            )))
        } else {
            Ok(Self {
                _passability: def.passability,
                visuals: GroundTileVisuals::from_def(def.visuals, ctx)?,
            })
        }
    }
}

#[derive(TypePath, Component, Debug)]
pub struct GroundTileVisuals {
    pub spritesheet: TileSpriteSheet,
    pub config: Vec<(AdjacentRequirements, GroundTileVisualLayers)>,
}

#[derive(Serialize, Deserialize)]
pub struct GroundTileVisualsDef {
    pub spritesheet: String,
    pub config: HashMap<AdjacentRequirementsConfig, GroundTileVisualLayersDef>,
}

impl FromDef for GroundTileVisuals {
    type Def = GroundTileVisualsDef;
    type Error = FromDefError;

    fn from_def(
        def: Self::Def,
        load_context: &mut LoadContext,
    ) -> std::result::Result<Self, Self::Error> {
        let mut parsed_visuals = Vec::new();
        for (req, visuals) in def.config {
            let req = AdjacentRequirements::from_def(req, load_context)?;
            let visuals = GroundTileVisualLayers::from_def(visuals, load_context)?;
            parsed_visuals.push((req, visuals));
        }
        parsed_visuals.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(Self {
            spritesheet: TileSpriteSheet::from_def(def.spritesheet, load_context)?,
            config: parsed_visuals,
        })
    }
}

impl GroundTileVisuals {
    pub fn default_config(&self) -> &GroundTileVisualLayers {
        &self.config.last().expect("empty config").1
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct AdjacentRequirements {
    pub top_left: AdjacentRequirement,
    pub top: AdjacentRequirement,
    pub top_right: AdjacentRequirement,
    pub left: AdjacentRequirement,
    pub right: AdjacentRequirement,
    pub bottom_left: AdjacentRequirement,
    pub bottom: AdjacentRequirement,
    pub bottom_right: AdjacentRequirement,
}

impl FromDef for AdjacentRequirements {
    type Def = AdjacentRequirementsConfig;
    type Error = FromDefError;

    fn from_def(
        config: Self::Def,
        load_context: &mut LoadContext,
    ) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            top_left: AdjacentRequirement::from_def(config.top_left, load_context)?,
            top: AdjacentRequirement::from_def(config.top, load_context)?,
            top_right: AdjacentRequirement::from_def(config.top_right, load_context)?,
            left: AdjacentRequirement::from_def(config.left, load_context)?,
            right: AdjacentRequirement::from_def(config.right, load_context)?,
            bottom_left: AdjacentRequirement::from_def(config.bottom_left, load_context)?,
            bottom: AdjacentRequirement::from_def(config.bottom, load_context)?,
            bottom_right: AdjacentRequirement::from_def(config.bottom_right, load_context)?,
        })
    }
}

impl AdjacentRequirements {
    pub fn matches(&self, surroundings: &GridView<AssetRef<TileKindAsset>>) -> bool {
        let center = surroundings.center();
        let result = self
            .all()
            .iter()
            .zip(surroundings.iter_exclusive())
            .all(|(req, neighbor)| req.matches(center.id(), neighbor.map(|l| l.id())));
        result
    }

    pub fn all(&self) -> [&AdjacentRequirement; 8] {
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

    fn prio(&self) -> usize {
        self.all().into_iter().map(|n| n.prio()).sum()
    }
}

impl Ord for AdjacentRequirements {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.prio().cmp(&self.prio())
    }
}

impl PartialOrd for AdjacentRequirements {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
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

#[derive(FromDef, Default, Clone, Debug)]
#[def_type(AdjacentRequirementConfig)]
pub enum AdjacentRequirement {
    #[default]
    Any,
    Same,
    Other,
    Either(Vec<String>),
}

impl AdjacentRequirement {
    fn matches(&self, identity: &str, other: Option<&str>) -> bool {
        match self {
            Self::Any => true,
            Self::Same => other.map(|n| n == identity).unwrap_or(false),
            Self::Other => other.map(|n| n != identity).unwrap_or(false),
            Self::Either(e) => other.map(|o| e.iter().any(|e| e == o)).unwrap_or(false),
        }
    }

    fn prio(&self) -> usize {
        match self {
            Self::Any => 1,
            Self::Same | Self::Other => 10,
            Self::Either(_) => 100,
        }
    }
}

impl PartialEq for AdjacentRequirement {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Any => matches!(other, Self::Any),
            Self::Same => matches!(other, Self::Same),
            Self::Other => matches!(other, Self::Other),
            Self::Either(_) => matches!(other, Self::Either(_)),
        }
    }
}
impl Eq for AdjacentRequirement {}

impl Ord for AdjacentRequirement {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.prio().cmp(&other.prio())
    }
}

impl PartialOrd for AdjacentRequirement {
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

#[derive(Debug)]
pub struct GroundTileVisualLayers {
    below: Vec<GroundTileVisual>,
    base: GroundTileVisual,
    above: Vec<GroundTileVisual>,
}

impl FromDef for GroundTileVisualLayers {
    type Def = GroundTileVisualLayersDef;
    type Error = FromDefError;
    fn from_def(
        config: Self::Def,
        load_context: &mut LoadContext,
    ) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            below: config
                .below
                .into_iter()
                .map(|c| GroundTileVisual::from_def(c, load_context))
                .filter_map(|result| result.inspect_err(|e| bevy::log::error!("{e}")).ok())
                .collect(),
            base: GroundTileVisual::from_def(config.base, load_context)?,
            above: config
                .above
                .into_iter()
                .map(|c| GroundTileVisual::from_def(c, load_context))
                .filter_map(|result| result.inspect_err(|e| bevy::log::error!("{e}")).ok())
                .collect(),
        })
    }
}

impl GroundTileVisualLayers {
    pub fn base(&self) -> &GroundTileVisual {
        &self.base
    }

    pub fn iter<'a>(&'a self) -> LayerIterator<'a> {
        LayerIterator::from(self)
    }

    pub fn _iter_mut<'a>(&'a mut self) -> _LayerIteratorMut<'a> {
        _LayerIteratorMut::from(self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VisualLayer {
    Below,
    Base,
    Above,
}

impl VisualLayer {
    fn z(&self) -> f32 {
        match self {
            Self::Below => 1.0,
            Self::Base => 10.0,
            Self::Above => 100.0,
        }
    }
}

struct LayerCursor {
    current_layer: VisualLayer,
    current_idx: usize,
}

enum LayerItem {
    Below,
    Base,
    Above,
}

impl LayerCursor {
    fn next(&mut self, below_len: usize, above_len: usize) -> Option<(f32, LayerItem)> {
        if matches!(self.current_layer, VisualLayer::Below) {
            if self.current_idx < below_len {
                let z = VisualLayer::Below.z() + self.current_idx as f32;
                self.current_idx += 1;
                Some((z, LayerItem::Below))
            } else {
                self.current_layer = VisualLayer::Above;
                self.current_idx = 0;
                Some((VisualLayer::Base.z(), LayerItem::Base))
            }
        } else if self.current_idx < above_len {
            let z = VisualLayer::Above.z() + self.current_idx as f32;
            self.current_idx += 1;
            Some((z, LayerItem::Above))
        } else {
            None
        }
    }
}

pub struct LayerIterator<'a> {
    below: slice::Iter<'a, GroundTileVisual>,
    base: Option<&'a GroundTileVisual>,
    above: slice::Iter<'a, GroundTileVisual>,
    cursor: LayerCursor,
}

impl<'a> From<&'a GroundTileVisualLayers> for LayerIterator<'a> {
    fn from(value: &'a GroundTileVisualLayers) -> Self {
        Self {
            below: value.below.iter(),
            base: Some(&value.base),
            above: value.above.iter(),
            cursor: LayerCursor {
                current_layer: VisualLayer::Below,
                current_idx: 0,
            },
        }
    }
}

impl<'a> Iterator for LayerIterator<'a> {
    type Item = (f32, &'a GroundTileVisual);
    fn next(&mut self) -> Option<Self::Item> {
        let (z, layer_item) = self.cursor.next(self.below.len(), self.above.len())?;
        match layer_item {
            LayerItem::Below => self.below.next().map(|v| (z, v)),
            LayerItem::Base => self.base.take().map(|v| (z, v)),
            LayerItem::Above => self.above.next().map(|v| (z, v)),
        }
    }
}

pub struct _LayerIteratorMut<'a> {
    below: slice::IterMut<'a, GroundTileVisual>,
    base: Option<&'a mut GroundTileVisual>,
    above: slice::IterMut<'a, GroundTileVisual>,
    cursor: LayerCursor,
}

impl<'a> From<&'a mut GroundTileVisualLayers> for _LayerIteratorMut<'a> {
    fn from(value: &'a mut GroundTileVisualLayers) -> Self {
        Self {
            below: value.below.iter_mut(),
            base: Some(&mut value.base),
            above: value.above.iter_mut(),
            cursor: LayerCursor {
                current_layer: VisualLayer::Below,
                current_idx: 0,
            },
        }
    }
}

impl<'a> Iterator for _LayerIteratorMut<'a> {
    type Item = (f32, &'a mut GroundTileVisual);

    fn next(&mut self) -> Option<Self::Item> {
        let (z, layer_item) = self.cursor.next(self.below.len(), self.above.len())?;
        match layer_item {
            LayerItem::Below => self.below.next().map(|v| (z, v)),
            LayerItem::Base => self.base.take().map(|v| (z, v)),
            LayerItem::Above => self.above.next().map(|v| (z, v)),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GroundTileVisualLayersDef {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub below: Vec<GroundTileVisualDef>,
    pub base: GroundTileVisualDef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub above: Vec<GroundTileVisualDef>,
}

impl From<GroundTileVisualDef> for GroundTileVisualLayersDef {
    fn from(value: GroundTileVisualDef) -> Self {
        Self {
            below: Vec::new(),
            base: value,
            above: Vec::new(),
        }
    }
}

impl Debug for GroundTileVisualLayersDef {
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

#[derive(FromDef, Debug)]
pub enum GroundTileVisual {
    Static(usize),
    Animated(AssetRef<SpriteAnimationAsset>),
    Neighbor(Neighbor),
}

impl Debug for GroundTileVisualDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static(idx) => {
                write!(f, "GroundTileVisualDef::Static({idx})")
            }
            Self::Animated(animation) => {
                write!(f, "GroundTileVisualDef::Animated({animation})")
            }
            Self::Neighbor(neighbor) => write!(f, "GroundTileVisualDef::Neighbor({neighbor:?})"),
        }
    }
}
