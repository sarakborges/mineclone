use serde::Deserialize;

use super::color::Hsi;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeSkyLayerVisuals {
    pub density: f32,
    pub color: Hsi,
}

impl Default for BiomeSkyLayerVisuals {
    fn default() -> Self {
        Self {
            density: 0.0,
            color: Hsi::WHITE,
        }
    }
}
