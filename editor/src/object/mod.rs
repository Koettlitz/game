use crate::asset::object::GameObjectKindAsset;
use crate::ui::PlaceObject;
use bevy::prelude::*;
use engine::asset::AssetRef;
use engine::asset::MissingAssetError;
use engine::overworld::tile::GridSize;
use engine::progress::ProgressState;

const OBJECT_LAYER: f32 = 128.0;

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
    grid_size: Res<GridSize>,
) -> Result<()> {
    for PlaceObject { pos, object_kind } in message_reader.read() {
        let asset = object_kinds.get(object_kind.handle().id()).ok_or_else(|| {
            MissingAssetError::<GameObjectKindAsset>::new(object_kind.handle().id())
        })?;
        commands.spawn((
            GameObject {
                kind_ref: object_kind.clone(),
            },
            Sprite::from_image(asset.sprite_sheet.handle().clone()),
            Transform::from_translation(grid_size.to_world_pos(pos).extend(OBJECT_LAYER)),
        ));
    }
    Ok(())
}

#[derive(Component)]
pub struct GameObject {
    kind_ref: AssetRef<GameObjectKindAsset>,
}

impl GameObject {
    pub fn kind_ref(&self) -> &AssetRef<GameObjectKindAsset> {
        &self.kind_ref
    }
}
