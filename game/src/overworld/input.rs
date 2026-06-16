use bevy::prelude::*;
use engine::overworld::{character::CharacterController, input::InputSystems};

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, move_character.in_set(InputSystems));
    }
}

fn move_character(input: Res<ButtonInput<KeyCode>>, mut query: Query<&mut CharacterController>) {
    for mut controller in &mut query {
        controller.up = input.pressed(KeyCode::KeyW);
        controller.left = input.pressed(KeyCode::KeyA);
        controller.right = input.pressed(KeyCode::KeyD);
        controller.down = input.pressed(KeyCode::KeyS);
    }
}
