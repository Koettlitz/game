use bevy::asset::AssetEventSystems;
use bevy::prelude::*;
use engine::asset::AssetMap;
use engine::asset::AssetRef;
use engine::asset::AssetSetPlugin;
use engine::asset::AssetsExt;
use engine::asset::RonAssetPlugin;
use engine::asset::spritesheet::SpritesheetKind;
use engine::overworld::tile::TILE_SIZE;
use macros::FromDef;
use macros::asset_set;
use serde::{Deserialize, Serialize};

pub type GameObjectKindMap = AssetMap<ObjectResolverSet, GameObjectKindAsset>;

pub struct GameObjectAssetPlugin;
impl Plugin for GameObjectAssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RonAssetPlugin::<GameObjectKindAsset>::default(),
            AssetSetPlugin::<GameObjectKindAsset>::default(),
        ))
        .add_systems(PreUpdate, derive_image_data.after(AssetEventSystems));
    }
}

fn derive_image_data(
    mut message_reader: MessageReader<AssetEvent<Image>>,
    object_kind_map: Res<GameObjectKindMap>,
    mut object_kinds: ResMut<Assets<GameObjectKindAsset>>,
    images: Res<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) -> Result<()> {
    for msg in message_reader.read() {
        let AssetEvent::LoadedWithDependencies { id } = msg else {
            continue;
        };
        for handle in object_kind_map.0.values() {
            let object_kind = object_kinds.require_handle_mut(handle)?;
            if &object_kind.spritesheet.image().handle().id() == id {
                object_kind
                    .spritesheet
                    .derive_image_data(&images, &mut layouts)?;
            }
        }
    }
    Ok(())
}

#[derive(FromDef, Asset, TypePath)]
#[def_type(GameObjectKindDef)]
#[asset_set(base_path = "objects")]
pub struct GameObjectKindAsset {
    collision_box: Option<IRect>,
    spritesheet: GameObjectSpritesheet,
}

impl GameObjectKindAsset {
    pub fn collision_box(&self) -> Option<IRect> {
        self.collision_box
    }

    pub fn spritesheet(&self) -> &GameObjectSpritesheet {
        &self.spritesheet
    }
}

#[derive(FromDef)]
pub struct GameObjectSpritesheet {
    #[from_def(implicit, with_resolver(SpritesheetKind::Object))]
    image: AssetRef<Image>,
    z_divide_at_y: Option<ZDivideY>,

    #[from_def(default)]
    layout: Option<Handle<TextureAtlasLayout>>,

    #[from_def(default)]
    offsets: Option<Vec<Vec2>>,
}

impl GameObjectSpritesheet {
    pub fn image(&self) -> &AssetRef<Image> {
        &self.image
    }

    pub fn kind(&self) -> GameObjectSpritesheetKind {
        if let Some(ref layout) = self.layout {
            let offsets = self
                .offsets
                .as_ref()
                .expect("missing offsets in divided game object sprite sheet");
            if offsets.len() != 2 {
                panic!(
                    "expected two offsets in divided GameObjectSpritesheet, but {} were present",
                    offsets.len()
                );
            }
            GameObjectSpritesheetKind::Divided {
                layout: layout.clone(),
                top: GameObjectSpritesheetPart {
                    name: "top".to_string(),
                    offset: offsets[0],
                    layout_index: 0,
                },
                bottom: GameObjectSpritesheetPart {
                    name: "bottom".to_string(),
                    offset: offsets[1],
                    layout_index: 1,
                },
            }
        } else {
            let offset = if let Some(ref offsets) = self.offsets {
                if offsets.len() != 1 {
                    panic!(
                        "expected one offset in undivided GameObjectSpritesheet, but {} were present",
                        offsets.len()
                    );
                }
                Some(offsets[0])
            } else {
                None
            };
            GameObjectSpritesheetKind::Single { offset }
        }
    }

    fn derive_image_data<'a>(
        &'a mut self,
        images: &Assets<Image>,
        layouts: &'a mut Assets<TextureAtlasLayout>,
    ) -> Result<()> {
        let image = images.require_handle(&self.image.handle())?;
        let Some(ref z_divide_y) = self.z_divide_at_y else {
            self.offsets = Some(vec![Self::grid_align_offset(&image)]);
            return Ok(());
        };
        let division_line_y = match z_divide_y {
            ZDivideY::Y(y) => *y,
            ZDivideY::Halve => image.size().y / 2,
        };
        let handle = layouts.add(TextureAtlasLayout {
            size: image.size(),
            textures: vec![
                URect::new(0, 0, image.size().x, division_line_y),
                URect::new(0, division_line_y, image.size().x, image.size().y),
            ],
        });
        self.layout = Some(handle.clone());
        let half_height = image.size_f32().y / 2.0;
        let offset_to_y = half_height - division_line_y as f32;
        let top_height = division_line_y as f32;
        let bottom_height = image.size_f32().y - top_height;
        let grid_align_offset = Self::grid_align_offset(&image);
        self.offsets = Some(vec![
            Vec2 {
                x: grid_align_offset.x,
                y: grid_align_offset.y + offset_to_y + top_height / 2.0,
            },
            Vec2 {
                x: grid_align_offset.x,
                y: grid_align_offset.y + offset_to_y - bottom_height / 2.0,
            },
        ]);
        Ok(())
    }

    fn grid_align_offset(image: &Image) -> Vec2 {
        let width = image.size().x.div_ceil(TILE_SIZE.x);
        let x = if width % 2 == 0 {
            -(TILE_SIZE.x as f32) / 2.0
        } else {
            0.0
        };
        let height = image.size().y.div_ceil(TILE_SIZE.y);
        let y = if height % 2 == 0 {
            TILE_SIZE.y as f32 / 2.0
        } else {
            0.0
        };
        Vec2 { x, y }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GameObjectKindDef {
    spritesheet: GameObjectSpritesheetDef,

    #[serde(skip_serializing_if = "Option::is_none")]
    z_divide_at_y: Option<ZDivideY>,

    #[serde(skip_serializing_if = "Option::is_none")]
    collision_box: Option<IRect>,
}

#[derive(FromDef, Serialize, Deserialize)]
#[def_type(Self)]
pub enum ZDivideY {
    Y(u32),
    Halve,
}

pub enum GameObjectSpritesheetKind {
    Single {
        offset: Option<Vec2>,
    },
    Divided {
        layout: Handle<TextureAtlasLayout>,
        top: GameObjectSpritesheetPart,
        bottom: GameObjectSpritesheetPart,
    },
}

pub struct GameObjectSpritesheetPart {
    name: String,
    offset: Vec2,
    layout_index: usize,
}

impl GameObjectSpritesheetPart {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn offset(&self) -> Vec2 {
        self.offset
    }

    pub fn layout_index(&self) -> usize {
        self.layout_index
    }
}
