use bevy::prelude::*;

use crate::{
    content::biome_distribution::BiomeDistribution,
    world::{
        deterministic::{hash_string, mix_seed},
        math::smoothstep,
        noise::fractal_noise_2d,
    },
};

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

    let seed = mix_seed(world_seed ^ hash_string(biome_id));
    let warp_position = position * warp_scale;
    let warp = Vec2::new(
        fractal_noise_2d(
            warp_position,
            seed ^ 0x243f_6a88_85a3_08d3,
            BELT_NOISE_OCTAVES,
        ),
        fractal_noise_2d(
            warp_position + Vec2::new(31.7, -19.3),
            seed ^ 0x1319_8a2e_0370_7344,
            BELT_NOISE_OCTAVES,
        ),
    ) * warp_strength;
    let noise = fractal_noise_2d(
        (position + warp) * scale,
        seed ^ 0xa409_3822_299f_31d0,
        BELT_NOISE_OCTAVES,
    );
    let ridge = 1.0 - noise.abs();
    let outer_edge = (threshold - width).max(0.0);
    let progress = ((ridge - outer_edge) / width).clamp(0.0, 1.0);

    smoothstep(progress)
}
