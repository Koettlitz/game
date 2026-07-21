use bevy::{input::mouse::MouseWheel, math::FloatPow, prelude::*};

const MIN_ZOOM: f32 = 0.08;
const MAX_ZOOM: f32 = 10.0;
const ZOOM_SPEED: f32 = 0.1;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init)
            .add_systems(Update, (update_movement, apply_movement, scroll).chain());
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
    max_addition: f32,
}

impl Default for CameraMovement {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(4.0, TimerMode::Once),
            up: false,
            left: false,
            right: false,
            down: false,
            min_velocity: 16.0,
            max_addition: 100.0,
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
        let velocity = self.min_velocity + self.max_addition * self.timer.fraction().cubed();
        translation.clamp_length(velocity, velocity)
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

fn scroll(
    mut mouse_wheel_reader: MessageReader<MouseWheel>,
    camera: Single<&mut Projection, With<Camera2d>>,
) {
    let Projection::Orthographic(ref mut projection) = *camera.into_inner() else {
        error!("scrolling not implemented for non orthographic projection");
        return;
    };
    for msg in mouse_wheel_reader.read() {
        projection.scale = (projection.scale - msg.y * ZOOM_SPEED).clamp(MIN_ZOOM, MAX_ZOOM);
    }
}
