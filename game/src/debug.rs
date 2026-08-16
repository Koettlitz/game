use bevy::prelude::*;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, debug);
    }
}

fn debug(input: Res<ButtonInput<KeyCode>>) -> Result {
    if !input.just_pressed(KeyCode::Enter) {
        return Ok(());
    }

    // Do some stuff for debugging purposes here

    Ok(())
}
