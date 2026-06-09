use crate::asset::object::GameObjectKindAsset;
use crate::asset::object::GameObjectSpritesheetKind;
use crate::ui::PlaceObject;
use bevy::prelude::*;
use engine::asset::AssetRef;
use engine::asset::AssetsExt;
use engine::asset::overworld::OBJECT_LAYER_BOTTOM;
use engine::asset::overworld::OBJECT_LAYER_TOP;
use engine::overworld::tile::GridSize;
use engine::progress::ProgressState;

pub struct GameObjectPlugin;
impl Plugin for GameObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            place_object.run_if(in_state(ProgressState::Finished)),
        );
    }
}

fn place_object(
    object_kinds: Res<Assets<GameObjectKindAsset>>,
    mut message_reader: MessageReader<PlaceObject>,
    mut commands: Commands,
    grid_size: Single<&GridSize>,
) -> Result<()> {
    for PlaceObject { pos, object_kind } in message_reader.read() {
        let kind = object_kinds.require_handle(object_kind.handle())?;
        let mut game_object = commands.spawn((
            GameObject {
                kind_ref: object_kind.clone(),
            },
            Transform::from_translation(grid_size.grid_to_world(pos.as_vec2()).extend(0.0)),
        ));
        for sprite in create_object_sprites_for(kind) {
            game_object.with_child(sprite);
        }
    }
    Ok(())
}

pub fn create_object_sprites_for(
    kind: &GameObjectKindAsset,
) -> Vec<(GameObjectSprite, Sprite, Transform)> {
    match kind.spritesheet().kind() {
        GameObjectSpritesheetKind::Single { offset } => {
            let translation = if let Some(offset) = offset {
                offset.extend(OBJECT_LAYER_BOTTOM)
            } else {
                Vec3::new(0.0, 0.0, OBJECT_LAYER_BOTTOM)
            };
            vec![(
                GameObjectSprite(kind.spritesheet().image().id().to_string()),
                Sprite::from_image(kind.spritesheet().image().handle().clone()),
                Transform::from_translation(translation),
            )]
        }
        GameObjectSpritesheetKind::Divided {
            layout,
            top,
            bottom,
        } => [(top, OBJECT_LAYER_TOP), (bottom, OBJECT_LAYER_BOTTOM)]
            .iter()
            .map(|(part, z)| {
                (
                    GameObjectSprite(format!(
                        "{}_{}",
                        kind.spritesheet().image().id(),
                        part.name()
                    )),
                    Sprite::from_atlas_image(
                        kind.spritesheet().image().handle().clone(),
                        TextureAtlas {
                            index: part.layout_index(),
                            layout: layout.clone(),
                        },
                    ),
                    Transform::from_translation(part.offset().extend(*z)),
                )
            })
            .collect(),
    }
}

#[derive(Component)]
#[require(Visibility)]
pub struct GameObject {
    kind_ref: AssetRef<GameObjectKindAsset>,
}

impl GameObject {
    pub fn kind_ref(&self) -> &AssetRef<GameObjectKindAsset> {
        &self.kind_ref
    }
}

#[derive(Component)]
pub struct GameObjectSprite(String);

impl GameObjectSprite {
    pub fn id(&self) -> &str {
        &self.0
    }
}
