use bevy::{prelude::*, window::WindowFocused};

use crate::{
    app::{
        controls_state::ControlsState, game_state::GameState, pause_state::PauseState,
        settings_state::SettingsState,
    },
    gameplay::modal::GameplayModalState,
    hud::chat::ChatState,
    ui::transition::{ScreenTransition, ScreenTransitionTarget},
};

pub struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                toggle_pause
                    .run_if(in_state(SettingsState::Closed))
                    .run_if(in_state(ControlsState::Closed))
                    .run_if(in_state(GameplayModalState::Closed)),
                pause_on_focus_lost
                    .run_if(in_state(SettingsState::Closed))
                    .run_if(in_state(ControlsState::Closed))
                    .run_if(in_state(GameplayModalState::Closed)),
            )
                .run_if(in_state(GameState::Gameplay)),
        );
    }
}

fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    pause_state: Res<State<PauseState>>,
    chat: Res<ChatState>,
    mut transition: ResMut<ScreenTransition>,
) {
    if !keys.just_pressed(KeyCode::Escape) || chat.blocks_pause_escape() {
        return;
    }

    let next = match pause_state.get() {
        PauseState::Running => PauseState::Paused,
        PauseState::Paused => PauseState::Running,
    };

    transition.request(ScreenTransitionTarget::pause(next));
}

fn pause_on_focus_lost(
    mut focused_events: MessageReader<WindowFocused>,
    pause_state: Res<State<PauseState>>,
    chat: Res<ChatState>,
    mut next_pause_state: ResMut<NextState<PauseState>>,
) {
    let lost_focus = focused_events.read().any(|event| !event.focused);
    if *pause_state.get() == PauseState::Paused || chat.is_open() || !lost_focus {
        return;
    }
    next_pause_state.set(PauseState::Paused);
}
