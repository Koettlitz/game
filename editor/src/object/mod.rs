use crate::ui::{PlaceObject, SpawnCursorSprite};
use asset::{Door, GameObjectAssetPlugin, GameObjectKindAsset};
use bevy::prelude::*;
use bevy_elf::AssetRef;
use engine::animation::SpriteAnimationAsset;
use engine::asset::AssetsExt;
use engine::overworld::CHARACTER_LAYER;
use engine::overworld::tile::GridSize;
use engine::progress::ProgressState;

pub mod asset;

pub struct GameObjectPlugin;
impl Plugin for GameObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GameObjectAssetPlugin)
            .add_observer(spawn_cursor_sprite)
            .add_systems(
                Update,
                place_object.run_if(in_state(ProgressState::Finished)),
            );
    }
}

fn spawn_cursor_sprite(
    event: On<SpawnCursorSprite<GameObjectKindAsset>>,
    object_kinds: Res<Assets<GameObjectKindAsset>>,
    animations: Res<Assets<SpriteAnimationAsset>>,
    mut commands: Commands,
) -> Result {
    let object_kind = object_kinds.require_handle(&event.asset)?;
    let sprites = object_kind.create_sprites(&animations)?;
    let mut parent = if let Some(offset) = object_kind.offset() {
        let offset = commands
            .spawn((
                Visibility::default(),
                Transform::from_translation(offset.extend(0.0)),
            ))
            .id();
        commands.entity(event.cursor).add_child(offset);
        commands.entity(offset)
    } else {
        commands.entity(event.cursor)
    };

    for (mut sprite, transform) in sprites {
        sprite.color = sprite.color.with_alpha(event.alpha);
        parent.with_child((sprite, transform));
    }

    Ok(())
}

fn place_object(
    mut message_reader: MessageReader<PlaceObject>,
    grid_size: Single<&GridSize>,
    object_kinds: Res<Assets<GameObjectKindAsset>>,
    animations: Res<Assets<SpriteAnimationAsset>>,
    mut commands: Commands,
) -> Result {
    for PlaceObject {
        world_position,
        object_kind,
    } in message_reader.read()
    {
        let kind = object_kinds.require_handle(object_kind.handle())?;
        let world_position = grid_size.snap_to_tile(*world_position);
        let mut game_object = commands.spawn((
            GameObject {
                kind_ref: object_kind.clone(),
            },
            Transform::from_translation(
                kind.offset()
                    .map_or_else(|| world_position, |offset| world_position + offset)
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
                    id: format!("{}_door{i}", object_kind.id()),
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

impl GameObjectSprite {
    pub fn id(&self) -> &str {
        match self {
            Self::Main { id } => id,
            Self::Door { id, .. } => id,
        }
    }
}
