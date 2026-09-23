use bevy::prelude::*;

use crate::app::{
    game_state::GameState, resource_systems::reset_resource, settings_state::SettingsState,
};
use crate::localization::ActiveLanguage;
use game_rules_section::{
    TicksPerSecondInputState, handle_boolean_game_rule_toggles, handle_ticks_input,
    handle_ticks_keyboard, handle_ticks_step_buttons, sync_boolean_game_rule_toggles,
    sync_ticks_per_second_text,
};
use biome_size_multiplier_section::{
    BiomeSizeMultiplierInputState, handle_biome_size_multiplier_input,
    handle_biome_size_multiplier_keyboard, sync_biome_size_multiplier_input,
    sync_biome_size_multiplier_slider_thumb, sync_biome_size_multiplier_visibility,
};
use hud_section::{
    TargetBlockPositionDropdownState, close_target_block_position_dropdown_outside,
    handle_hide_hints_toggle, handle_hint_toggles, handle_target_block_position_dropdown_button,
    handle_target_block_position_options, sync_hide_hints_toggle, sync_hint_toggles,
    sync_target_block_position_dropdown, sync_target_block_position_options,
};
use languages_section::{
    LanguageDropdownState, close_language_dropdown_outside, handle_language_dropdown_button,
    handle_language_options, sync_language_dropdown,
};
use layout::spawn_settings_screen;
use keybinds_section::{
    KeybindCaptureState, handle_keybind_buttons, handle_keybind_capture, sync_keybinds_section,
};
use navigation::{
    SettingsSectionSelection, apply_pending_section_scroll, handle_close_requests,
    handle_section_buttons, sync_section_ui,
};
use new_world_section::{
    SeedInputState, handle_new_world_footer, handle_new_world_settings_control_focus,
    handle_random_seed, handle_seed_focus, handle_seed_keyboard,
    handle_single_biome_toggle, handle_spawn_structures_toggle,
    handle_world_generation_feature_toggles, handle_world_generation_mode_buttons,
    reset_new_world_settings, sync_seed_text, sync_world_generation_feature_toggles,
    sync_world_generation_mode_buttons, sync_world_generation_toggles,
};
use render_distance_logic::{
    handle_render_distance_input, handle_render_distance_keyboard,
    sync_render_distance_input, sync_render_distance_slider_thumb, sync_render_distance_text,
};
use render_distance_section::{RenderDistanceInput, RenderDistanceInputState};
use spawn_biome_section::{
    SpawnBiomeDropdownState, close_spawn_biome_dropdown_outside,
    focus_spawn_biome_search_frame, handle_spawn_biome_dropdown_button,
    handle_spawn_biome_option_buttons, handle_spawn_biome_search_focus,
    handle_spawn_biome_search_keyboard, populate_spawn_biome_options,
    sync_spawn_biome_dropdown_state, sync_spawn_biome_option_labels,
    sync_spawn_biome_options, sync_spawn_biome_search_frame,
    sync_spawn_biome_selected_label,
};
use world_name_section::{WorldNameFeedback, handle_world_name_focus, sync_world_name_view};
use world_settings_section::{handle_game_mode_buttons, sync_game_mode_buttons};

