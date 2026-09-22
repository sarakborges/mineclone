use bevy::prelude::*;

use crate::{
    app::{
        game_state::GameState,
        keybinds::{KeybindAction, Keybinds},
        pause_state::PauseState,
        state_systems::reset_next_state,
    },
    hud::chat::ChatState,
    player::inventory::InventoryState,
    tools::BrushPaletteState,
};

#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) enum CharacterInfoState {
    #[default]
    Closed,
    Open,
}

#[derive(Resource, Default)]
pub(crate) struct CharacterInfoInputState {
    escape_consumed: bool,
}

impl CharacterInfoInputState {
    pub(crate) fn blocks_pause_escape(&self) -> bool {
        self.escape_consumed
    }
}

pub(crate) struct PlayerCharacterInfoPlugin;

impl Plugin for PlayerCharacterInfoPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<CharacterInfoState>()
            .init_resource::<CharacterInfoInputState>()
            .add_systems(
                Update,
                toggle_character_info
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running))
                    .run_if(in_state(InventoryState::Closed))
                    .run_if(in_state(BrushPaletteState::Closed)),
            )
            .add_systems(
                OnEnter(PauseState::Paused),
                reset_next_state::<CharacterInfoState>.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnExit(GameState::Gameplay),
                reset_next_state::<CharacterInfoState>,
            );
    }
}

fn toggle_character_info(
    keys: Res<ButtonInput<KeyCode>>,
    keybinds: Res<Keybinds>,
    state: Res<State<CharacterInfoState>>,
    chat: Res<ChatState>,
    mut input_state: ResMut<CharacterInfoInputState>,
    mut next_state: ResMut<NextState<CharacterInfoState>>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        input_state.escape_consumed = false;
    }
    if chat.is_open() {
        return;
    }

    let toggle_pressed = keys.just_pressed(keybinds.key_code(KeybindAction::CharacterInfo));
    match state.get() {
        CharacterInfoState::Closed if toggle_pressed => {
            next_state.set(CharacterInfoState::Open);
        }
        CharacterInfoState::Open if toggle_pressed || keys.just_pressed(KeyCode::Escape) => {
            if keys.just_pressed(KeyCode::Escape) {
                input_state.escape_consumed = true;
            }
            next_state.set(CharacterInfoState::Closed);
        }
        _ => {}
    }
}
