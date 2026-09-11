use bevy::prelude::*;

use crate::content::biome_distribution::BiomeDistribution;

const BELT_NOISE_OCTAVES: usize = 4;

pub(super) fn mountain_belt_strength(
    distribution: BiomeDistribution,
    position: Vec2,
    world_seed: u64,
    biome_id: &str,
) -> f32 {
    let BiomeDistribution::MountainBelt {
        scale,
        threshold,
        width,
        warp_scale,
        warp_strength,
    } = distribution
    else {
        return 0.0;
    };

    let seed = mix_seed(world_seed ^ string_hash(biome_id));
    let warp_position = position * warp_scale;
    let warp = Vec2::new(
        fractal_noise(warp_position, seed ^ 0x243f_6a88_85a3_08d3),
        fractal_noise(
            warp_position + Vec2::new(31.7, -19.3),
            seed ^ 0x1319_8a2e_0370_7344,
        ),
    ) * warp_strength;
    let noise = fractal_noise((position + warp) * scale, seed ^ 0xa409_3822_299f_31d0);
    let ridge = 1.0 - noise.abs();
    let outer_edge = (threshold - width).max(0.0);
    let progress = ((ridge - outer_edge) / width).clamp(0.0, 1.0);

    smoothstep(progress)
}

fn fractal_noise(position: Vec2, seed: u64) -> f32 {
    let mut value = 0.0;
    let mut normalization = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;

    for octave in 0..BELT_NOISE_OCTAVES {
        let octave_seed = seed.wrapping_add((octave as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
        value += value_noise(position * frequency, octave_seed) * amplitude;
        normalization += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    value / normalization
}

fn value_noise(position: Vec2, seed: u64) -> f32 {
    let x0 = position.x.floor() as i32;
    let z0 = position.y.floor() as i32;
    let x1 = x0 + 1;
    let z1 = z0 + 1;
    let tx = smoothstep(position.x - x0 as f32);
    let tz = smoothstep(position.y - z0 as f32);
    let top = lerp(lattice_noise(x0, z0, seed), lattice_noise(x1, z0, seed), tx);
    let bottom = lerp(lattice_noise(x0, z1, seed), lattice_noise(x1, z1, seed), tx);

    lerp(top, bottom, tz)
}

fn lattice_noise(x: i32, z: i32, seed: u64) -> f32 {
    let mut hash = seed;
    hash ^= (x as i64 as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
    hash ^= (z as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^= hash >> 33;
    let normalized = (hash & 0xffff) as f32 / u16::MAX as f32;

    normalized * 2.0 - 1.0
}

fn string_hash(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in value.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn mix_seed(mut seed: u64) -> u64 {
    seed ^= seed >> 33;
    seed = seed.wrapping_mul(0xff51_afd7_ed55_8ccd);
    seed ^= seed >> 33;
    seed = seed.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    seed ^= seed >> 33;
    seed
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

fn lerp(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
}
