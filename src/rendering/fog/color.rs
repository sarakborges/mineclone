use bevy::prelude::*;

use crate::{player::camera::GameplayCamera, rendering::environment::EnvironmentVisualState};

pub(super) fn update_fog_color(
    visuals: Res<EnvironmentVisualState>,
    mut fogs: Query<&mut DistanceFog, With<GameplayCamera>>,
) {
    if !visuals.is_changed() {
        return;
    }

    for mut fog in &mut fogs {
        fog.color = visuals.fog_color;
    }
}
