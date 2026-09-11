use bevy::prelude::*;

use crate::world::{
    deterministic::{hash_unit, mix_hash_u64},
    math::{lerp, smoothstep},
};

const OCTAVE_SEED_STEP: u64 = 0x9e37_79b9_7f4a_7c15;

pub(crate) fn fractal_noise_2d(position: Vec2, seed: u64, octaves: usize) -> f32 {
    if octaves == 0 {
        return 0.0;
    }

    let mut value = 0.0;
    let mut normalization = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;

    for octave in 0..octaves {
        let octave_seed = seed.wrapping_add((octave as u64).wrapping_mul(OCTAVE_SEED_STEP));
        value += value_noise_2d(position * frequency, octave_seed) * amplitude;
        normalization += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    value / normalization
}

pub(crate) fn value_noise_2d(position: Vec2, seed: u64) -> f32 {
    let x0 = position.x.floor() as i32;
    let z0 = position.y.floor() as i32;
    let x1 = x0 + 1;
    let z1 = z0 + 1;
    let tx = smoothstep(position.x - x0 as f32);
    let tz = smoothstep(position.y - z0 as f32);
    let top = lerp(
        lattice_noise_3d(x0, 0, z0, seed),
        lattice_noise_3d(x1, 0, z0, seed),
        tx,
    );
    let bottom = lerp(
        lattice_noise_3d(x0, 0, z1, seed),
        lattice_noise_3d(x1, 0, z1, seed),
        tx,
    );

    lerp(top, bottom, tz)
}

pub(crate) fn value_noise_3d(position: Vec3, seed: u64) -> f32 {
    let x0 = position.x.floor() as i32;
    let y0 = position.y.floor() as i32;
    let z0 = position.z.floor() as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let z1 = z0 + 1;
    let tx = smoothstep(position.x - x0 as f32);
    let ty = smoothstep(position.y - y0 as f32);
    let tz = smoothstep(position.z - z0 as f32);

    let c000 = lattice_noise_3d(x0, y0, z0, seed);
    let c100 = lattice_noise_3d(x1, y0, z0, seed);
    let c010 = lattice_noise_3d(x0, y1, z0, seed);
    let c110 = lattice_noise_3d(x1, y1, z0, seed);
    let c001 = lattice_noise_3d(x0, y0, z1, seed);
    let c101 = lattice_noise_3d(x1, y0, z1, seed);
    let c011 = lattice_noise_3d(x0, y1, z1, seed);
    let c111 = lattice_noise_3d(x1, y1, z1, seed);

    let x00 = lerp(c000, c100, tx);
    let x10 = lerp(c010, c110, tx);
    let x01 = lerp(c001, c101, tx);
    let x11 = lerp(c011, c111, tx);
    let y0 = lerp(x00, x10, ty);
    let y1 = lerp(x01, x11, ty);

    lerp(y0, y1, tz)
}

fn lattice_noise_3d(x: i32, y: i32, z: i32, seed: u64) -> f32 {
    let mut hash = seed;
    hash ^= (x as i64 as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
    hash ^= (y as i64 as u64).wrapping_mul(0xd6e8_feb8_6659_fd93);
    hash ^= (z as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);

    hash_unit(mix_hash_u64(hash)) * 2.0 - 1.0
}
