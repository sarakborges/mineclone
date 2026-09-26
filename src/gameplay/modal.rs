use bevy::{input_focus::InputFocus, prelude::*};

use crate::{
    app::{
        game_state::GameState,
        pause_state::PauseState,
        state_systems::reset_next_state,
    },
};

#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) enum GameplayModalState {
    #[default]
    Closed,
    Inventory,
    CharacterInfo,
    BrushPalette,
}

impl GameplayModalState {
    pub(crate) const fn is_open(self) -> bool {
        !matches!(self, Self::Closed)
    }

    pub(crate) const fn shows_inventory(self) -> bool {
        matches!(self, Self::Inventory | Self::CharacterInfo)
    }
}

pub(super) struct GameplayModalPlugin;

impl Plugin for GameplayModalPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameplayModalState>()
            .add_systems(
                Update,
                close_modal_on_escape
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running)),
            )
            .add_systems(
                OnEnter(PauseState::Paused),
                reset_next_state::<GameplayModalState>.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnExit(GameState::Gameplay),
                reset_next_state::<GameplayModalState>,
            );
    }
}

fn close_modal_on_escape(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameplayModalState>>,
    mut next_state: ResMut<NextState<GameplayModalState>>,
    mut focus: ResMut<InputFocus>,
) {
    if !state.get().is_open() || !keys.just_pressed(KeyCode::Escape) {
        return;
    }

    focus.clear();
    next_state.set(GameplayModalState::Closed);
}
