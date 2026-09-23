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
                    .run_if(in_state(PauseState::Running)),
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

#[derive(SystemParam)]
struct CharacterInfoModalInput<'w, 's> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    keybinds: Res<'w, Keybinds>,
    state: Res<'w, State<CharacterInfoState>>,
    chat: Res<'w, ChatState>,
    focus: Res<'w, InputFocus>,
    editable_text: Query<'w, 's, (), With<EditableText>>,
    input_state: ResMut<'w, CharacterInfoInputState>,
    next_state: ResMut<'w, NextState<CharacterInfoState>>,
    next_inventory: ResMut<'w, NextState<InventoryState>>,
    next_brush_palette: ResMut<'w, NextState<BrushPaletteState>>,
}

fn toggle_character_info(mut input: CharacterInfoModalInput) {
    if !input.keys.just_pressed(KeyCode::Escape) {
        input.input_state.escape_consumed = false;
    }
    let typing = input
        .focus
        .get()
        .is_some_and(|entity| input.editable_text.get(entity).is_ok());
    if input.chat.is_open() || typing {
        return;
    }

    let toggle_pressed = input
        .keys
        .just_pressed(input.keybinds.key_code(KeybindAction::CharacterInfo));
    match input.state.get() {
        CharacterInfoState::Closed if toggle_pressed => {
            input.next_inventory.set(InventoryState::Closed);
            input.next_brush_palette.set(BrushPaletteState::Closed);
            input.next_state.set(CharacterInfoState::Open);
        }
        CharacterInfoState::Open
            if toggle_pressed || input.keys.just_pressed(KeyCode::Escape) =>
        {
            if input.keys.just_pressed(KeyCode::Escape) {
                input.input_state.escape_consumed = true;
            }
            input.next_state.set(CharacterInfoState::Closed);
        }
        _ => {}
    }
}
