use bevy::prelude::*;

use crate::content::biome_distribution::BiomeDistribution;

use super::{
    mountain_belt::mountain_belt_strength,
    mountain_peak::mountain_peak_strength,
    noise_band::noise_band_strength,
};

pub(super) fn distribution_strength(
    distribution: BiomeDistribution,
    position: Vec2,
    world_seed: u64,
    biome_id: &str,
) -> f32 {
    mountain_belt_strength(distribution, position, world_seed, biome_id)
        .max(mountain_peak_strength(
            distribution,
            position,
            world_seed,
            biome_id,
        ))
        .max(noise_band_strength(
            distribution,
            position,
            world_seed,
            biome_id,
        ))
}
