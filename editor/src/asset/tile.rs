use engine::{
    asset::{
        AssetMap, AssetRef, AssetSetPlugin, AssetsExt, FromDef, FromDefError, RonAssetPlugin,
        animation::sprite::SpriteAnimationAsset,
        one_or_many,
        overworld::{TILE_LAYER, tile::TileKindSpritesheet},
    },
    overworld::tile::{GridCursor, Neighbor, Passability},
};
use macros::{FromDef, asset_set, asset_spec};
use std::{collections::HashMap, fmt::Debug, slice};

use bevy::{
    asset::{AssetEventSystems, LoadContext},
    prelude::*,
};
use engine::asset::implicit_option;
use serde::{Deserialize, Serialize};

pub type TileKindMap = AssetMap<TileResolverSet, TileKindAsset>;

pub struct TileAssetPlugin;
impl Plugin for TileAssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RonAssetPlugin::<TileKindAsset>::default(),
            RonAssetPlugin::<TileEdgeConfig>::default(),
            AssetSetPlugin::<TileKindAsset>::default(),
        ))
        .add_systems(PreUpdate, derive_layouts.after(AssetEventSystems));
    }
}

fn derive_layouts(
    mut message_reader: MessageReader<AssetEvent<Image>>,
    tile_kind_map: Res<TileKindMap>,
    mut tile_kinds: ResMut<Assets<TileKindAsset>>,
    images: Res<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) -> Result<()> {
    for msg in message_reader.read() {
        let AssetEvent::LoadedWithDependencies { id } = msg else {
            continue;
        };
        for tile_kind in tile_kind_map.0.values() {
            let tile_kind = tile_kinds.require_handle_mut(tile_kind)?;
            if &tile_kind.spritesheet.image().id() != id {
                continue;
            }
            tile_kind.spritesheet.derive_layout(&images, &mut layouts)?;
        }
    }
    Ok(())
}

#[derive(Asset, TypePath, FromDef, Debug)]
#[asset_set(base_path = "tiles", progress_name = "tiles")]
pub struct TileKindAsset {
    pub passability: Passability,

    #[from_def(implicit)]
    pub spritesheet: TileKindSpritesheet,

    #[from_def(implicit)]
    pub edge_config: Handle<TileEdgeConfig>,
}

#[derive(Asset, TypePath, Debug)]
#[asset_spec(base_path = "editor://tiles/edge_config", extension = "edge.ron")]
pub struct TileEdgeConfig {
    pub group: Option<String>,
    pub edge_cases: Vec<(AdjacentRequirements, GroundTileVisualLayers)>,
}

#[derive(Serialize, Deserialize)]
pub struct TileEdgeConfigDef {
    #[serde(default, with = "implicit_option")]
    group: Option<String>,
    edge_cases: HashMap<AdjacentRequirementsDef, GroundTileVisualLayersDef>,
}

impl FromDef for TileEdgeConfig {
    type Def = TileEdgeConfigDef;
    type Error = FromDefError;

    fn from_def(
        def: Self::Def,
        load_context: &mut LoadContext,
    ) -> std::result::Result<Self, Self::Error> {
        if !def.edge_cases.keys().any(|k| k.is_default()) {
            let path = load_context.path().path().to_string_lossy();
            return Err(FromDefError::InvalidDef(format!(
                "missing default visuals for tile {path}"
            )));
        }
        let mut edge_cases = Vec::new();
        for (req, visuals) in def.edge_cases {
            let req = AdjacentRequirements::from_def(req, load_context)?;
            let visuals = GroundTileVisualLayers::from_def(visuals, load_context)?;
            edge_cases.push((req, visuals));
        }
        edge_cases.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(Self {
            group: def.group,
            edge_cases,
        })
    }
}

