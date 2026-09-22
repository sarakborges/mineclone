use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState, settings_state::SettingsState},
    hud::chat::ChatState,
    player::{
        character_info::{CharacterInfoInputState, CharacterInfoState},
        inventory::InventoryState,
    },
    tools::BrushPaletteState,
    ui::transition::{ScreenTransition, ScreenTransitionTarget},
};

pub struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            toggle_pause
                .run_if(in_state(GameState::Gameplay))
                .run_if(in_state(SettingsState::Closed))
                .run_if(in_state(InventoryState::Closed))
                .run_if(in_state(BrushPaletteState::Closed))
                .run_if(in_state(CharacterInfoState::Closed)),
        );
    }
}

fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    pause_state: Res<State<PauseState>>,
    chat: Res<ChatState>,
    character_info_input: Res<CharacterInfoInputState>,
    mut transition: ResMut<ScreenTransition>,
) {
    if !keys.just_pressed(KeyCode::Escape)
        || chat.blocks_pause_escape()
        || character_info_input.blocks_pause_escape()
    {
        return;
    }

    let next = match pause_state.get() {
        PauseState::Running => PauseState::Paused,
        PauseState::Paused => PauseState::Running,
    };

    transition.request(ScreenTransitionTarget::pause(next));
}

