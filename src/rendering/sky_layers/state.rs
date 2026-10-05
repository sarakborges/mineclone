use bevy::prelude::*;

use crate::content::color::Hsi;

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
