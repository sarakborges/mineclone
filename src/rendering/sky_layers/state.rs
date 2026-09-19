use bevy::prelude::*;

use crate::{content::color::Hsi, rendering::biome_visuals::CurrentBiomeVisuals};

#[derive(Resource, PartialEq)]
pub(super) struct SkyLayerVisualState {
    pub star_density: f32,
    pub star_color: Hsi,
    pub cloud_density: f32,
    pub cloud_color: Hsi,
}

impl Default for SkyLayerVisualState {
    fn default() -> Self {
        Self {
            star_density: 0.0,
            star_color: Hsi::WHITE,
            cloud_density: 0.0,
            cloud_color: Hsi::WHITE,
        }
    }
}

pub(super) fn update_sky_layer_visuals(
    biome_visuals: CurrentBiomeVisuals,
    mut visuals: ResMut<SkyLayerVisualState>,
) {
    if !biome_visuals.inputs_changed() {
        return;
    }

    let next = SkyLayerVisualState {
        star_density: biome_visuals
            .weighted_scalar(|biome| biome.visuals().stars.density)
            .clamp(0.0, 1.0),
        cloud_density: biome_visuals
            .weighted_scalar(|biome| biome.visuals().clouds.density)
            .clamp(0.0, 1.0),
        star_color: biome_visuals.blend_hsi(|biome| biome.visuals().stars.color),
        cloud_color: biome_visuals.blend_hsi(|biome| biome.visuals().clouds.color),
    };

    if *visuals != next {
        *visuals = next;
    }
}
