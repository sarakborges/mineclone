use bevy::prelude::*;

use crate::content::biome_structure::StructurePlacementRules;

pub(super) fn candidate_anchor(
    world_seed: u64,
    biome_id: &str,
    structure_reference: &str,
    placement: StructurePlacementRules,
    cell: IVec2,
) -> Option<IVec2> {
    let hash = placement_hash(world_seed, biome_id, structure_reference, cell);
    let chance = unit_interval(hash);
    if chance >= placement.chance {
        return None;
    }

    let spacing = placement.spacing;
    let half_spacing = spacing / 2;
    let center_x = cell.x.checked_mul(spacing)?.checked_add(half_spacing)?;
    let center_z = cell.y.checked_mul(spacing)?.checked_add(half_spacing)?;
    let jitter = placement.jitter;
    let jitter_x = signed_jitter(hash ^ 0x517c_c1b7_2722_0a95, jitter);
    let jitter_z = signed_jitter(hash ^ 0x6eed_0e9d_a4d9_4a4f, jitter);

    Some(IVec2::new(
        center_x.checked_add(jitter_x)?,
        center_z.checked_add(jitter_z)?,
    ))
}


pub(super) fn structure_member_hash(
    world_seed: u64,
    biome_id: &str,
    structure_reference: &str,
    anchor: IVec2,
) -> u64 {
    let mut hash =
        world_seed.rotate_left(17) ^ string_hash(structure_reference).rotate_left(7);
    hash ^= string_hash(biome_id).rotate_left(37);
    hash ^= (anchor.x as i64 as u64).wrapping_mul(0xd6e8_feb8_6659_fd93);
    hash ^= (anchor.y as i64 as u64).wrapping_mul(0xa5a3_58d5_33f6_8d21);
    avalanche(hash)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_anchor_rejects_coordinate_overflow() {
        let placement = StructurePlacementRules {
            spacing: 1_000,
            chance: 1.0,
            jitter: 100,
        };

        assert!(candidate_anchor(42, "biome", "structure", placement, IVec2::ZERO).is_some());
        assert!(
            candidate_anchor(
                42,
                "biome",
                "structure",
                placement,
                IVec2::new(i32::MAX, 0),
            )
            .is_none()
        );
    }
}
