use bevy::prelude::*;

use crate::content::{
    biome::{BiomeKind, BiomeRegistry},
    biome_terrain::BiomeTerrain,
    biome_terrain_modifier::BiomeTerrainModifier,
    dimension::DimensionDefinition,
};

use super::biome_field::{BiomeField, BiomeFieldSample};

const TERRAIN_MIN_CHUNK_Y: i32 = 0;
const NOISE_OCTAVES: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HeightInfluencePolicy {
    Blend,
    LowerOnly,
}

pub fn surface_height(
    position: IVec2,
    dimension: &DimensionDefinition,
    _biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> i32 {
    let sample = biome_field.sample_surface(position.as_vec2() + Vec2::splat(0.5));

    surface_height_from_sample(position, dimension, biome_field, &sample)
}

pub(crate) fn surface_height_from_sample(
    position: IVec2,
    dimension: &DimensionDefinition,
    biome_field: &BiomeField,
    sample: &BiomeFieldSample<'_>,
) -> i32 {
    let horizontal = position.as_vec2();
    let height = compose_surface_height(sample.influences.iter().map(|influence| {
        let (terrain, modifiers, terrain_seed) =
            biome_field.surface_terrain(influence.surface_index);
        let sampled_height = biome_surface_height(
            horizontal,
            dimension.sea_level,
            terrain_seed,
            terrain,
            modifiers,
            influence.terrain_strength,
        );
        (
            sampled_height,
            influence.weight,
            height_influence_policy(terrain),
        )
    }));

    height.round().max(1.0) as i32
}

fn height_influence_policy(terrain: BiomeTerrain) -> HeightInfluencePolicy {
    match terrain {
        BiomeTerrain::Ocean { .. } => HeightInfluencePolicy::LowerOnly,
        _ => HeightInfluencePolicy::Blend,
    }
}

fn compose_surface_height(
    influences: impl IntoIterator<Item = (f32, f32, HeightInfluencePolicy)>,
) -> f32 {
    let mut blended_sum = 0.0_f32;
    let mut blended_weight = 0.0_f32;
    let mut unrestricted_sum = 0.0_f32;
    let mut unrestricted_weight = 0.0_f32;

    for (height, weight, policy) in influences {
        if weight <= 0.0 {
            continue;
        }
        blended_sum += height * weight;
        blended_weight += weight;
        if policy == HeightInfluencePolicy::Blend {
            unrestricted_sum += height * weight;
            unrestricted_weight += weight;
        }
    }

    if blended_weight <= f32::EPSILON {
        return 0.0;
    }

    let blended = blended_sum / blended_weight;
    if unrestricted_weight <= f32::EPSILON {
        return blended;
    }

    let height_before_lower_only = unrestricted_sum / unrestricted_weight;
    blended.min(height_before_lower_only)
}

pub(crate) fn terrain_density(surface_height: i32, world_y: i32) -> f32 {
    surface_height as f32 - (world_y as f32 + 0.5)
}

pub(crate) fn chunk_y_bounds(
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
) -> (i32, i32) {
    let maximum_offset = dimension
        .biomes
        .iter()
        .filter_map(|dimension_biome| {
            let biome_id = &dimension_biome.id;
            let biome = biomes
                .get(biome_id)
                .unwrap_or_else(|| panic!("missing biome definition: {biome_id}"));

            if biome.kind != BiomeKind::Surface {
                return None;
            }

            let terrain_offset = biome
                .terrain
                .as_ref()
                .unwrap_or_else(|| panic!("surface biome {} must define terrain", biome.id))
                .maximum_height_offset();
            let modifier_offset: f32 = biome
                .terrain_modifiers
                .iter()
                .copied()
                .map(BiomeTerrainModifier::maximum_height_offset)
                .sum();
            Some(terrain_offset + modifier_offset)
        })
        .fold(0.0_f32, f32::max)
        .max(0.0);
    let maximum_surface = dimension.sea_level as f32 + maximum_offset;
    let maximum_block_y = maximum_surface.ceil().max(1.0) as i32 - 1;
    let maximum_chunk_y = maximum_block_y.div_euclid(crate::voxel::chunk::CHUNK_SIZE as i32);

    (TERRAIN_MIN_CHUNK_Y, maximum_chunk_y)
}

fn biome_surface_height(
    position: Vec2,
    sea_level: i32,
    seed: u64,
    terrain: BiomeTerrain,
    modifiers: &[BiomeTerrainModifier],
    distribution_strength: f32,
) -> f32 {
    let sea_level = sea_level as f32;

    let base_height = match terrain {
        BiomeTerrain::Rolling {
            base_height,
            amplitude,
            scale,
            detail_amplitude,
            detail_scale,
        } => {
            let broad = fractal_noise(position * scale, seed);
            let detail = fractal_noise(position * detail_scale, seed.rotate_left(23));
            sea_level + base_height + broad * amplitude + detail * detail_amplitude
        }
        BiomeTerrain::Ocean {
            depth,
            amplitude,
            scale,
            detail_amplitude,
            detail_scale,
        } => {
            let broad = fractal_noise(position * scale, seed);
            let detail = fractal_noise(position * detail_scale, seed.rotate_left(23));
            sea_level - depth + broad * amplitude + detail * detail_amplitude
        }
        BiomeTerrain::Mountains {
            base_height,
            amplitude,
            scale,
            sharpness,
        } => {
            let noise = fractal_noise(position * scale, seed);
            let ridge = (1.0 - noise.abs()).clamp(0.0, 1.0).powf(sharpness);
            sea_level + base_height + ridge * amplitude
        }
        BiomeTerrain::Gorge {
            base_height,
            depth,
            wall_height,
            top_amplitude,
            top_scale,
            floor_amplitude,
            floor_scale,
        } => {
            let strength = smoothstep(distribution_strength.clamp(0.0, 1.0));
            let top_noise = fractal_noise(position * top_scale, seed.rotate_left(13));
            let floor_noise = fractal_noise(position * floor_scale, seed.rotate_left(41));
            let rim_shape = (1.0 - strength).powf(0.8);
            let floor_shape = strength.powf(1.35);
            sea_level
                + base_height
                + wall_height * (1.0 - strength)
                + top_noise * top_amplitude * rim_shape
                - depth * strength
                + floor_noise * floor_amplitude * floor_shape
        }
        BiomeTerrain::Alps {
            base_height,
            amplitude,
            scale,
            sharpness,
            detail_amplitude,
            detail_scale,
        } => {
            let broad = fractal_noise(position * scale, seed);
            let ridge = (1.0 - broad.abs()).clamp(0.0, 1.0).powf(sharpness);
            let detail = fractal_noise(position * detail_scale, seed.rotate_left(29));
            let jagged = (1.0 - detail.abs()).clamp(0.0, 1.0).powf(1.35);
            sea_level + base_height + ridge * amplitude + jagged * detail_amplitude
        }
        BiomeTerrain::MountainBelt {
            base_height,
            amplitude,
            scale,
            sharpness,
            detail_amplitude,
            detail_scale,
        } => {
            let broad = fractal_noise(position * scale, seed);
            let ridge = (1.0 - broad.abs()).clamp(0.0, 1.0).powf(sharpness);
            let detail = fractal_noise(position * detail_scale, seed.rotate_left(17));
            sea_level + base_height + ridge * amplitude + detail * detail_amplitude * ridge
        }
        BiomeTerrain::Volcano {
            base_height,
            height,
            crater_depth,
            crater_radius,
            irregularity,
            irregularity_scale,
            detail_irregularity,
            detail_scale,
            crater_irregularity,
        } => {
            let strength = distribution_strength.clamp(0.0, 1.0);
            let broad = fractal_noise(position * irregularity_scale, seed.rotate_left(11));
            let detail = fractal_noise(position * detail_scale, seed.rotate_left(37));
            let slope_band = 4.0 * strength * (1.0 - strength);
            let distorted_strength = (
                strength
                    + (broad * irregularity + detail * detail_irregularity) * slope_band
            )
                .clamp(0.0, 1.0);

            let crater_noise = fractal_noise(
                position * (irregularity_scale * 1.7),
                seed.rotate_left(53),
            );
            let crater_start =
                (1.0 - crater_radius + crater_noise * crater_irregularity).clamp(0.0, 0.99);
            let crater_width = (1.0 - crater_start).max(0.01);
            let crater_strength = smoothstep(
                ((distorted_strength - crater_start) / crater_width).clamp(0.0, 1.0),
            );

            sea_level
                + base_height
                + height * distorted_strength
                - crater_depth * crater_strength
        }
    };

    base_height
        + modifiers
            .iter()
            .enumerate()
            .map(|(index, modifier)| {
                terrain_modifier_height(
                    position,
                    seed.wrapping_add((index as u64 + 1).wrapping_mul(0x517c_c1b7_2722_0a95)),
                    *modifier,
                )
            })
            .sum::<f32>()
}

fn terrain_modifier_height(position: Vec2, seed: u64, modifier: BiomeTerrainModifier) -> f32 {
    match modifier {
        BiomeTerrainModifier::HeightOffset { height } => height,
        BiomeTerrainModifier::Cliffs {
            scale,
            threshold,
            height,
            edge_width,
            warp_scale,
            warp_strength,
        } => {
            let warp_position = position * warp_scale;
            let warp = Vec2::new(
                fractal_noise(warp_position, seed ^ 0x9e37_79b9_7f4a_7c15),
                fractal_noise(
                    warp_position + Vec2::new(-23.1, 41.9),
                    seed ^ 0xc2b2_ae3d_27d4_eb4f,
                ),
            ) * warp_strength;
            let value = ((fractal_noise((position + warp) * scale, seed) + 1.0) * 0.5)
                .clamp(0.0, 1.0);
            let half_edge = edge_width * 0.5;
            let lower = (threshold - half_edge).clamp(0.0, 1.0);
            let upper = (threshold + half_edge).clamp(0.0, 1.0);
            let progress = if upper > lower {
                ((value - lower) / (upper - lower)).clamp(0.0, 1.0)
            } else if value >= threshold {
                1.0
            } else {
                0.0
            };

            smoothstep(progress) * height
        }
    }
}

fn fractal_noise(position: Vec2, seed: u64) -> f32 {
    let mut value = 0.0;
    let mut normalization = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;

    for octave in 0..NOISE_OCTAVES {
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

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

fn lerp(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_only_influence_cannot_raise_unrestricted_height() {
        let composed = compose_surface_height([
            (84.0, 0.6, HeightInfluencePolicy::Blend),
            (120.0, 0.4, HeightInfluencePolicy::LowerOnly),
        ]);

        assert_eq!(composed, 84.0);
    }

    #[test]
    fn lower_only_influence_can_lower_unrestricted_height() {
        let composed = compose_surface_height([
            (140.0, 0.75, HeightInfluencePolicy::Blend),
            (80.0, 0.25, HeightInfluencePolicy::LowerOnly),
        ]);

        assert_eq!(composed, 125.0);
    }

    #[test]
    fn lower_only_terrain_without_unrestricted_neighbor_keeps_its_shape() {
        let composed = compose_surface_height([
            (72.0, 0.7, HeightInfluencePolicy::LowerOnly),
            (68.0, 0.3, HeightInfluencePolicy::LowerOnly),
        ]);

        assert!((composed - 70.8).abs() < 0.001);
    }

    #[test]
    fn ocean_uses_lower_only_height_policy() {
        let ocean = BiomeTerrain::Ocean {
            depth: 18.0,
            amplitude: 6.0,
            scale: 0.006,
            detail_amplitude: 3.0,
            detail_scale: 0.026,
        };
        let mountains = BiomeTerrain::Mountains {
            base_height: 10.0,
            amplitude: 80.0,
            scale: 0.005,
            sharpness: 2.0,
        };

        assert_eq!(height_influence_policy(ocean), HeightInfluencePolicy::LowerOnly);
        assert_eq!(height_influence_policy(mountains), HeightInfluencePolicy::Blend);
    }
}
