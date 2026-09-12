use bevy::prelude::*;

use crate::world::{
    generation_region::GenerationRegion,
    hydrology::{HydrologyRiverSurfaceSample, HydrologyWaterKind, HydrologyWaterSample},
    math::{lerp, smoothstep},
    noise::value_noise_2d,
};

const CAVE_WATER_PROTECTION_DEPTH: f32 = 14.0;
const CAVE_WATER_PROTECTION_FADE_DEPTH: f32 = 20.0;
const CAVE_WATER_HORIZONTAL_CLEARANCE: f32 = 8.0;
const RIVER_CHANNEL_HEADROOM: f32 = 8.0;
const RIVER_BANK_NOISE_SCALE: f32 = 0.035;
const RIVER_BANK_DETAIL_NOISE_SCALE: f32 = 0.11;
const WATER_VOLUME_AIR_DENSITY: f32 = -0.001;

#[derive(Clone, Copy, Debug)]
struct WaterLevels {
    water_level: f32,
    bed_level: f32,
    kind: HydrologyWaterKind,
}

impl From<HydrologyWaterSample<'_>> for WaterLevels {
    fn from(sample: HydrologyWaterSample<'_>) -> Self {
        Self {
            water_level: sample.water_level,
            bed_level: sample.bed_level,
            kind: sample.kind,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct RiverSurface {
    water_level: f32,
    strength: f32,
}

impl From<HydrologyRiverSurfaceSample> for RiverSurface {
    fn from(sample: HydrologyRiverSurfaceSample) -> Self {
        Self {
            water_level: sample.water_level,
            strength: sample.strength,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DensityColumnHydrology {
    cave_water: Option<WaterLevels>,
    river_surface: Option<RiverSurface>,
    water: Option<WaterLevels>,
}

pub(crate) fn sample_density_column_hydrology(
    horizontal: Vec2,
    region: &GenerationRegion,
) -> DensityColumnHydrology {
    DensityColumnHydrology {
        cave_water: region
            .hydrology
            .water_near(horizontal, CAVE_WATER_HORIZONTAL_CLEARANCE)
            .map(Into::into),
        river_surface: region.hydrology.river_surface_at(horizontal).map(Into::into),
        water: region.hydrology.water_at(horizontal).map(Into::into),
    }
}

pub(super) fn enforce_hydrology_water_volume(
    density: f32,
    base_density: f32,
    position: Vec3,
    column_hydrology: DensityColumnHydrology,
    seed: u64,
) -> f32 {
    let cell_bottom = position.y - 0.5;
    let cell_top = position.y + 0.5;
    let horizontal = Vec2::new(position.x, position.z);

    if let Some(river) = column_hydrology.river_surface {
        let headroom = river_headroom(river.strength, horizontal, seed);
        if headroom > 0.0
            && cell_top > river.water_level
            && cell_bottom < river.water_level + headroom
            && base_density > 0.0
        {
            return density.min(WATER_VOLUME_AIR_DENSITY);
        }
    }

    let Some(water) = column_hydrology.water else {
        return density;
    };

    // Lakes are open basins. Rivers are not: their opening is controlled only by
    // the bounded headroom profile above, so high terrain can close again once it
    // is more than eight blocks above the water instead of being carved to sky.
    if base_density > 0.0
        && matches!(water.kind, HydrologyWaterKind::Lake)
        && cell_top > water.water_level
    {
        return density.min(WATER_VOLUME_AIR_DENSITY);
    }

    if cell_top <= water.bed_level || cell_bottom >= water.water_level {
        return density;
    }

    density.min(WATER_VOLUME_AIR_DENSITY)
}

pub(super) fn cave_water_clearance(y: f32, column_hydrology: DensityColumnHydrology) -> f32 {
    let Some(water) = column_hydrology.cave_water else {
        return 1.0;
    };
    let depth_below_water = water.water_level - y;

    if depth_below_water < 0.0 {
        return 1.0;
    }
    if depth_below_water <= CAVE_WATER_PROTECTION_DEPTH {
        return 0.0;
    }
    if depth_below_water >= CAVE_WATER_PROTECTION_FADE_DEPTH {
        return 1.0;
    }

    smoothstep(
        (depth_below_water - CAVE_WATER_PROTECTION_DEPTH)
            / (CAVE_WATER_PROTECTION_FADE_DEPTH - CAVE_WATER_PROTECTION_DEPTH),
    )
}

fn river_headroom(strength: f32, horizontal: Vec2, seed: u64) -> f32 {
    let strength = strength.clamp(0.0, 1.0);
    if strength <= 0.0 {
        return 0.0;
    }

    let broad = value_noise_2d(
        horizontal * RIVER_BANK_NOISE_SCALE,
        seed ^ 0x243f_6a88_85a3_08d3,
    ) * 0.5
        + 0.5;
    let detail = value_noise_2d(
        horizontal * RIVER_BANK_DETAIL_NOISE_SCALE,
        seed ^ 0x1319_8a2e_0370_7344,
    ) * 0.5
        + 0.5;
    let variation = lerp(0.72, 1.28, broad * 0.7 + detail * 0.3);
    let shaped_strength =
        (strength * (1.0 + (variation - 1.0) * (1.0 - strength) * 1.5)).clamp(0.0, 1.0);

    RIVER_CHANNEL_HEADROOM * smoothstep(shaped_strength)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn river_headroom_tapers_toward_the_bank() {
        let horizontal = Vec2::new(12.5, -8.5);
        let seed = 42;

        assert_eq!(river_headroom(0.0, horizontal, seed), 0.0);
        assert!(
            river_headroom(0.25, horizontal, seed) < river_headroom(0.5, horizontal, seed)
        );
        assert!(river_headroom(0.5, horizontal, seed) < river_headroom(1.0, horizontal, seed));
        assert_eq!(
            river_headroom(1.0, horizontal, seed),
            RIVER_CHANNEL_HEADROOM
        );
    }

    #[test]
    fn river_headroom_never_exceeds_eight_blocks() {
        for strength in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert!(river_headroom(strength, Vec2::ZERO, 42) <= 8.0);
        }
    }

    #[test]
    fn river_bank_shape_varies_across_world_positions() {
        let left = river_headroom(0.5, Vec2::new(-80.5, 14.5), 42);
        let right = river_headroom(0.5, Vec2::new(96.5, 14.5), 42);

        assert_ne!(left, right);
    }
}
