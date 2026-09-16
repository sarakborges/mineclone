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
    fn river_bed_profile_ends_at_the_actual_water_boundary() {
        assert_eq!(river_channel_profile(0.0), 1.0);
        assert!(river_channel_profile(0.5) > 0.0);
        assert_eq!(
            river_channel_profile(RIVER_WATER_BOUNDARY_NORMALIZED_DISTANCE),
            0.0
        );
        assert_eq!(river_channel_profile(0.9), 0.0);
    }
}
