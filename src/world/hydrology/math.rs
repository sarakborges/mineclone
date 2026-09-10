use bevy::prelude::*;

use super::constants::{OCEAN_CONTINENTALNESS_THRESHOLD, OCEAN_TRANSITION_WIDTH};

pub(super) fn ocean_strength(continentalness: f32) -> f32 {
    let raw = (OCEAN_CONTINENTALNESS_THRESHOLD - continentalness) / OCEAN_TRANSITION_WIDTH;
    smoothstep(raw.clamp(0.0, 1.0))
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

pub(super) fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

pub(super) fn lerp(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
}
