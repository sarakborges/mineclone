use bevy::prelude::*;

use crate::app::{
    game_state::GameState, resource_systems::reset_resource, settings_state::SettingsState,
};
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
use spawn_biome_section::{
    SpawnBiomeDropdownState, close_spawn_biome_dropdown_outside_general,
    handle_spawn_biome_dropdown_button, handle_spawn_biome_option_buttons,
    handle_spawn_biome_search_focus, handle_spawn_biome_search_keyboard,
    populate_spawn_biome_options, sync_spawn_biome_dropdown_view,
};
use world_settings_section::{handle_game_mode_buttons, sync_game_mode_buttons};

pub(crate) mod game_rules_section;
mod languages_section;
mod layout;
mod miscellaneous_section;
mod navigation;
mod new_world_section;
mod render_distance_logic;
mod render_distance_section;
mod spawn_biome_section;
pub(crate) mod world_settings_section;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SettingsScreenSet {
    Input,
    Sync,
}

pub struct SettingsScreenPlugin;

impl Plugin for SettingsScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SettingsState>()
            .init_resource::<SettingsSectionSelection>()
            .init_resource::<TicksPerSecondInputState>()
            .init_resource::<SeedInputState>()
            .init_resource::<SpawnBiomeDropdownState>()
            .configure_sets(
                Update,
                (SettingsScreenSet::Input, SettingsScreenSet::Sync)
                    .chain()
                    .run_if(settings_screen_active),
            )
            .add_systems(
                OnEnter(SettingsState::Open),
                (
                    reset_resource::<TicksPerSecondInputState>,
                    spawn_settings_screen,
                )
                    .chain(),
            )
            .add_systems(
                OnEnter(GameState::NewWorld),
                (reset_new_world_settings, spawn_settings_screen).chain(),
            )
            .add_systems(
                Update,
                (
                    handle_section_buttons,
                    close_spawn_biome_dropdown_outside_general,
                    handle_seed_focus,
                    handle_random_seed,
                    handle_spawn_biome_dropdown_button,
                    handle_spawn_biome_search_focus,
                    handle_spawn_biome_option_buttons,
                    handle_game_mode_buttons,
                    handle_ticks_step_buttons,
                    handle_ticks_input,
                    handle_new_world_footer,
                    handle_spawn_biome_search_keyboard,
                    handle_seed_keyboard,
                    handle_ticks_keyboard,
                    handle_language_buttons,
                    handle_display_tooltips_toggle,
                )
                    .chain()
                    .in_set(SettingsScreenSet::Input),
            )
            .add_systems(
                Update,
                (
                    populate_spawn_biome_options,
                    sync_section_ui,
                    sync_game_mode_buttons,
                    sync_language_buttons,
                    sync_display_tooltips_toggle,
                    sync_spawn_biome_dropdown_view,
                    sync_seed_text,
                    sync_ticks_per_second_text,
                    sync_render_distance_text,
                    sync_slider_thumb,
                )
                    .chain()
                    .in_set(SettingsScreenSet::Sync),
            )
            .add_systems(
                Update,
                handle_close_requests
                    .in_set(SettingsScreenSet::Input)
                    .run_if(in_state(SettingsState::Open)),
            );
    }
}

fn settings_screen_active(
    settings_state: Res<State<SettingsState>>,
    game_state: Res<State<GameState>>,
) -> bool {
    *settings_state.get() == SettingsState::Open || *game_state.get() == GameState::NewWorld
}
