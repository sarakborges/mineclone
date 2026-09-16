use bevy::prelude::*;

use crate::content::{
    biome_structure::StructurePlacementRules,
    structure::StructureDefinition,
};

pub(super) fn candidate_anchor(
    world_seed: u64,
    biome_id: &str,
    structure: &StructureDefinition,
    placement: StructurePlacementRules,
    cell: IVec2,
) -> Option<IVec2> {
    let hash = placement_hash(world_seed, biome_id, &structure.id, cell);
    let chance = unit_interval(hash);
    if chance >= placement.chance {
        return None;
    }

    let spacing = placement.spacing;
    let center = cell * spacing + IVec2::splat(spacing / 2);
    let jitter = placement.jitter;
    let jitter_x = signed_jitter(hash ^ 0x517c_c1b7_2722_0a95, jitter);
    let jitter_z = signed_jitter(hash ^ 0x6eed_0e9d_a4d9_4a4f, jitter);

    Some(center + IVec2::new(jitter_x, jitter_z))
}

fn placement_hash(world_seed: u64, biome_id: &str, structure_id: &str, cell: IVec2) -> u64 {
    let mut hash = world_seed ^ string_hash(structure_id);
    hash ^= string_hash(biome_id).rotate_left(29);
    hash ^= (cell.x as i64 as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
    hash ^= (cell.y as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    avalanche(hash)
}

fn string_hash(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;

    for byte in value.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    hash
}

fn signed_jitter(hash: u64, maximum: i32) -> i32 {
    if maximum == 0 {
        return 0;
    }

    let range = (maximum * 2 + 1) as u64;
    (avalanche(hash) % range) as i32 - maximum
}

fn unit_interval(hash: u64) -> f32 {
    let value = hash >> 11;
    (value as f64 * (1.0 / (1_u64 << 53) as f64)) as f32
}

fn avalanche(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
