use bevy::prelude::*;

use crate::world::deterministic::{avalanche_u64, compare_vec3, mix_u32_components};
pub(super) use crate::world::deterministic::hash_unit;

pub(super) fn pair_hash(left: Vec3, right: Vec3, seed: u64) -> u64 {
    let (first, second) = if compare_vec3(&left, &right) != std::cmp::Ordering::Greater {
        (left, right)
    } else {
        (right, left)
    };
    let hash = position_hash(first, seed);

    avalanche_u64(mix_u32_components(
        hash,
        [second.x.to_bits(), second.y.to_bits(), second.z.to_bits()],
    ))
}

pub(super) fn position_hash(position: Vec3, seed: u64) -> u64 {
    avalanche_u64(mix_u32_components(
        seed,
        [position.x.to_bits(), position.y.to_bits(), position.z.to_bits()],
    ))
}
