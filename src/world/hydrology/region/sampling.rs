use bevy::prelude::*;

use super::HydrologyRegion;
use crate::world::{
    hydrology::{
        constants::{
            HYDROLOGY_REGION_SIZE, MACRO_SAMPLE_GRID, OCEAN_EXTRA_DEPTH,
            OCEAN_FLOOR_DETAIL_SCALE, OCEAN_FLOOR_NOISE_SCALE, OCEAN_FLOOR_VARIATION,
            OCEAN_MINIMUM_DEPTH,
        },
        math::{lerp, ocean_strength, smoothstep},
        types::HydrologyMacroSample,
    },
    noise::fractal_noise_2d,
};

impl HydrologyRegion {
    pub(super) fn ocean_floor_target(&self, position: Vec2, strength: f32) -> f32 {
        let broad = fractal_noise_2d(
            position * OCEAN_FLOOR_NOISE_SCALE,
            self.seed ^ 0x7f4a_7c15_d6e8_feb8,
            4,
        );
        let detail = fractal_noise_2d(
            position * OCEAN_FLOOR_DETAIL_SCALE,
            self.seed ^ 0x94d0_49bb_1331_11eb,
            3,
        );
        let relief = (broad * 0.72 + detail * 0.28) * OCEAN_FLOOR_VARIATION * strength;

        self.sea_level - OCEAN_MINIMUM_DEPTH - OCEAN_EXTRA_DEPTH * strength + relief
    }

    pub fn ocean_strength_at(&self, position: Vec2) -> f32 {
        self.macro_sample_at(position).map_or(0.0, |sample| {
            ocean_strength(sample.continentalness, self.ocean_weight)
        })
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
