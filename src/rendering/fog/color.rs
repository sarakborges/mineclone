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
        // The far end of the fog must visually merge with the horizon; otherwise
        // fully fogged terrain remains visible as a flat silhouette against the sky.
        fog.color = visuals.sky_color;
    }
}
