use bevy::prelude::*;

use crate::{
    content::biome_distribution::BiomeDistribution,
    world::{
        deterministic::{hash_signed, hash_string, mix_seed},
        noise::fractal_noise_2d,
    },
};

use super::spatial::{cell_hash, hash_unit, lerp, smoothstep};

const PEAK_NOISE_OCTAVES: usize = 3;
const PEAK_JITTER_FRACTION: f32 = 0.35;

pub(super) fn mountain_peak_strength(
    distribution: BiomeDistribution,
    position: Vec2,
    world_seed: u64,
    biome_id: &str,
) -> f32 {
    let BiomeDistribution::MountainPeak {
        spacing,
        chance,
        radius,
        warp_scale,
        warp_strength,
    } = distribution
    else {
        return 0.0;
    };

    let seed = mix_seed(world_seed ^ hash_string(biome_id) ^ 0x517c_c1b7_2722_0a95);
    let warp_position = position * warp_scale;
    let warp = Vec2::new(
        fractal_noise_2d(
            warp_position,
            seed ^ 0x243f_6a88_85a3_08d3,
            PEAK_NOISE_OCTAVES,
        ),
        fractal_noise_2d(
            warp_position + Vec2::new(47.1, -28.6),
            seed ^ 0x1319_8a2e_0370_7344,
            PEAK_NOISE_OCTAVES,
        ),
    ) * warp_strength;
    let sampled = position + warp;
    let center = IVec2::new(
        (sampled.x / spacing).floor() as i32,
        (sampled.y / spacing).floor() as i32,
    );
    let jitter_extent = spacing * PEAK_JITTER_FRACTION;
    let search_radius = ((radius.max + warp_strength + jitter_extent) / spacing).ceil() as i32 + 1;
    let mut strongest = 0.0_f32;

    for z in -search_radius..=search_radius {
        for x in -search_radius..=search_radius {
            let cell = center + IVec2::new(x, z);
            let hash = cell_hash(cell, seed);
            if hash_unit(hash) > chance {
                continue;
            }

            let base = (cell.as_vec2() + Vec2::splat(0.5)) * spacing;
            let jitter = Vec2::new(
                hash_signed(hash.rotate_left(17)) * jitter_extent,
                hash_signed(hash.rotate_left(37)) * jitter_extent,
            );
            let anchor = base + jitter;
            let peak_radius = lerp(radius.min, radius.max, hash_unit(hash.rotate_left(53)));
            let progress = 1.0 - sampled.distance(anchor) / peak_radius;

            if progress > 0.0 {
                strongest = strongest.max(smoothstep(progress.clamp(0.0, 1.0)));
            }
        }
    }

    strongest
}
