use bevy::{
    ecs::system::SystemParam,
    input_focus::InputFocus,
    prelude::*,
    text::EditableText,
};

use crate::{
    app::{
        game_state::GameState,
        keybinds::{KeybindAction, Keybinds},
        pause_state::PauseState,
    },
    gameplay::modal::GameplayModalState,
    hud::chat::ChatState,
};

pub(crate) struct PlayerCharacterInfoPlugin;

impl Plugin for PlayerCharacterInfoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            toggle_character_info
                .run_if(in_state(GameState::Gameplay))
                .run_if(in_state(PauseState::Running)),
        );
    }
}

#[derive(SystemParam)]
struct CharacterInfoModalInput<'w, 's> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    keybinds: Res<'w, Keybinds>,
    modal: Res<'w, State<GameplayModalState>>,
    chat: Res<'w, ChatState>,
    focus: ResMut<'w, InputFocus>,
    editable_text: Query<'w, 's, (), With<EditableText>>,
    next_modal: ResMut<'w, NextState<GameplayModalState>>,
}

fn toggle_character_info(mut input: CharacterInfoModalInput) {
    let typing = input
        .focus
        .get()
        .is_some_and(|entity| input.editable_text.get(entity).is_ok());
    if input.chat.is_open() || typing {
        return;
    }

    if !input
        .keys
        .just_pressed(input.keybinds.key_code(KeybindAction::Inventory))
    {
        return;
    }

    let next = if *input.modal.get() == GameplayModalState::CharacterInfo {
        GameplayModalState::Closed
    } else {
        GameplayModalState::CharacterInfo
    };
    input.next_modal.set(next);
}
