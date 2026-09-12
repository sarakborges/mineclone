use bevy::prelude::*;

use crate::app::settings_state::SettingsState;
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
use render_distance_logic::{sync_render_distance_text, sync_slider_thumb};
use world_settings_section::{handle_game_mode_buttons, sync_game_mode_buttons};

mod game_rules_section;
mod languages_section;
mod layout;
mod miscellaneous_section;
mod navigation;
mod render_distance_logic;
mod render_distance_section;
mod scroll_area;
mod world_settings_section;

pub struct SettingsScreenPlugin;

impl Plugin for SettingsScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SettingsState>()
            .init_resource::<SettingsSectionSelection>()
            .init_resource::<TicksPerSecondInputState>()
            .add_systems(OnEnter(SettingsState::Open), spawn_settings_screen)
            .add_systems(
                Update,
                (
                    handle_close_requests,
                    handle_section_buttons,
                    handle_game_mode_buttons,
                    handle_ticks_step_buttons,
                    handle_ticks_input,
                    handle_ticks_keyboard,
                    handle_language_buttons,
                    handle_display_tooltips_toggle,
                    sync_section_ui,
                    sync_game_mode_buttons,
                    sync_language_buttons,
                    sync_display_tooltips_toggle,
                    sync_ticks_per_second_text,
                    sync_render_distance_text,
                    sync_slider_thumb,
                )
                    .chain()
                    .run_if(in_state(SettingsState::Open)),
            );
    }
}
