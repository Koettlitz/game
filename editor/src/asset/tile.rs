use bevy_elf::{AssetRef, FromDef, FromDefError, RonAssetPlugin, asset_spec};
use engine::{
    asset::{
        AssetMap, AssetSetPlugin, AssetsExt, animation::sprite::SpriteAnimationAsset, one_or_many,
        overworld::TILE_LAYER, spritesheet::SpritesheetKind,
    },
    overworld::tile::{GridCursor, Neighbor, Passability},
};
use macros::asset_set;
use std::{
    collections::HashMap,
    fmt::{self, Debug, Display},
    slice,
};

use engine::overworld::tile::TILE_SIZE;

use bevy::{
    asset::{AssetEventSystems, LoadContext},
    prelude::*,
};
use engine::asset::implicit_option;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
            let mut tile_kind = tile_kinds.require_handle_mut(tile_kind)?;
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

    #[elf(from_default)]
    pub spritesheet: TileKindSpritesheet,

    #[elf(implicit)]
    pub edge_config: Handle<TileEdgeConfig>,
}

#[derive(FromDef, TypePath, Debug)]
#[elf(def_type(()))]
pub struct TileKindSpritesheet {
    #[elf(implicit, with_resolver(SpritesheetKind::Tile))]
    image: Handle<Image>,

    #[elf(default)]
    layout: Option<Handle<TextureAtlasLayout>>,
}

impl TileKindSpritesheet {
    pub fn image(&self) -> &Handle<Image> {
        &self.image
    }

    pub fn layout(&self) -> Result<&Handle<TextureAtlasLayout>, TileSpriteLayoutError> {
        self.layout
            .as_ref()
            .ok_or_else(|| TileSpriteLayoutError(self.image.id().to_string()))
    }

    pub fn derive_layout<'a>(
        &'a mut self,
        images: &Assets<Image>,
        layouts: &'a mut Assets<TextureAtlasLayout>,
    ) -> Result<&'a Handle<TextureAtlasLayout>> {
        let image = images.require_handle(&self.image)?;
        let layout =
            derive_texture_atlas_layout(image).ok_or_else(|| InvalidTileSpritesheetSize {
                id: self.image.id().to_string(),
            })?;
        let handle = layouts.add(layout);
        self.layout = Some(handle.clone());
        Ok(self.layout.as_ref().unwrap())
    }
}

#[derive(Error, Debug)]
pub struct InvalidTileSpritesheetSize {
    pub id: String,
}

impl Display for InvalidTileSpritesheetSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "size of sprite sheet \"{}\" not a multiple of tile size: {TILE_SIZE}",
            self.id
        )
    }
}

#[derive(Error, Debug)]
#[error("missing TextureAtlasLayout for tile sprite \"{0}\"")]
pub struct TileSpriteLayoutError(String);

fn derive_texture_atlas_layout(image: &Image) -> Option<TextureAtlasLayout> {
    if image.size() % TILE_SIZE != UVec2::splat(0) {
        return None;
    }
    let size_in_tiles = image.size() / UVec2::splat(TILE_SIZE);
    let layout = TextureAtlasLayout::from_grid(
        UVec2::splat(TILE_SIZE),
        size_in_tiles.x,
        size_in_tiles.y,
        None,
        None,
    );
    Some(layout)
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

    fn from_def(
        def: Self::Def,
        load_context: &mut LoadContext,
    ) -> std::result::Result<Self, FromDefError> {
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

#[derive(FromDef, Debug, Default, PartialEq, Eq)]
#[elf(on_def(
    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
))]
pub struct AdjacentRequirements {
    #[elf(on_def(
        #[serde(default, skip_serializing_if = "AdjacentRequirementDef::is_any")]
    ))]
    pub top_left: AdjacentRequirement,

    #[elf(on_def(
        #[serde(default, skip_serializing_if = "AdjacentRequirementDef::is_any")]
    ))]
    pub top: AdjacentRequirement,

    #[elf(on_def(
        #[serde(default, skip_serializing_if = "AdjacentRequirementDef::is_any")]
    ))]
    pub top_right: AdjacentRequirement,

    #[elf(on_def(
        #[serde(default, skip_serializing_if = "AdjacentRequirementDef::is_any")]
    ))]
    pub left: AdjacentRequirement,

    #[elf(on_def(
        #[serde(default, skip_serializing_if = "AdjacentRequirementDef::is_any")]
    ))]
    pub right: AdjacentRequirement,

    #[elf(on_def(
        #[serde(default, skip_serializing_if = "AdjacentRequirementDef::is_any")]
    ))]
    pub bottom_left: AdjacentRequirement,

    #[elf(on_def(
        #[serde(default, skip_serializing_if = "AdjacentRequirementDef::is_any")]
    ))]
    pub bottom: AdjacentRequirement,

    #[elf(on_def(
        #[serde(default, skip_serializing_if = "AdjacentRequirementDef::is_any")]
    ))]
    pub bottom_right: AdjacentRequirement,
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
#[elf(on_def(
    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Default)]
))]
pub enum AdjacentRequirement {
    #[default]
    #[elf(on_def(#[default]))]
    Any,
    Same,
    Other,
    SameGroup,
    OtherGroup,

    #[elf(on_def(
        #[serde(with = "one_or_many")]
    ))]
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

#[derive(FromDef, Debug)]
#[elf(on_def(
    #[derive(Serialize, Deserialize, Debug)]
))]
pub struct GroundTileVisualLayers {
    #[elf(on_def(
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
    ))]
    below: Vec<GroundTileVisual>,
    base: GroundTileVisual,

    #[elf(on_def(
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
    ))]
    above: Vec<GroundTileVisual>,
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
            Self::Below => TILE_LAYER - 10.0,
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
                let z = VisualLayer::Below.z() + self.current_idx as f32 / 10.0;
                self.current_idx += 1;
                Some((z, LayerItem::Below))
            } else {
                self.current_layer = VisualLayer::Above;
                self.current_idx = 0;
                Some((VisualLayer::Base.z(), LayerItem::Base))
            }
        } else if self.current_idx < self.above_len {
            let z = VisualLayer::Above.z() + self.current_idx as f32 / 10.0;
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

impl From<GroundTileVisualDef> for GroundTileVisualLayersDef {
    fn from(value: GroundTileVisualDef) -> Self {
        Self {
            below: Vec::new(),
            base: value,
            above: Vec::new(),
        }
    }
}

#[derive(FromDef, Debug)]
#[elf(on_def(
    #[derive(Serialize, Deserialize, Debug)]
))]
pub enum GroundTileVisual {
    Static(usize),
    Animated(
        #[elf(with_spec(base_path = "tiles/animations", extension = "ani.ron"))]
        AssetRef<SpriteAnimationAsset>,
    ),
    Neighbor(Neighbor),
}
