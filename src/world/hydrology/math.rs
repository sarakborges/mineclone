use bevy::prelude::*;

pub(super) use crate::world::math::{lerp, smoothstep};

use super::constants::{
    COAST_OCEAN_BLEND_END_STRENGTH, COAST_OCEAN_BLEND_START_STRENGTH,
    COAST_SURFACE_BLEND_END_STRENGTH, OCEAN_CONTINENTALNESS_THRESHOLD, OCEAN_TRANSITION_WIDTH,
};

pub(super) fn ocean_strength(continentalness: f32) -> f32 {
    let raw = (OCEAN_CONTINENTALNESS_THRESHOLD - continentalness) / OCEAN_TRANSITION_WIDTH;
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
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^ (hash >> 31)
}

pub(super) fn hash_unit(hash: u64) -> f32 {
    (hash & 0xffff) as f32 / u16::MAX as f32
}

pub(super) fn hash_signed(hash: u64) -> f32 {
    hash_unit(hash) * 2.0 - 1.0
}
