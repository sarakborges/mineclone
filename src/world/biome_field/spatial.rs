use bevy::prelude::*;

use crate::world::deterministic::{hash_signed, mix_hash_u64};
pub(super) use crate::world::deterministic::hash_unit;
pub(super) use crate::world::math::{lerp, smoothstep};

use super::constants::{
    BORDER_TRANSITION_WIDTH, BORDER_WARP_BROAD_AMPLITUDE, BORDER_WARP_BROAD_SCALE,
    BORDER_WARP_DETAIL_AMPLITUDE, BORDER_WARP_DETAIL_SCALE, SITE_JITTER_FRACTION,
    VOLUME_SITE_JITTER_FRACTION, VOLUME_WARP_AMPLITUDE,
};

pub(super) fn surface_minimum_spacing(minimum_radius: Vec2) -> Vec2 {
    let total_warp = BORDER_WARP_BROAD_AMPLITUDE + BORDER_WARP_DETAIL_AMPLITUDE;
    let border_allowance = BORDER_TRANSITION_WIDTH + total_warp * 2.0;
    minimum_radius * 2.0 + Vec2::splat(border_allowance)
}

pub(super) fn warp_surface_position(position: Vec2, seed: u64) -> Vec2 {
    let broad_position = position * BORDER_WARP_BROAD_SCALE;
    let detail_position = position * BORDER_WARP_DETAIL_SCALE;

    let broad = Vec2::new(
        surface_value_noise(broad_position, seed ^ 0x243f_6a88_85a3_08d3),
        surface_value_noise(
            broad_position + Vec2::new(37.25, -19.75),
            seed ^ 0x1319_8a2e_0370_7344,
        ),
    ) * BORDER_WARP_BROAD_AMPLITUDE;

    let detail = Vec2::new(
        surface_value_noise(
            detail_position + Vec2::new(-11.5, 43.0),
            seed ^ 0xa409_3822_299f_31d0,
        ),
        surface_value_noise(
            detail_position + Vec2::new(29.0, 7.75),
            seed ^ 0x082e_fa98_ec4e_6c89,
        ),
    ) * BORDER_WARP_DETAIL_AMPLITUDE;

    position + broad + detail
}

pub(super) fn varied_surface_margin_width(
    position: Vec2,
    seed: u64,
    width: f32,
    width_variation: f32,
    variation_scale: f32,
) -> f32 {
    if width_variation <= f32::EPSILON {
        return width;
    }

    let noise = surface_value_noise(
        position * variation_scale + Vec2::new(17.0, -31.0),
        seed ^ 0x4528_21e6_38d0_1377,
    );
    width + noise * width_variation
}

fn surface_value_noise(position: Vec2, seed: u64) -> f32 {
    let x0 = position.x.floor() as i32;
    let z0 = position.y.floor() as i32;
    let x1 = x0 + 1;
    let z1 = z0 + 1;
    let tx = smoothstep(position.x - x0 as f32);
    let tz = smoothstep(position.y - z0 as f32);

    let top = lerp(
        hash_signed(cell_hash(IVec2::new(x0, z0), seed)),
        hash_signed(cell_hash(IVec2::new(x1, z0), seed)),
        tx,
    );
    let bottom = lerp(
        hash_signed(cell_hash(IVec2::new(x0, z1), seed)),
        hash_signed(cell_hash(IVec2::new(x1, z1), seed)),
        tx,
    );
    lerp(top, bottom, tz)
}

pub(super) fn warp_volume_position(position: Vec3, seed: u64) -> Vec3 {
    let phase_x = hash_signed(seed.rotate_left(5)) * std::f32::consts::TAU;
    let phase_y = hash_signed(seed.rotate_left(19)) * std::f32::consts::TAU;
    let phase_z = hash_signed(seed.rotate_left(37)) * std::f32::consts::TAU;

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
    let jitter_x = hash_signed(hash) * spacing.x * SITE_JITTER_FRACTION;
    let jitter_z = hash_signed(hash.rotate_left(29)) * spacing.y * SITE_JITTER_FRACTION;

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
        hash_signed(hash) * spacing.x * VOLUME_SITE_JITTER_FRACTION,
        hash_signed(hash.rotate_left(21)) * spacing.y * VOLUME_SITE_JITTER_FRACTION,
        hash_signed(hash.rotate_left(43)) * spacing.z * VOLUME_SITE_JITTER_FRACTION,
    );
    let mut position = base + jitter;

    position.y = position.y.max(0.0);
    position
}

pub(super) fn cell_hash(cell: IVec2, seed: u64) -> u64 {
    let mut hash = seed ^ 0xa076_1d64_78bd_642f;
    hash ^= (cell.x as i64 as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
    hash ^= (cell.y as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    mix_hash_u64(hash)
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