impl TileEdgeConfig {
    pub fn get_default(&self) -> &GroundTileVisualLayers {
        &self.edge_cases.last().expect("empty config").1
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
    type Def = AdjacentRequirementsDef;
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
    pub fn matches(&self, cursor: &GridCursor<Option<crate::tile::Tile>>) -> bool {
        let center = cursor.get().as_ref().unwrap();
        self.all()
            .iter()
            .zip(cursor.around_exclusive().iter())
            .all(|(req, neighbor)| req.matches(center, neighbor.unwrap_or(&None).as_ref()))
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
pub struct AdjacentRequirementsDef {
    #[serde(default, skip_serializing_if = "AdjacentRequirementDef::is_any")]
    pub top_left: AdjacentRequirementDef,
    #[serde(default, skip_serializing_if = "AdjacentRequirementDef::is_any")]
    pub top: AdjacentRequirementDef,
    #[serde(default, skip_serializing_if = "AdjacentRequirementDef::is_any")]
    pub top_right: AdjacentRequirementDef,
    #[serde(default, skip_serializing_if = "AdjacentRequirementDef::is_any")]
    pub left: AdjacentRequirementDef,
    #[serde(default, skip_serializing_if = "AdjacentRequirementDef::is_any")]
    pub right: AdjacentRequirementDef,
    #[serde(default, skip_serializing_if = "AdjacentRequirementDef::is_any")]
    pub bottom_left: AdjacentRequirementDef,
    #[serde(default, skip_serializing_if = "AdjacentRequirementDef::is_any")]
    pub bottom: AdjacentRequirementDef,
    #[serde(default, skip_serializing_if = "AdjacentRequirementDef::is_any")]
    pub bottom_right: AdjacentRequirementDef,
}

impl AdjacentRequirementsDef {
    fn all(&self) -> [&AdjacentRequirementDef; 8] {
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

impl Ord for AdjacentRequirementsDef {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.prio().cmp(&self.prio())
    }
}

impl PartialOrd for AdjacentRequirementsDef {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(FromDef, Default, Clone, Debug)]
#[def_type(AdjacentRequirementDef)]
pub enum AdjacentRequirement {
    #[default]
    Any,
    Same,
    Other,
    SameGroup,
    OtherGroup,
    Either(Vec<String>),
}

impl AdjacentRequirement {
    fn matches(&self, identity: &crate::tile::Tile, other: Option<&crate::tile::Tile>) -> bool {
        match self {
            Self::Any => true,
            Self::Same => other
                .map(|n| n.kind.id() == identity.kind.id())
                .unwrap_or(false),
            Self::Other => other
                .map(|n| n.kind.id() != identity.kind.id())
                .unwrap_or(false),
            Self::SameGroup => other.map(|o| o.group == identity.group).unwrap_or(false),
            Self::OtherGroup => other.map(|o| o.group != identity.group).unwrap_or(false),
            Self::Either(e) => other
                // Any of the given `e` matches the id or the group
                .map(|o| {
                    e.iter().any(|e| {
                        e == o.kind.id() || o.group.as_ref().map(|o| o == e).unwrap_or(false)
                    })
                })
                .unwrap_or(false),
        }
    }

    fn prio(&self) -> usize {
        match self {
            Self::Any => 1,
            Self::SameGroup | Self::OtherGroup => 10,
            Self::Same | Self::Other => 100,
            Self::Either(_) => 1000,
        }
    }
}

impl PartialEq for AdjacentRequirement {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Any => matches!(other, Self::Any),
            Self::Same => matches!(other, Self::Same),
            Self::Other => matches!(other, Self::Other),
            Self::SameGroup => matches!(other, Self::SameGroup),
            Self::OtherGroup => matches!(other, Self::OtherGroup),
            Self::Either(_) => matches!(other, Self::Either(_)),
        }
    }
}
impl Eq for AdjacentRequirement {}

impl Ord for AdjacentRequirement {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.prio().cmp(&self.prio())
    }
}

impl PartialOrd for AdjacentRequirement {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum AdjacentRequirementDef {
    #[default]
    Any,
    Same,
    SameGroup,
    Other,
    OtherGroup,
    #[serde(with = "one_or_many")]
    Either(Vec<String>),
}

impl AdjacentRequirementDef {
    fn prio(&self) -> usize {
        match self {
            Self::Any => 1,
            Self::SameGroup | Self::OtherGroup => 10,
            Self::Same | Self::Other => 100,
            Self::Either(_) => 1000,
        }
    }
}

impl Ord for AdjacentRequirementDef {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.prio().cmp(&other.prio())
    }
}

impl PartialOrd for AdjacentRequirementDef {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl AdjacentRequirementDef {
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
                .filter_map(|result| result.inspect_err(|e| error!("{e}")).ok())
                .collect(),
            base: GroundTileVisual::from_def(config.base, load_context)?,
            above: config
                .above
                .into_iter()
                .map(|c| GroundTileVisual::from_def(c, load_context))
                .filter_map(|result| result.inspect_err(|e| error!("{e}")).ok())
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
            Self::Below => TILE_LAYER - 20.0,
            Self::Base => TILE_LAYER,
            Self::Above => TILE_LAYER + 1.0,
        }
    }
}

struct LayerCursor {
    current_layer: VisualLayer,
    current_idx: usize,
    below_len: usize,
    above_len: usize,
}

enum LayerItem {
    Below,
    Base,
    Above,
}

impl LayerCursor {
    fn next(&mut self) -> Option<(f32, LayerItem)> {
        if matches!(self.current_layer, VisualLayer::Below) {
            if self.current_idx < self.below_len {
                let z = VisualLayer::Below.z() + self.current_idx as f32;
                self.current_idx += 1;
                Some((z, LayerItem::Below))
            } else {
                self.current_layer = VisualLayer::Above;
                self.current_idx = 0;
                Some((VisualLayer::Base.z(), LayerItem::Base))
            }
        } else if self.current_idx < self.above_len {
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
                below_len: value.below.len(),
                above_len: value.above.len(),
            },
        }
    }
}

impl<'a> Iterator for LayerIterator<'a> {
    type Item = (f32, &'a GroundTileVisual);
    fn next(&mut self) -> Option<Self::Item> {
        let (z, layer_item) = self.cursor.next()?;
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
        let below_len = value.below.len();
        let above_len = value.above.len();
        Self {
            below: value.below.iter_mut(),
            base: Some(&mut value.base),
            above: value.above.iter_mut(),
            cursor: LayerCursor {
                current_layer: VisualLayer::Below,
                current_idx: 0,
                below_len,
                above_len,
            },
        }
    }
}

impl<'a> Iterator for _LayerIteratorMut<'a> {
    type Item = (f32, &'a mut GroundTileVisual);

    fn next(&mut self) -> Option<Self::Item> {
        let (z, layer_item) = self.cursor.next()?;
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
