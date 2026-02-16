use std::{iter::Sum, ops};

use bevy::prelude::*;

pub struct ProgressPlugin;
impl Plugin for ProgressPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<ProgressState>()
            .add_observer(init_progress_screen)
            .add_observer(init_progress_panel)
            .add_systems(
                PostUpdate,
                (update_progress_panels, update_progress_screen)
                    .run_if(in_state(ProgressState::Loading)),
            )
            .add_systems(
                PostUpdate,
                check_finished.run_if(in_state(ProgressState::Loading)),
            )
            .add_systems(OnEnter(ProgressState::Finished), cleanup);
    }
}

fn init_progress_screen(event: On<Add, ProgressScreen>, mut commands: Commands) {
    commands
        .entity(event.entity)
        .insert(Text("Loading... 0%".to_string()));
}

fn init_progress_panel(
    event: On<Add, ProgressPanel>,
    panels: Query<&ProgressPanel>,
    mut commands: Commands,
    mut counter: Local<usize>,
) {
    if *counter == 0 {
        *counter = 2;
    }
    let panel = panels
        .get(event.entity)
        .expect("added entity not queryable?");
    let y_pos = *counter * 64;
    let text = format!("{}... 0%", panel.name);
    commands.entity(event.entity).insert((
        Text(text),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(y_pos as f32),
            ..Default::default()
        },
    ));
    *counter = *counter + 1;
}

fn update_progress_panels(
    mut progresses: Query<(&Progress, &ProgressPanel, &mut Text), Changed<Progress>>,
) {
    for (progress, panel, mut text) in progresses.iter_mut() {
        let percent = progress.get_relative() * 100.0;
        text.0 = format!("{}... {}%", panel.name, percent as usize);
    }
}

fn update_progress_screen(
    changed: Query<(), Changed<Progress>>,
    progresses: Query<&Progress>,
    mut progress_screens: Query<&mut Text, With<ProgressScreen>>,
) {
    if changed.is_empty() {
        return;
    }
    let progress: Progress = progresses.iter().sum();
    for mut text in progress_screens.iter_mut() {
        let percent = progress.get_relative() * 100.0;
        text.0 = format!("Loading... {}%", percent as usize);
    }
}

fn check_finished(progress: Query<&Progress>, mut next_state: ResMut<NextState<ProgressState>>) {
    let progress: Progress = progress.iter().sum();
    if progress.is_finished() {
        next_state.set(ProgressState::Finished);
    }
}

fn cleanup(mut commands: Commands, progress_panels: Query<Entity, With<ProgressPanel>>) {
    for panel in progress_panels.iter() {
        commands.entity(panel).despawn();
    }
}

#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum ProgressState {
    #[default]
    Loading,
    Finished,
}

#[derive(Component)]
pub struct ProgressScreen;
#[derive(Component)]
#[require(Progress)]
pub struct ProgressPanel {
    pub name: String,
    pub unit: Option<String>,
}

impl ProgressPanel {
    pub fn new(name: String) -> Self {
        Self { name, unit: None }
    }
}

#[derive(Component)]
pub struct Progress {
    current: usize,
    max: usize,
}

impl Default for Progress {
    fn default() -> Self {
        Self::new(0, 100)
    }
}

impl Progress {
    pub fn new(min: usize, max: usize) -> Self {
        if max < min {
            panic!("max cannot be less than min");
        }
        Self { current: min, max }
    }

    pub fn add(&mut self, progress: usize) {
        self.current = std::cmp::min(self.current + progress, self.max);
    }

    pub fn max(&self) -> usize {
        self.max
    }

    pub fn get(&self) -> usize {
        self.current
    }

    pub fn get_relative(&self) -> f32 {
        self.current as f32 / self.max as f32
    }

    pub fn is_finished(&self) -> bool {
        self.current >= self.max
    }
}

impl ops::AddAssign<usize> for Progress {
    fn add_assign(&mut self, rhs: usize) {
        self.add(rhs);
    }
}

impl ops::Add for &Progress {
    type Output = Progress;
    fn add(self, rhs: Self) -> Self::Output {
        Progress {
            current: self.current + rhs.current,
            max: self.max + rhs.max,
        }
    }
}

impl ops::AddAssign<&Self> for Progress {
    fn add_assign(&mut self, rhs: &Self) {
        self.current += rhs.current;
        self.max += rhs.max;
    }
}

impl<'a> Sum<&'a Self> for Progress {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        let mut sum = Self { current: 0, max: 0 };
        for p in iter {
            sum += p;
        }
        sum
    }
}
