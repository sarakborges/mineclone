use serde::Deserialize;

use super::color::Rgb;

#[derive(Clone, Copy, Deserialize)]
pub struct BiomeSkyLayerVisuals {
    pub density: f32,
    pub color: Rgb,
}

impl Default for BiomeSkyLayerVisuals {
    fn default() -> Self {
        Self {
            density: 0.0,
            color: Rgb {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
        }
    }
}
