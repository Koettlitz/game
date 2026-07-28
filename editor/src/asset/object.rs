use bevy::asset::AssetEventSystems;
use bevy::prelude::*;
use bevy_elf::AppExt;
use bevy_elf::AssetRef;
use bevy_elf::AssetResolver;
use bevy_elf::FromDef;
use engine::asset::AssetMap;
use engine::asset::AssetSetPlugin;
use engine::asset::AssetsExt;
use engine::asset::animation::sprite::SpriteAnimationAsset;
use engine::asset::spritesheet::SpritesheetKind;
use engine::overworld::tile::TILE_SIZE;
use macros::asset_set;
use serde::{Deserialize, Serialize};
use std::cmp;

pub type GameObjectKindMap = AssetMap<ObjectResolverSet, GameObjectKindAsset>;

pub struct GameObjectAssetPlugin;
impl Plugin for GameObjectAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_ron_asset::<GameObjectKindAsset>()
            .add_plugins(AssetSetPlugin::<GameObjectKindAsset>::default())
            .add_systems(PreUpdate, derive_image_data.after(AssetEventSystems));
    }
}

fn derive_image_data(
    mut message_reader: MessageReader<AssetEvent<TextureAtlasLayout>>,
    object_kind_map: Res<GameObjectKindMap>,
    mut object_kinds: ResMut<Assets<GameObjectKindAsset>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) -> Result<()> {
    for msg in message_reader.read() {
        let AssetEvent::LoadedWithDependencies { id } = msg else {
            continue;
        };
        for handle in object_kind_map.0.values() {
            let mut object_kind = object_kinds.require_handle_mut(handle)?;
            if &object_kind.spritesheet.layout().id() == id {
                object_kind.derive_offsets(&mut layouts)?;
            }
        }
    }
    Ok(())
}

#[derive(FromDef, Asset, TypePath)]
#[elf(def_type(GameObjectKindDef))]
#[asset_set(base_path = "objects")]
pub struct GameObjectKindAsset {
    collision_box: Option<IRect>,
    spritesheet: GameObjectSpritesheet,
    doors: Vec<Door>,

    #[elf(default)]
    offset: Option<Vec2>,
}

impl GameObjectKindAsset {
    pub fn collision_box(&self) -> Option<IRect> {
        self.collision_box
    }

    pub fn spritesheet(&self) -> &GameObjectSpritesheet {
        &self.spritesheet
    }

    pub fn offset(&self) -> Option<Vec2> {
        self.offset
    }

    pub fn doors(&self) -> &Vec<Door> {
        &self.doors
    }

    pub fn create_sprites(
        &self,
        animations: &Assets<SpriteAnimationAsset>,
    ) -> Result<Vec<(Sprite, Transform)>> {
        let mut sprites: Vec<(Sprite, Transform)> = self
            .create_main_sprites()
            .into_iter()
            .map(|(_, bundle)| bundle)
            .collect();
        self.create_door_sprites(animations)?
            .into_iter()
            .for_each(|(_, bundle)| sprites.push(bundle));

        Ok(sprites)
    }

    pub fn create_main_sprites(&self) -> Vec<(Option<String>, (Sprite, Transform))> {
        match self.spritesheet().kind() {
            ObjectSpriteKind::Single(index) => {
                let sprite = Sprite::from_atlas_image(
                    self.spritesheet().image().clone(),
                    TextureAtlas {
                        layout: self.spritesheet().layout().clone(),
                        index: *index,
                    },
                );
                if let Some(offset) = self.offset() {
                    vec![(
                        None,
                        (sprite, Transform::from_translation(offset.extend(-1.0))),
                    )]
                } else {
                    vec![(None, (sprite, Transform::default()))]
                }
            }
            ObjectSpriteKind::DepthSplit {
                top,
                bottom,
                top_offset,
                bottom_offset,
            } => {
                vec![
                    (
                        Some("top".to_string()),
                        (
                            Sprite::from_atlas_image(
                                self.spritesheet().image().clone(),
                                TextureAtlas {
                                    layout: self.spritesheet().layout().clone(),
                                    index: *top,
                                },
                            ),
                            Transform::from_translation(top_offset.as_ref().unwrap().extend(1.0)),
                        ),
                    ),
                    (
                        Some("bottom".to_string()),
                        (
                            Sprite::from_atlas_image(
                                self.spritesheet().image().clone(),
                                TextureAtlas {
                                    layout: self.spritesheet().layout().clone(),
                                    index: *bottom,
                                },
                            ),
                            Transform::from_translation(
                                bottom_offset.as_ref().unwrap().extend(-1.0),
                            ),
                        ),
                    ),
                ]
            }
        }
    }

    pub fn create_door_sprites(
        &self,
        animations: &Assets<SpriteAnimationAsset>,
    ) -> Result<Vec<(&Door, (Sprite, Transform))>> {
        let mut sprites = Vec::with_capacity(self.doors().len());

        for door in self.doors() {
            let index = animations
                .require_handle(door.open_animation().handle())?
                .frames[0];
            sprites.push((
                door,
                (
                    Sprite::from_atlas_image(
                        self.spritesheet().image().clone(),
                        TextureAtlas {
                            layout: self.spritesheet().layout().clone(),
                            index,
                        },
                    ),
                    Transform::from_translation(door.offset().as_vec2().extend(-1.0)),
                ),
            ))
        }

        Ok(sprites)
    }

