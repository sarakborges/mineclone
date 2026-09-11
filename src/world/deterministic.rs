use std::cmp::Ordering;

use bevy::prelude::Vec3;

pub(crate) fn compare_vec3(left: &Vec3, right: &Vec3) -> Ordering {
    left.x
        .total_cmp(&right.x)
        .then_with(|| left.y.total_cmp(&right.y))
        .then_with(|| left.z.total_cmp(&right.z))
}

pub(crate) fn sorted_unique_vec3s(
    positions: impl IntoIterator<Item = Vec3>,
) -> Vec<Vec3> {
    let mut positions = positions.into_iter().collect::<Vec<_>>();
    positions.sort_by(compare_vec3);
    positions.dedup_by(|left, right| *left == *right);
    positions
}

pub(crate) fn mix_u32_components(
    mut hash: u64,
    components: impl IntoIterator<Item = u32>,
) -> u64 {
    for component in components {
        hash ^= component as u64;
        hash = hash.wrapping_mul(0x9e37_79b1_85eb_ca87);
        hash ^= hash >> 31;
    }

    hash
}

pub(crate) fn mix_hash_u64(mut value: u64) -> u64 {
    value ^= value >> 33;
    value = value.wrapping_mul(0xff51_afd7_ed55_8ccd);
    value ^ (value >> 33)
}

pub(crate) fn avalanche_u64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

pub(crate) fn mix_seed(value: u64) -> u64 {
    let mut value = mix_hash_u64(value);
    value = value.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    value ^ (value >> 33)
}

pub(crate) fn hash_string(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in value.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

pub(crate) fn hash_unit(hash: u64) -> f32 {
    (hash & 0xffff) as f32 / u16::MAX as f32
}

pub(crate) fn hash_signed(hash: u64) -> f32 {
    hash_unit(hash) * 2.0 - 1.0
}
