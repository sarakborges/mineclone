use bevy::{prelude::*, time::Virtual};

use crate::app::{game_state::GameState, pause_state::PauseState};

pub struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            toggle_pause.run_if(in_state(GameState::Gameplay)),
        )
        .add_systems(OnEnter(PauseState::Paused), pause_time)
        .add_systems(OnEnter(PauseState::Running), resume_time);
    }
}

fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    pause_state: Res<State<PauseState>>,
    mut next_pause_state: ResMut<NextState<PauseState>>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }

    let next = match pause_state.get() {
        PauseState::Running => PauseState::Paused,
        PauseState::Paused => PauseState::Running,
    };

    next_pause_state.set(next);
}

fn pause_time(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}

fn resume_time(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}
