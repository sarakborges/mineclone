use bevy::prelude::*;

use super::HydrologyRegion;
use crate::world::hydrology::{
    constants::{HYDROLOGY_REGION_SIZE, MACRO_SAMPLE_GRID},
    math::{lerp, ocean_strength, smoothstep},
    types::HydrologyMacroSample,
};

impl HydrologyRegion {
    pub fn ocean_strength_at(&self, position: Vec2) -> f32 {
        self.macro_sample_at(position)
            .map_or(0.0, |sample| ocean_strength(sample.continentalness))
    }

    pub(super) fn macro_sample_at(&self, position: Vec2) -> Option<HydrologyMacroSample> {
        if self.macro_samples.len() != MACRO_SAMPLE_GRID * MACRO_SAMPLE_GRID {
            return None;
        }

        let origin = self.coord.as_vec2() * HYDROLOGY_REGION_SIZE;
        let step = HYDROLOGY_REGION_SIZE / (MACRO_SAMPLE_GRID - 1) as f32;
        let local = (position - origin) / step;
        let x = local.x.clamp(0.0, (MACRO_SAMPLE_GRID - 1) as f32);
        let z = local.y.clamp(0.0, (MACRO_SAMPLE_GRID - 1) as f32);
        let x0 = x.floor() as usize;
        let z0 = z.floor() as usize;
        let x1 = (x0 + 1).min(MACRO_SAMPLE_GRID - 1);
        let z1 = (z0 + 1).min(MACRO_SAMPLE_GRID - 1);
        let tx = smoothstep(x - x0 as f32);
        let tz = smoothstep(z - z0 as f32);
        let top = interpolate_macro(
            self.macro_samples[macro_index(x0, z0)],
            self.macro_samples[macro_index(x1, z0)],
            tx,
        );
        let bottom = interpolate_macro(
            self.macro_samples[macro_index(x0, z1)],
            self.macro_samples[macro_index(x1, z1)],
            tx,
        );

        Some(interpolate_macro(top, bottom, tz))
    }
}

fn macro_index(x: usize, z: usize) -> usize {
    x + z * MACRO_SAMPLE_GRID
}

fn interpolate_macro(
    from: HydrologyMacroSample,
    to: HydrologyMacroSample,
    amount: f32,
) -> HydrologyMacroSample {
    HydrologyMacroSample {
        elevation: lerp(from.elevation, to.elevation, amount),
        continentalness: lerp(from.continentalness, to.continentalness, amount),
    }
}
