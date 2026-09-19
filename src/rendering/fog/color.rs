use bevy::prelude::*;

use crate::{player::camera::GameplayCamera, rendering::environment::EnvironmentVisualState};

pub(super) fn update_fog_color(
    visuals: Res<EnvironmentVisualState>,
    mut fogs: Query<&mut DistanceFog, With<GameplayCamera>>,
) {
    if !visuals.is_changed() {
        return;
    }

    // Keep fully fogged geometry indistinguishable from the flat sky background.
    let color = visuals.sky_color.to_color();
    for mut fog in &mut fogs {
        if fog.color != color {
            fog.color = color;
        }
    }
}
