use bevy::prelude::*;

use crate::app::settings_state::SettingsState;
use layout::spawn_settings_screen;
use navigation::handle_close_requests;
use render_distance_logic::{sync_render_distance_text, sync_slider_thumb};

mod layout;
mod navigation;
mod render_distance_logic;
mod render_distance_section;

pub struct SettingsScreenPlugin;

impl Plugin for SettingsScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SettingsState>()
            .add_systems(OnEnter(SettingsState::Open), spawn_settings_screen)
            .add_systems(
                Update,
                (
                    handle_close_requests,
                    sync_render_distance_text,
                    sync_slider_thumb,
                )
                    .run_if(in_state(SettingsState::Open)),
            );
    }
}
