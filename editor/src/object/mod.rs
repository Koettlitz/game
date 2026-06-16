use crate::asset::object::Door;
use crate::asset::object::GameObjectKindAsset;
use crate::ui::PlaceObject;
use bevy::prelude::*;
use engine::asset::AssetRef;
use engine::asset::AssetsExt;
use engine::asset::animation::sprite::SpriteAnimationAsset;
use engine::asset::overworld::CHARACTER_LAYER;
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
    animations: Res<Assets<SpriteAnimationAsset>>,
    mut message_reader: MessageReader<PlaceObject>,
    mut commands: Commands,
    grid_size: Single<&GridSize>,
) -> Result<()> {
    for PlaceObject { pos, object_kind } in message_reader.read() {
        let kind = object_kinds.require_handle(object_kind.handle())?;
        let position = grid_size.grid_to_world(pos.as_vec2());
        let mut game_object = commands.spawn((
            GameObject {
                kind_ref: object_kind.clone(),
            },
            Transform::from_translation(
                kind.offset()
                    .map_or_else(|| position, |offset| position + offset)
                    .extend(CHARACTER_LAYER),
            ),
        ));

        for (id_suffix, (sprite, transform)) in kind.create_main_sprites() {
            let id = if let Some(id_suffix) = id_suffix {
                format!("{}_{id_suffix}", object_kind.id())
            } else {
                object_kind.id().to_string()
            };
            game_object.with_child((GameObjectSprite::Main { id }, sprite, transform));
        }

        for (i, (door, (sprite, transform))) in kind
            .create_door_sprites(&animations)?
            .into_iter()
            .enumerate()
        {
            game_object.with_child((
                GameObjectSprite::Door {
                    id: format!("{}_door{i}", object_kind.id().to_string()),
                    door: door.clone(),
                },
                sprite,
                transform,
            ));
        }
    }
    Ok(())
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

#[derive(Component, Clone)]
pub enum GameObjectSprite {
    Main { id: String },
    Door { id: String, door: Door },
}
