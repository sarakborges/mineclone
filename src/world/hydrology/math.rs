use bevy::prelude::*;

use crate::world::deterministic::avalanche_u64;
pub(super) use crate::world::deterministic::{hash_signed, hash_unit};
pub(super) use crate::world::math::{lerp, smoothstep};

use super::constants::{
    COAST_OCEAN_BLEND_END_STRENGTH, COAST_OCEAN_BLEND_START_STRENGTH,
    COAST_SURFACE_BLEND_END_STRENGTH, OCEAN_CONTINENTALNESS_THRESHOLD, OCEAN_TRANSITION_WIDTH,
    SHORE_STRENGTH,
};

// A river's water occupies strength > SHORE_STRENGTH, not the full graph radius.
// Carving and the water's bed must taper against that very same physical edge.
pub(super) const RIVER_WATER_BOUNDARY_NORMALIZED_DISTANCE: f32 = 1.0 - SHORE_STRENGTH;

pub(super) fn river_channel_profile(normalized_distance: f32) -> f32 {
    smoothstep(
        (1.0 - normalized_distance / RIVER_WATER_BOUNDARY_NORMALIZED_DISTANCE).clamp(0.0, 1.0),
    )
}

pub(super) fn ocean_continentalness_threshold(ocean_weight: f32) -> f32 {
    (OCEAN_CONTINENTALNESS_THRESHOLD * ocean_weight.max(0.0)).clamp(0.0, 1.0)
}

pub(crate) fn ocean_strength(continentalness: f32, ocean_weight: f32) -> f32 {
    let threshold = ocean_continentalness_threshold(ocean_weight);
    let raw = (threshold - continentalness) / OCEAN_TRANSITION_WIDTH;
    smoothstep(raw.clamp(0.0, 1.0))
}

pub(crate) fn ocean_continentalness_for_surface_weight(
    surface_weight: f32,
    ocean_weight: f32,
) -> f32 {
    let threshold = ocean_continentalness_threshold(ocean_weight);
    threshold - OCEAN_TRANSITION_WIDTH * surface_weight.clamp(0.0, 1.0)
}

pub(super) fn ocean_floor_is_submerged(floor: f32, sea_level: f32) -> bool {
    floor < sea_level - 0.5
}

pub(crate) fn suppress_ocean_continentalness(
    continentalness: f32,
    ocean_weight: f32,
    suppression: f32,
) -> f32 {
    let threshold = ocean_continentalness_threshold(ocean_weight);
    let raw = ((threshold - continentalness) / OCEAN_TRANSITION_WIDTH).clamp(0.0, 1.0);
    if raw <= 0.0 {
        return continentalness;
    }

    let retained_raw = raw * (1.0 - suppression.clamp(0.0, 1.0));
    threshold - retained_raw * OCEAN_TRANSITION_WIDTH
}

pub(super) fn hydrology_biome_weights(strength: f32) -> (f32, f32, f32) {
    let strength = strength.clamp(0.0, 1.0);

    if strength <= 0.0 {
        return (1.0, 0.0, 0.0);
    }

    if strength < COAST_SURFACE_BLEND_END_STRENGTH {
        let coast = smoothstep(strength / COAST_SURFACE_BLEND_END_STRENGTH);
        return (1.0 - coast, coast, 0.0);
    }

    if strength <= COAST_OCEAN_BLEND_START_STRENGTH {
        return (0.0, 1.0, 0.0);
    }

    if strength < COAST_OCEAN_BLEND_END_STRENGTH {
        let ocean = smoothstep(
            (strength - COAST_OCEAN_BLEND_START_STRENGTH)
                / (COAST_OCEAN_BLEND_END_STRENGTH - COAST_OCEAN_BLEND_START_STRENGTH),
        );
        return (0.0, 1.0 - ocean, ocean);
    }

    (0.0, 0.0, 1.0)
}

pub(super) fn hydrology_dominates_surface(strength: f32) -> bool {
    let (surface, coast, ocean) = hydrology_biome_weights(strength);
    coast.max(ocean) >= surface
}

pub(super) fn cell_hash(cell: IVec2, seed: u64) -> u64 {
    let mut hash = seed ^ 0x9e37_79b9_7f4a_7c15;
    hash ^= (cell.x as i64 as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= (cell.y as i64 as u64).wrapping_mul(0x94d0_49bb_1331_11eb);
    avalanche_u64(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ocean_surface_weight_maps_back_to_physical_ocean_strength() {
        for surface_weight in [0.0_f32, 0.25, 0.5, 0.75, 1.0] {
            let continentalness =
                ocean_continentalness_for_surface_weight(surface_weight, 1.0);
            let expected = smoothstep(surface_weight);
            assert!(
                (ocean_strength(continentalness, 1.0) - expected).abs() < 0.0001
            );
        }
    }

    #[test]
    fn ocean_strength_stops_at_the_authored_ocean_boundary() {
        assert_eq!(ocean_strength(0.38, 1.0), 0.0);
        assert_eq!(ocean_strength(0.45, 1.0), 0.0);
        assert!(ocean_strength(0.36, 1.0) > 0.0);
        assert!(ocean_strength(0.20, 1.0) > ocean_strength(0.36, 1.0));
    }

    #[test]
    fn dry_ocean_floor_is_not_submerged() {
        assert!(!ocean_floor_is_submerged(89.5, 90.0));
        assert!(ocean_floor_is_submerged(89.49, 90.0));
    }

    #[test]
    fn river_bed_profile_ends_at_the_actual_water_boundary() {
        assert_eq!(river_channel_profile(0.0), 1.0);
        assert!(river_channel_profile(0.5) > 0.0);
        assert_eq!(river_channel_profile(RIVER_WATER_BOUNDARY_NORMALIZED_DISTANCE), 0.0);
        assert_eq!(river_channel_profile(0.9), 0.0);
    }
}
