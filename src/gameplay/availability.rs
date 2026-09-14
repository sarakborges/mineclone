use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::{game_state::GameState, pause_state::PauseState, settings_state::SettingsState},
    player::inventory::InventoryState,
    tools::BrushPaletteState,
};

#[derive(SystemParam)]
pub(crate) struct WorldInteractionState<'w> {
    game: Res<'w, State<GameState>>,
    pause: Res<'w, State<PauseState>>,
    settings: Res<'w, State<SettingsState>>,
    inventory: Res<'w, State<InventoryState>>,
    brush_palette: Res<'w, State<BrushPaletteState>>,
}

impl WorldInteractionState<'_> {
    pub(crate) fn available(&self) -> bool {
        *self.game.get() == GameState::Gameplay
            && *self.pause.get() == PauseState::Running
            && *self.settings.get() == SettingsState::Closed
            && *self.inventory.get() == InventoryState::Closed
            && *self.brush_palette.get() == BrushPaletteState::Closed
    }
}

pub(crate) fn world_interaction_available(state: WorldInteractionState) -> bool {
    state.available()
}