    fn derive_offsets<'a>(&'a mut self, layouts: &'a mut Assets<TextureAtlasLayout>) -> Result<()> {
        let layout = layouts.require_handle(self.spritesheet().layout())?;
        match &mut self.spritesheet.kind {
            ObjectSpriteKind::Single(index) => {
                let size = layout.textures[*index].size();
                self.offset = Some(Vec2 {
                    x: -Self::grid_align_offset(size.x),
                    y: Self::grid_align_offset(size.y),
                });
            }
            ObjectSpriteKind::DepthSplit {
                top,
                bottom,
                top_offset,
                bottom_offset,
            } => {
                let top_rect = layout.textures[*top];
                let bottom_rect = layout.textures[*bottom];
                let size = UVec2 {
                    x: cmp::max(top_rect.size().x, bottom_rect.size().x),
                    y: cmp::max(top_rect.size().y, bottom_rect.size().y),
                };
                let grid_align_offset = Vec2 {
                    x: -Self::grid_align_offset(size.x),
                    y: Self::grid_align_offset(size.y),
                };
                let y_offset = |rect: URect| rect.size().y as f32 / 2.0;
                let mut calculated_top_offset = Vec2 {
                    x: 0.0,
                    y: y_offset(top_rect),
                };
                if Self::grid_align_offset(top_rect.size().x) == 0.0 {
                    calculated_top_offset.x -= grid_align_offset.x;
                }
                if Self::grid_align_offset(top_rect.size().y) == 0.0 {
                    calculated_top_offset.y -= grid_align_offset.y;
                }
                *top_offset = Some(calculated_top_offset);

                let mut calculated_bottom_offset = Vec2 {
                    x: 0.0,
                    y: -y_offset(bottom_rect),
                };
                if Self::grid_align_offset(bottom_rect.size().x) == 0.0 {
                    calculated_bottom_offset.x -= grid_align_offset.x;
                }
                if Self::grid_align_offset(bottom_rect.size().y) == 0.0 {
                    calculated_bottom_offset.y -= grid_align_offset.y;
                }
                *bottom_offset = Some(calculated_bottom_offset);

                self.offset = Some(grid_align_offset);
            }
        }
        Ok(())
    }

    fn grid_align_offset(length: u32) -> f32 {
        let length_in_tiles = length.div_ceil(TILE_SIZE);
        if length_in_tiles.is_multiple_of(2) {
            TILE_SIZE as f32 / 2.0
        } else {
            0.0
        }
    }
}

#[derive(FromDef)]
pub struct GameObjectSpritesheet {
    #[elf(implicit, with_resolver(SpritesheetKind::Object))]
    image: Handle<Image>,

    #[elf(
        implicit,
        with_spec(base_path = "objects/spritesheets/layouts", extension = "layout.ron")
    )]
    layout: Handle<TextureAtlasLayout>,
    kind: ObjectSpriteKind,
}

impl GameObjectSpritesheet {
    pub fn image(&self) -> &Handle<Image> {
        &self.image
    }

    pub fn layout(&self) -> &Handle<TextureAtlasLayout> {
        &self.layout
    }

    pub fn kind(&self) -> &ObjectSpriteKind {
        &self.kind
    }
}

#[derive(FromDef, Serialize, Deserialize, Clone)]
pub enum ObjectSpriteKind {
    Single(usize),
    DepthSplit {
        top: usize,
        bottom: usize,

        #[elf(default)]
        top_offset: Option<Vec2>,

        #[elf(default)]
        bottom_offset: Option<Vec2>,
    },
}

#[derive(Serialize, Deserialize)]
pub struct GameObjectKindDef {
    spritesheet: GameObjectSpritesheetDef,

    #[serde(skip_serializing_if = "Option::is_none")]
    collision_box: Option<IRect>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    doors: Vec<DoorDef>,
}

#[derive(FromDef, Clone)]
pub struct Door {
    offset: IVec2,
    target_lozo: String,

    #[elf(
        with_spec(base_path = "objects/animations", extension = "ani.ron"),
        expose_resolver
    )]
    open_animation: AssetRef<SpriteAnimationAsset>,

    #[elf(
        with_spec(base_path = "objects/animations", extension = "ani.ron"),
        expose_resolver
    )]
    close_animation: AssetRef<SpriteAnimationAsset>,
}

impl Door {
    pub fn open_animation(&self) -> &AssetRef<SpriteAnimationAsset> {
        &self.open_animation
    }

    pub fn open_animation_path(&self) -> Result<String> {
        Ok(Self::open_animation_resolver()
            .resolve(self.open_animation.id())?
            .path()
            .to_string_lossy()
            .to_string())
    }

    pub fn close_animation(&self) -> &AssetRef<SpriteAnimationAsset> {
        &self.close_animation
    }

    pub fn close_animation_path(&self) -> Result<String> {
        Ok(Self::close_animation_resolver()
            .resolve(self.close_animation().id())?
            .path()
            .to_string_lossy()
            .to_string())
    }

    pub fn offset(&self) -> IVec2 {
        self.offset
    }

    pub fn target_lozo(&self) -> &str {
        &self.target_lozo
    }
}
