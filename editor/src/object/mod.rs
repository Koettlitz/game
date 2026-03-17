use std::collections::HashMap;

use crate::assets::object::ObjectSprites;
use crate::assets::object::{GameObjectAsset, ObjectAssets, ObjectSpriteSheetMap};
use crate::ui::PlaceObject;
use bevy::prelude::*;
use engine::Id;
use engine::assets::LoadState;
use engine::assets::SpriteSheetMap;
use engine::assets::{AssetMap, SpriteSheet};
use engine::overworld::tile::GridSize;
use engine::progress::ProgressState;

const OBJECT_LAYER: f32 = 128.0;

pub struct GameObjectPlugin;
impl Plugin for GameObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            spawn_object_kinds
                .run_if(in_state(LoadState::<ObjectSprites>::finished()))
                .run_if(resource_exists::<AssetMap<ObjectAssets>>),
        )
        .add_systems(
            Update,
            cleanup
                .run_if(resource_exists::<AssetMap<ObjectAssets>>)
                .run_if(in_state(LoadState::<ObjectAssets>::finished())),
        )
        .add_systems(
            Update,
            place_object.run_if(in_state(ProgressState::Finished)),
        );
    }
}

fn spawn_object_kinds(
    mut commands: Commands,
    mut assets: ResMut<Assets<GameObjectAsset>>,
    asset_map: Res<AssetMap<ObjectAssets>>,
    mut spritesheet_map: ResMut<ObjectSpriteSheetMap>,
) {
    for (id, handle) in asset_map.iter() {
        let Some(asset) = assets.remove(handle.id()) else {
            continue;
        };
        let Some(sprite_sheet) = spritesheet_map.remove(&asset.sprite_sheet_id) else {
            error!(
                "missing sprite sheet \"{}\" for game object \"{id}\"",
                asset.sprite_sheet_id
            );
            continue;
        };
        commands.spawn((Id(id.clone()), GameObjectKind::from(asset), sprite_sheet));
    }
}

fn place_object(
    mut message_reader: MessageReader<PlaceObject>,
    mut commands: Commands,
    grid_size: Res<GridSize>,
    object_kinds: Query<&SpriteSheet, With<GameObjectKind>>,
) {
    for PlaceObject { pos, object_kind } in message_reader.read() {
        let sprite_sheet = object_kinds
            .get(*object_kind)
            .expect("missing sprite sheet of game object kind");
        commands.spawn((
            GameObject { kind: *object_kind },
            Sprite {
                image: sprite_sheet.image.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: sprite_sheet.layout.clone(),
                    index: 0,
                }),
                ..Default::default()
            },
            Transform::from_translation(grid_size.to_world_pos(pos).extend(OBJECT_LAYER)),
        ));
    }
}

#[derive(Component)]
struct GameObject {
    kind: Entity,
}

#[derive(Component)]
pub struct GameObjectKind {
    _collision_box: Option<IRect>,
    _lozo_transitions: HashMap<IVec2, String>,
}

impl From<GameObjectAsset> for GameObjectKind {
    fn from(config: GameObjectAsset) -> Self {
        Self {
            _collision_box: config.collision_box,
            _lozo_transitions: config.lozo_transitions.clone(),
        }
    }
}

fn cleanup(mut commands: Commands, object_assets: Res<AssetMap<ObjectAssets>>) {
    if object_assets.0.is_empty() {
        commands.remove_resource::<AssetMap<ObjectAssets>>();
    }
}
