use bevy::{math::FloatPow, prelude::*};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init)
            .add_systems(Update, (update_movement, apply_movement).chain());
    }
}

#[derive(Component)]
pub struct CameraMovement {
    pub up: bool,
    pub left: bool,
    pub right: bool,
    pub down: bool,
    timer: Timer,
    min_velocity: f32,
    max_addtition: f32,
}

impl Default for CameraMovement {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(4.0, TimerMode::Once),
            up: false,
            left: false,
            right: false,
            down: false,
            min_velocity: 10.0,
            max_addtition: 100.0,
        }
    }
}

impl CameraMovement {
    fn translation(&self) -> Vec3 {
        let mut translation = Vec3::default();
        if self.up && !self.down {
            translation += Vec3::Y;
        } else if self.down && !self.up {
            translation -= Vec3::Y;
        }
        if self.left && !self.right {
            translation -= Vec3::X;
        } else if self.right && !self.left {
            translation += Vec3::X;
        }
        let velocity = self.min_velocity + self.timer.fraction().cubed() * self.max_addtition;
        let velocity = translation.clamp_length(velocity, velocity);
        velocity
    }

    fn moving(&self) -> bool {
        (self.up ^ self.down) || (self.left ^ self.right)
    }
}

fn init(mut commands: Commands) {
    commands.spawn((Camera2d, CameraMovement::default()));
}

fn update_movement(
    mut query: Query<&mut CameraMovement>,
    time: Res<Time>,
    mut moving: Local<bool>,
) {
    for mut movement in &mut query {
        if !*moving && movement.moving() {
            movement.timer.reset();
        }
        *moving = movement.moving();
        if *moving {
            movement.timer.tick(time.delta());
        }
    }
}

fn apply_movement(mut query: Query<(&mut Transform, &CameraMovement)>) {
    for (mut transform, movement) in &mut query {
        if movement.moving() {
            transform.translation += movement.translation();
        }
    }
}
