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
const RIVER_CHANNEL_HEADROOM: f32 = 7.0;
const RIVER_MINIMUM_SURFACE_HEADROOM: f32 = 5.0;
const LAKE_MINIMUM_SURFACE_HEADROOM: f32 = 5.0;
const LAKE_MAXIMUM_SURFACE_HEADROOM: f32 = 7.5;
const RIVER_BANK_NOISE_SCALE: f32 = 0.035;
const RIVER_BANK_DETAIL_NOISE_SCALE: f32 = 0.11;
const LAKE_HEADROOM_NOISE_SCALE: f32 = 0.028;
const WATER_VOLUME_AIR_DENSITY: f32 = -0.001;

#[derive(Clone, Copy, Debug)]
struct WaterLevels {
    water_level: f32,
    bed_level: f32,
    strength: f32,
    kind: HydrologyWaterKind,
}

impl From<HydrologyWaterSample<'_>> for WaterLevels {
    fn from(sample: HydrologyWaterSample<'_>) -> Self {
        Self {
            water_level: sample.water_level,
            bed_level: sample.bed_level,
            strength: sample.strength,
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
    original_surface_height: Option<f32>,
) -> DensityColumnHydrology {
    // The authoritative wet volume and the roof opening must use the same
    // support rule as the subsequent physical fluid pass. A high unsupported
    // lake must not hide a lower river, and an unsupported river must not cut
    // a dry headroom tunnel into the terrain.
    let water = match original_surface_height {
        Some(height) => region.hydrology.supported_water_at(horizontal, height),
        None => region.hydrology.water_at(horizontal),
    };
    let river_surface = match original_surface_height {
        Some(height) => region.hydrology.supported_river_surface_at(horizontal, height),
        None => region.hydrology.river_surface_at(horizontal),
    };
    DensityColumnHydrology {
        cave_water: region
            .hydrology
            .water_near(horizontal, CAVE_WATER_HORIZONTAL_CLEARANCE)
            .map(Into::into),
        river_surface: river_surface.map(Into::into),
        water: water.map(Into::into),
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

    if water.kind == HydrologyWaterKind::Lake {
        let headroom = lake_headroom(water.strength, horizontal, seed);
        if cell_top > water.water_level
            && cell_bottom < water.water_level + headroom
            && base_density > 0.0
        {
            return density.min(WATER_VOLUME_AIR_DENSITY);
        }
    }

    // Only the actual wet volume is forced open below the surface. Surface
    // clearance above rivers and lakes is handled by the bounded headroom paths
    // above so hydrology never excavates an unbounded vertical shaft.
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
    let variation = lerp(0.82, 1.18, broad * 0.7 + detail * 0.3);
    let shaped_strength =
        (strength * (1.0 + (variation - 1.0) * (1.0 - strength))).clamp(0.0, 1.0);

    lerp(
        RIVER_MINIMUM_SURFACE_HEADROOM,
        RIVER_CHANNEL_HEADROOM,
        smoothstep(shaped_strength),
    )
}

fn lake_headroom(strength: f32, horizontal: Vec2, seed: u64) -> f32 {
    let noise = value_noise_2d(
        horizontal * LAKE_HEADROOM_NOISE_SCALE,
        seed ^ 0x9e37_79b9_7f4a_7c15,
    ) * 0.5
        + 0.5;
    let variation = smoothstep((strength.clamp(0.0, 1.0) * 0.75 + noise * 0.25).clamp(0.0, 1.0));

    lerp(
        LAKE_MINIMUM_SURFACE_HEADROOM,
        LAKE_MAXIMUM_SURFACE_HEADROOM,
        variation,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn river_headroom_tapers_but_never_below_five_blocks() {
        let horizontal = Vec2::new(12.5, -8.5);
        let seed = 42;

        assert_eq!(river_headroom(0.0, horizontal, seed), 0.0);
        for strength in [0.25, 0.5, 0.75, 1.0] {
            let headroom = river_headroom(strength, horizontal, seed);
            assert!(headroom >= RIVER_MINIMUM_SURFACE_HEADROOM);
            assert!(headroom <= RIVER_CHANNEL_HEADROOM);
        }
    }

    #[test]
    fn lake_headroom_never_drops_below_five_blocks() {
        for strength in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let headroom = lake_headroom(strength, Vec2::new(31.5, -12.5), 42);
            assert!(headroom >= LAKE_MINIMUM_SURFACE_HEADROOM);
            assert!(headroom <= LAKE_MAXIMUM_SURFACE_HEADROOM);
        }
    }

    #[test]
    fn river_bank_shape_varies_across_world_positions() {
        let left = river_headroom(0.5, Vec2::new(-80.5, 14.5), 42);
        let right = river_headroom(0.5, Vec2::new(96.5, 14.5), 42);

        assert_ne!(left, right);
    }

    #[test]
    fn supported_river_water_volume_does_not_carve_an_unsupported_high_lake() {
        // Supported river: water 83, bed 78. A lake at 100 with bed 94
        // would have won unfiltered water_at, despite no physical source.
        let column = DensityColumnHydrology {
            water: Some(WaterLevels {
                water_level: 83.0,
                bed_level: 78.0,
                strength: 1.0,
                kind: HydrologyWaterKind::River,
            }),
            ..Default::default()
        };
        assert_eq!(
            enforce_hydrology_water_volume(3.0, 3.0, Vec3::new(0.5, 95.5, 0.5), column, 42),
            3.0
        );
        assert_eq!(
            enforce_hydrology_water_volume(3.0, 3.0, Vec3::new(0.5, 80.5, 0.5), column, 42),
            WATER_VOLUME_AIR_DENSITY
        );
    }
}
