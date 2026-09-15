use bevy::prelude::*;

use crate::{player::camera::GameplayCamera, rendering::environment::EnvironmentVisualState};

pub(super) fn update_fog_color(
    visuals: Res<EnvironmentVisualState>,
    mut fogs: Query<&mut DistanceFog, With<GameplayCamera>>,
) {
    if !visuals.is_changed() {
        return;
    }

    let color = visuals.fog_color.to_color();
    for mut fog in &mut fogs {
        if fog.color != color {
            fog.color = color;
        }
    }
}
