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
    player::game_mode::GameMode,
};

pub(crate) struct PlayerCharacterInfoPlugin;

impl Plugin for PlayerCharacterInfoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            toggle_inventory_screen
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
    focus: Res<'w, InputFocus>,
    editable_text: Query<'w, 's, (), With<EditableText>>,
    next_modal: ResMut<'w, NextState<GameplayModalState>>,
}

fn toggle_inventory_screen(
    mut input: CharacterInfoModalInput,
    game_mode: Single<&GameMode>,
) {
    if game_mode.is_spectator() {
        return;
    }

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

    let target = inventory_modal_for_mode(**game_mode);
    let next = if *input.modal.get() == target {
        GameplayModalState::Closed
    } else {
        target
    };
    input.next_modal.set(next);
}

const fn inventory_modal_for_mode(game_mode: GameMode) -> GameplayModalState {
    match game_mode {
        GameMode::Survival => GameplayModalState::Inventory,
        GameMode::Creative => GameplayModalState::CharacterInfo,
        GameMode::Spectator => GameplayModalState::Closed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn survival_inventory_action_opens_crafting_inventory() {
        assert_eq!(
            inventory_modal_for_mode(GameMode::Survival),
            GameplayModalState::Inventory
        );
    }

    #[test]
    fn creative_inventory_action_keeps_character_info_inventory() {
        assert_eq!(
            inventory_modal_for_mode(GameMode::Creative),
            GameplayModalState::CharacterInfo
        );
    }
}
