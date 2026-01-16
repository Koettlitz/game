use std::time::Duration;

use bevy::{platform::collections::HashMap, prelude::*};
use engine::assets::SpriteSheet;
use strum::IntoEnumIterator;

use crate::tile::GroundTileKind;

pub struct TileVisualsPlugin;
impl Plugin for TileVisualsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TextureAtlasLayoutRegistry>()
            .add_systems(Startup, spawn_tile_visuals)
            .add_systems(PostUpdate, animate_tiles);
    }
}

fn spawn_tile_visuals(mut commands: Commands) {
    for tile_kind in GroundTileKind::iter() {
        commands.spawn((tile_kind, tile_kind.visual()));
    }
}

fn animate_tiles(
    tile_visuals: Query<(&GroundTileKind, &mut GroundTileVisual)>,
    mut sprites: Query<(&GroundTileKind, &mut Sprite), With<GroundTileSprite>>,
    time: Res<Time>,
) {
    for (&tile_kind, mut visual) in tile_visuals {
        let GroundTileVisual::Animation(animation) = visual.as_mut() else {
            continue;
        };
        if animation.timer.tick(time.delta()).just_finished() {
            let next_index = animation.next_index();
            for (_, mut sprite) in sprites.iter_mut().filter(|(k, _)| **k == tile_kind) {
                let atlas = sprite
                    .texture_atlas
                    .as_mut()
                    .expect("tile sprite to have a texture atlas");
                atlas.index = next_index;
            }
        }
    }
}

#[derive(Component)]
pub struct GroundTileSprite;

#[derive(Resource, Default)]
pub struct TextureAtlasLayoutRegistry(HashMap<SpriteSheet, Handle<TextureAtlasLayout>>);
impl TextureAtlasLayoutRegistry {
    pub fn get_or_load(
        &mut self,
        sprite_sheet: SpriteSheet,
        assets: &mut Assets<TextureAtlasLayout>,
    ) -> Handle<TextureAtlasLayout> {
        self.0
            .entry(sprite_sheet)
            .or_insert_with(|| assets.add(sprite_sheet.texture_atlas_layout()))
            .clone()
    }
}

pub struct GroundTileAnimation {
    indices: Vec<usize>,
    current: usize,
    timer: Timer,
}

impl GroundTileAnimation {
    pub fn new(indices: impl Into<Vec<usize>>, frame_duration: Duration) -> Self {
        Self {
            indices: indices.into(),
            current: 0,
            timer: Timer::new(frame_duration, TimerMode::Repeating),
        }
    }

    pub fn current_idx(&self) -> usize {
        self.indices[self.current]
    }

    fn next_index(&mut self) -> usize {
        if self.current == self.indices.len() - 1 {
            self.current = 0;
        } else {
            self.current += 1;
        }
        self.current_idx()
    }
}

#[derive(Component)]
#[require(GroundTileKind)]
pub enum GroundTileVisual {
    Static(usize),
    Animation(GroundTileAnimation),
}

impl GroundTileVisual {
    pub fn texture_atlas_index(&self) -> usize {
        match self {
            Self::Static(idx) => *idx,
            Self::Animation(animation) => animation.current_idx(),
        }
    }
}

impl GroundTileKind {
    pub fn visual(&self) -> GroundTileVisual {
        match self {
            Self::Gras => GroundTileVisual::Static(1),
            Self::WaterCalm => GroundTileVisual::Static(7),
            Self::WaterDeep => GroundTileVisual::Animation(GroundTileAnimation::new(
                vec![2, 5, 8, 11, 14, 17, 20, 23],
                Duration::from_millis(200),
            )),
        }
    }
}
