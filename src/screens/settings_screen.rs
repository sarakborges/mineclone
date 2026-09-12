use bevy::prelude::*;

use crate::app::{game_state::GameState, settings_state::SettingsState};
use game_rules_section::{
    TicksPerSecondInputState, handle_ticks_input, handle_ticks_keyboard,
    handle_ticks_step_buttons, sync_ticks_per_second_text,
};
use languages_section::{handle_language_buttons, sync_language_buttons};
use layout::spawn_settings_screen;
use miscellaneous_section::{handle_display_tooltips_toggle, sync_display_tooltips_toggle};
use navigation::{
    SettingsSectionSelection, handle_close_requests, handle_section_buttons, sync_section_ui,
};
use new_world_section::{
    SeedInputState, handle_new_world_footer, handle_random_seed, handle_seed_focus,
    handle_seed_keyboard, reset_new_world_settings, sync_seed_text,
};
use render_distance_logic::{sync_render_distance_text, sync_slider_thumb};
use world_settings_section::{handle_game_mode_buttons, sync_game_mode_buttons};

pub(crate) mod game_rules_section;
mod languages_section;
mod layout;
mod miscellaneous_section;
mod navigation;
mod new_world_section;
mod render_distance_logic;
mod render_distance_section;
pub(crate) mod world_settings_section;

pub struct SettingsScreenPlugin;

impl Plugin for SettingsScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SettingsState>()
            .init_resource::<SettingsSectionSelection>()
            .init_resource::<TicksPerSecondInputState>()
            .init_resource::<SeedInputState>()
            .add_systems(
                OnEnter(SettingsState::Open),
                (reset_regular_settings_inputs, spawn_settings_screen).chain(),
            )
            .add_systems(
                OnEnter(GameState::NewWorld),
                (reset_new_world_settings, spawn_settings_screen).chain(),
            )
            .add_systems(
                Update,
                (
                    handle_section_buttons,
                    handle_seed_focus,
                    handle_random_seed,
                    handle_game_mode_buttons,
                    handle_ticks_step_buttons,
                    handle_ticks_input,
                    handle_new_world_footer,
                    handle_seed_keyboard,
                    handle_ticks_keyboard,
                    handle_language_buttons,
                    handle_display_tooltips_toggle,
                    sync_section_ui,
                    sync_game_mode_buttons,
                    sync_language_buttons,
                    sync_display_tooltips_toggle,
                    sync_seed_text,
                    sync_ticks_per_second_text,
                    sync_render_distance_text,
                    sync_slider_thumb,
                )
                    .chain()
                    .run_if(settings_screen_active),
            )
            .add_systems(
                Update,
                handle_close_requests.run_if(in_state(SettingsState::Open)),
            );
    }
}

fn reset_regular_settings_inputs(mut ticks: ResMut<TicksPerSecondInputState>) {
    *ticks = TicksPerSecondInputState::default();
}

fn settings_screen_active(
    settings_state: Res<State<SettingsState>>,
    game_state: Res<State<GameState>>,
) -> bool {
    *settings_state.get() == SettingsState::Open || *game_state.get() == GameState::NewWorld
}
