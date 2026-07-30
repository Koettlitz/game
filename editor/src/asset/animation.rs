use std::ops::Deref;

use bevy::prelude::*;
use engine::animation::AnimationTimersAsset;
use engine::progress::{Progress, ProgressPanel, ProgressState};

pub struct SpriteAnimationPlugin;
impl Plugin for SpriteAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_timers).add_systems(
            Update,
            check_progress.run_if(in_state(ProgressState::Loading)),
        );
    }
}

fn load_timers(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Progress::new(0, 1),
        ProgressPanel::new("Animation timers".to_string()),
        TimerProgress,
    ));
    let handle = asset_server.load("animation_timers.ron");
    commands.insert_resource(AnimationTimers(handle));
}

fn check_progress(
    asset_server: Res<AssetServer>,
    mut progress: Single<&mut Progress, With<TimerProgress>>,
    animation_timers: Res<AnimationTimers>,
) {
    if asset_server.is_loaded(animation_timers.id()) {
        progress.add(1);
    }
}

#[derive(Resource)]
pub struct AnimationTimers(Handle<AnimationTimersAsset>);

impl Deref for AnimationTimers {
    type Target = Handle<AnimationTimersAsset>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Component)]
struct TimerProgress;
