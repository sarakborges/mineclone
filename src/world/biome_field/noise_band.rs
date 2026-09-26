use bevy::prelude::*;

use crate::{
    content::biome_distribution::BiomeDistribution,
    world::{
        deterministic::{hash_string, mix_seed},
        math::smoothstep,
        noise::fractal_noise_2d,
    },
};

const NOISE_BAND_OCTAVES: usize = 4;

pub(super) fn noise_band_strength(
    distribution: BiomeDistribution,
    position: Vec2,
    world_seed: u64,
    biome_id: &str,
) -> f32 {
    let BiomeDistribution::NoiseBand {
        scale,
        core_width,
        edge_width,
        warp_scale,
        warp_strength,
    } = distribution
    else {
        return 0.0;
    };

    let seed = mix_seed(world_seed ^ hash_string(biome_id) ^ 0x94d0_49bb_1331_11eb);
    let warp_position = position * warp_scale;
    let warp = Vec2::new(
        fractal_noise_2d(
            warp_position,
            seed ^ 0x243f_6a88_85a3_08d3,
            NOISE_BAND_OCTAVES,
        ),
        fractal_noise_2d(
            warp_position + Vec2::new(-37.4, 52.8),
            seed ^ 0x1319_8a2e_0370_7344,
            NOISE_BAND_OCTAVES,
        ),
    ) * warp_strength;
    let distance = fractal_noise_2d(
        (position + warp) * scale,
        seed ^ 0xa409_3822_299f_31d0,
        NOISE_BAND_OCTAVES,
    )
    .abs();

    if distance <= core_width {
        return 1.0;
    }

    let outer = core_width + edge_width;
    if distance >= outer {
        return 0.0;
    }

    smoothstep(1.0 - (distance - core_width) / edge_width)
}
