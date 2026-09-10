use bevy::prelude::*;

use super::constants::{
    BORDER_TRANSITION_WIDTH, BORDER_WARP_AMPLITUDE, SITE_JITTER_FRACTION,
    VOLUME_SITE_JITTER_FRACTION, VOLUME_WARP_AMPLITUDE,
};

pub(super) fn surface_minimum_spacing(minimum_radius: Vec2) -> Vec2 {
    let border_allowance = BORDER_TRANSITION_WIDTH + BORDER_WARP_AMPLITUDE * 2.0;
    minimum_radius * 2.0 + Vec2::splat(border_allowance)
}

pub(super) fn warp_surface_position(position: Vec2, seed: u64) -> Vec2 {
    let phase_x = hash_component(seed) * std::f32::consts::TAU;
    let phase_z = hash_component(seed.rotate_left(31)) * std::f32::consts::TAU;

    position
        + Vec2::new(
            (position.y * 0.011 + phase_x).sin() * BORDER_WARP_AMPLITUDE,
            (position.x * 0.009 + phase_z).sin() * BORDER_WARP_AMPLITUDE,
        )
}

pub(super) fn warp_volume_position(position: Vec3, seed: u64) -> Vec3 {
    let phase_x = hash_component(seed.rotate_left(5)) * std::f32::consts::TAU;
    let phase_y = hash_component(seed.rotate_left(19)) * std::f32::consts::TAU;
    let phase_z = hash_component(seed.rotate_left(37)) * std::f32::consts::TAU;

    position
        + Vec3::new(
            ((position.y + position.z) * 0.008 + phase_x).sin() * VOLUME_WARP_AMPLITUDE,
            ((position.x + position.z) * 0.006 + phase_y).sin() * VOLUME_WARP_AMPLITUDE,
            ((position.x + position.y) * 0.008 + phase_z).sin() * VOLUME_WARP_AMPLITUDE,
        )
}

pub(super) fn surface_site_position(cell: IVec2, spacing: Vec2, seed: u64) -> Vec2 {
    let base = Vec2::new(cell.x as f32 * spacing.x, cell.y as f32 * spacing.y);

    if cell == IVec2::ZERO {
        return base;
    }

    let hash = cell_hash(cell, seed);
    let jitter_x = hash_component(hash) * spacing.x * SITE_JITTER_FRACTION;
    let jitter_z = hash_component(hash.rotate_left(29)) * spacing.y * SITE_JITTER_FRACTION;

    base + Vec2::new(jitter_x, jitter_z)
}

pub(super) fn volume_site_position(cell: IVec3, spacing: Vec3, seed: u64) -> Vec3 {
    let base = Vec3::new(
        cell.x as f32 * spacing.x,
        cell.y as f32 * spacing.y,
        cell.z as f32 * spacing.z,
    );

    if cell == IVec3::ZERO {
        return base;
    }

    let hash = volume_cell_hash(cell, seed);
    let jitter = Vec3::new(
        hash_component(hash) * spacing.x * VOLUME_SITE_JITTER_FRACTION,
        hash_component(hash.rotate_left(21)) * spacing.y * VOLUME_SITE_JITTER_FRACTION,
        hash_component(hash.rotate_left(43)) * spacing.z * VOLUME_SITE_JITTER_FRACTION,
    );
    let mut position = base + jitter;

    position.y = position.y.max(0.0);
    position
}

pub(super) fn biome_index(cell: IVec2, biome_count: usize, seed: u64) -> usize {
    if cell == IVec2::ZERO {
        return 0;
    }

    cell_hash(cell, seed) as usize % biome_count
}

pub(super) fn cell_hash(cell: IVec2, seed: u64) -> u64 {
    let mut hash = seed ^ 0xa076_1d64_78bd_642f;
    hash ^= (cell.x as i64 as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
    hash ^= (cell.y as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^= hash >> 33;
    hash
}

pub(super) fn volume_cell_hash(cell: IVec3, seed: u64) -> u64 {
    let mut hash = seed ^ 0xe703_7ed1_a0b4_28db;
    hash ^= (cell.x as i64 as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
    hash ^= (cell.y as i64 as u64).wrapping_mul(0xd6e8_feb8_6659_fd93);
    hash ^= (cell.z as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    hash ^= hash >> 32;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 29;
    hash
}

fn hash_component(hash: u64) -> f32 {
    hash_unit(hash) * 2.0 - 1.0
}

pub(super) fn hash_unit(hash: u64) -> f32 {
    (hash & 0xffff) as f32 / u16::MAX as f32
}

pub(super) fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

pub(super) fn lerp(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
}
