use bevy::prelude::*;
use engine::assets::overworld::lozo::LozoAsset;

include!(concat!(env!("OUT_DIR"), "/lozo.rs"));

pub struct LozoPlugin;
impl Plugin for LozoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NextLozo>()
            .init_state::<LozoState>()
            .add_systems(
                FixedPreUpdate,
                detect_lozo_transition.run_if(resource_changed::<NextLozo>),
            );
    }
}

#[derive(Resource, Default)]
pub struct NextLozo {
    target: Option<Lozo>,
}

impl NextLozo {
    pub fn set(&mut self, target: Lozo) {
        self.target = Some(target);
    }
}

#[derive(Resource)]
struct LozoTransition {
    target: Lozo,
    asset_handle: Handle<LozoAsset>,
}

fn detect_lozo_transition(
    mut transition: ResMut<NextLozo>,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<LozoState>>,
    mut commands: Commands,
) {
    let Some(ref target) = transition.target else {
        return;
    };

    commands.insert_resource(LozoTransition {
        target: *target,
        asset_handle: asset_server.load(target.asset_path()),
    });
    next_state.set(LozoState::LoadingLozoAsset);

    transition.target = None;
}

#[derive(States, Default, PartialEq, Eq, Hash, Clone, Copy, Debug)]
enum LozoState {
    #[default]
    Default,
    LoadingLozoAsset,
    LoadingDependencies,
}
