use bevy::prelude::*;

use crate::{
    app::settings_state::SettingsState,
    ui::transition::{ScreenTransition, ScreenTransitionTarget},
};

#[derive(Component)]
pub(super) struct SettingsBackButton;

pub(super) fn handle_close_requests(
    keys: Res<ButtonInput<KeyCode>>,
    interactions: Query<&Interaction, (Changed<Interaction>, With<SettingsBackButton>)>,
    mut transition: ResMut<ScreenTransition>,
) {
    let back_pressed = interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);

    if back_pressed || keys.just_pressed(KeyCode::Escape) {
        transition.request(ScreenTransitionTarget::settings(SettingsState::Closed));
    }
}
