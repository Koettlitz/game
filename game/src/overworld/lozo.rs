use bevy::prelude::*;
use engine::assets::AssetResolver;
use engine::assets::overworld::lozo::LozoAsset;

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
pub struct NextLozo(Option<NextLozoRequest>);

#[derive(Clone)]
struct NextLozoRequest {
    target: String,
    player_location: UVec2,
}

impl NextLozo {
    pub fn set(&mut self, target: String, player_location: UVec2) {
        self.0 = Some(NextLozoRequest {
            target,
            player_location,
        });
    }
}

#[derive(Resource)]
struct LozoTransition {
    next_lozo: NextLozoRequest,
    asset_handle: Handle<LozoAsset>,
}

fn detect_lozo_transition(
    mut transition: ResMut<NextLozo>,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<LozoState>>,
    mut commands: Commands,
) {
    let Some(next_lozo) = transition.0.take() else {
        return;
    };

    let asset_path = LozoAsset::resolve(&next_lozo.target).unwrap();
    commands.insert_resource(LozoTransition {
        next_lozo: next_lozo,
        asset_handle: asset_server.load(asset_path),
    });
    next_state.set(LozoState::LoadingLozoAsset);
}

#[derive(States, Default, PartialEq, Eq, Hash, Clone, Copy, Debug)]
enum LozoState {
    #[default]
    Default,
    LoadingLozoAsset,
}
