use bevy::prelude::*;
use bevy_elf::{AppExt, AssetRef};
use engine::{
    animation::{Animated, SpriteAnimationAsset},
    asset::{AssetSetPlugin, AssetsExt},
    overworld::{
        CHARACTER_LAYER,
        character::{
            CHARACTER_SPRITE_SCALE, Orientation,
            asset::{CharacterState, CharacterVisual},
        },
        tile::GridSize,
    },
    progress::ProgressState,
};

use asset::CharacterKindAsset;

use crate::ui::{PlaceCharacter, SpawnCursorSprite};

pub mod asset;

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.init_ron_asset::<CharacterKindAsset>()
            .add_plugins(AssetSetPlugin::<CharacterKindAsset>::default())
            .add_systems(
                Update,
                place_character.run_if(in_state(ProgressState::Finished)),
            )
            .add_observer(spawn_cursor_sprite);
    }
}

fn spawn_cursor_sprite(
    event: On<SpawnCursorSprite<CharacterKindAsset>>,
    character_kinds: Res<Assets<CharacterKindAsset>>,
    animations: Res<Assets<SpriteAnimationAsset>>,
    mut commands: Commands,
) -> Result {
    let asset = character_kinds.require_handle(&event.asset)?;

    let mut sprite = Sprite::from_atlas_image(
        asset.spritesheet.image().handle().clone(),
        TextureAtlas {
            layout: asset.spritesheet.layout().handle().clone(),
            index: asset.default_visual().atlas_index(&animations)?,
        },
    );
    sprite.color = sprite.color.with_alpha(event.alpha);

    commands
        .entity(event.cursor)
        .with_child((sprite, Transform::from_scale(CHARACTER_SPRITE_SCALE)));

    Ok(())
}

#[derive(Component)]
#[require(Transform, Sprite)]
pub struct Character {
    pub asset_ref: AssetRef<CharacterKindAsset>,
    pub orientation: Orientation,
}

fn place_character(
    mut event_reader: MessageReader<PlaceCharacter>,
    grid_size: Single<&GridSize>,
    assets: Res<Assets<CharacterKindAsset>>,
    mut commands: Commands,
) -> Result {
    for PlaceCharacter {
        world_position,
        character_kind,
        orientation,
    } in event_reader.read()
    {
        let asset = assets.require_handle(character_kind.handle())?;
        let (index, animation) = match &asset.animations[&CharacterState::Standing(*orientation)] {
            CharacterVisual::Static(idx) => (*idx, None),
            CharacterVisual::Animated(animation) => (0, Some(animation)),
        };
        let world_position = grid_size.snap_to_tile(*world_position);

        let mut sprite_commands = commands.spawn((
            Character {
                asset_ref: character_kind.clone(),
                orientation: *orientation,
            },
            Sprite::from_atlas_image(
                asset.spritesheet.image().handle().clone(),
                TextureAtlas {
                    layout: asset.spritesheet.layout().handle().clone(),
                    index,
                },
            ),
            Transform::from_translation(world_position.extend(CHARACTER_LAYER))
                .with_scale(CHARACTER_SPRITE_SCALE),
        ));
        if let Some(animation) = animation {
            sprite_commands.insert(Animated::by(animation.handle().clone()));
        }
    }

    Ok(())
}