pub(crate) mod game_rules_section;
mod biome_size_multiplier_section;
mod hud_section;
mod keybinds_section;
mod languages_section;
mod layout;
mod navigation;
mod new_world_section;
mod render_distance_logic;
mod render_distance_section;
mod spawn_biome_section;
mod world_name_section;
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
            .init_resource::<crate::app::settings_state::SettingsScreenMode>()
            .init_resource::<SettingsSectionSelection>()
            .init_resource::<TicksPerSecondInputState>()
            .init_resource::<RenderDistanceInputState>()
            .init_resource::<BiomeSizeMultiplierInputState>()
            .init_resource::<SeedInputState>()
            .init_resource::<SpawnBiomeDropdownState>()
            .init_resource::<WorldNameFeedback>()
            .init_resource::<TargetBlockPositionDropdownState>()
            .init_resource::<LanguageDropdownState>()
            .init_resource::<KeybindCaptureState>()
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
                    reset_resource::<RenderDistanceInputState>,
                    reset_resource::<TargetBlockPositionDropdownState>,
                    reset_resource::<LanguageDropdownState>,
                    reset_resource::<KeybindCaptureState>,
                    spawn_settings_screen,
                )
                    .chain(),
            )
            .add_systems(
                OnEnter(GameState::NewWorld),
                (
                    reset_resource::<RenderDistanceInputState>,
                    reset_resource::<TargetBlockPositionDropdownState>,
                    reset_resource::<LanguageDropdownState>,
                    reset_new_world_settings,
                    spawn_settings_screen,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    (
                        (
                            apply_pending_section_scroll,
                            handle_section_buttons,
                            close_spawn_biome_dropdown_outside,
                            close_target_block_position_dropdown_outside,
                            close_language_dropdown_outside,
                            handle_seed_focus,
                            handle_biome_size_multiplier_input
                                .run_if(in_state(GameState::NewWorld)),
                            handle_render_distance_input,
                            handle_world_name_focus.run_if(in_state(GameState::NewWorld)),
                            handle_new_world_settings_control_focus
                                .run_if(in_state(GameState::NewWorld)),
                            handle_random_seed,
                        )
                            .chain(),
                        (
                            handle_spawn_biome_dropdown_button
                                .run_if(in_state(GameState::NewWorld)),
                            handle_spawn_biome_search_focus.run_if(in_state(GameState::NewWorld)),
                            focus_spawn_biome_search_frame.run_if(in_state(GameState::NewWorld)),
                            handle_spawn_biome_option_buttons.run_if(in_state(GameState::NewWorld)),
                            handle_game_mode_buttons,
                            handle_world_generation_mode_buttons
                                .run_if(in_state(GameState::NewWorld)),
                            handle_spawn_structures_toggle.run_if(in_state(GameState::NewWorld)),
                            handle_single_biome_toggle.run_if(in_state(GameState::NewWorld)),
                            handle_world_generation_feature_toggles
                                .run_if(in_state(GameState::NewWorld)),
                            handle_keybind_buttons,
                            handle_keybind_capture,
                        )
                            .chain(),
                    )
                        .chain(),
                    (
                        handle_ticks_step_buttons,
                        handle_boolean_game_rule_toggles,
                        handle_ticks_input,
                        handle_new_world_footer,
                        handle_spawn_biome_search_keyboard.run_if(in_state(GameState::NewWorld)),
                        handle_seed_keyboard,
                        handle_biome_size_multiplier_keyboard
                            .run_if(in_state(GameState::NewWorld)),
                        handle_render_distance_keyboard.run_if(has_render_distance_input),
                        handle_ticks_keyboard.run_if(has_ticks_input),
                        handle_language_dropdown_button,
                        handle_language_options,
                        handle_hide_hints_toggle,
                        handle_hint_toggles,
                        handle_target_block_position_dropdown_button,
                        handle_target_block_position_options,
                    )
                        .chain(),
                )
                    .chain()
                    .in_set(SettingsScreenSet::Input),
            )
            .add_systems(
                Update,
                (
                    (
                        populate_spawn_biome_options,
                        sync_section_ui,
                        sync_game_mode_buttons,
                        sync_world_generation_mode_buttons.run_if(in_state(GameState::NewWorld)),
                        sync_world_generation_toggles.run_if(in_state(GameState::NewWorld)),
                        sync_world_generation_feature_toggles
                            .run_if(in_state(GameState::NewWorld)),
                        sync_keybinds_section,
                        sync_language_dropdown,
                        sync_hide_hints_toggle,
                        sync_hint_toggles,
                        sync_target_block_position_dropdown,
                        sync_target_block_position_options,
                        sync_spawn_biome_dropdown_state.run_if(in_state(GameState::NewWorld)),
                    )
                        .chain(),
                    (
                        sync_spawn_biome_search_frame.run_if(in_state(GameState::NewWorld)),
                        sync_spawn_biome_selected_label,
                        sync_spawn_biome_option_labels,
                        sync_spawn_biome_options.run_if(in_state(GameState::NewWorld)),
                        sync_world_name_view.run_if(in_state(GameState::NewWorld)),
                        sync_seed_text,
                        sync_biome_size_multiplier_visibility.run_if(in_state(GameState::NewWorld)),
                        sync_biome_size_multiplier_input.run_if(in_state(GameState::NewWorld)),
                        sync_biome_size_multiplier_slider_thumb
                            .run_if(in_state(GameState::NewWorld)),
                        sync_ticks_per_second_text,
                        sync_boolean_game_rule_toggles,
                        sync_render_distance_text,
                        sync_render_distance_input,
                        sync_render_distance_slider_thumb,
                    )
                        .chain(),
                )
                    .chain()
                    .in_set(SettingsScreenSet::Sync),
            )
            // Some labels and descriptions are captured as plain Text bundles
            // during UI construction. Rebuild only on a real language change,
            // after input/sync, preserving the selected section and settings.
            .add_systems(
                Update,
                spawn_settings_screen
                    .run_if(resource_changed::<ActiveLanguage>)
                    .run_if(in_state(SettingsState::Open))
                    .after(SettingsScreenSet::Sync),
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

fn has_ticks_input(inputs: Query<(), With<game_rules_section::TicksPerSecondInput>>) -> bool {
    !inputs.is_empty()
}

fn has_render_distance_input(inputs: Query<(), With<RenderDistanceInput>>) -> bool {
    !inputs.is_empty()
}
