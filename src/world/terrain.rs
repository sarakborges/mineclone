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

pub fn surface_height(
    position: IVec2,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> i32 {
    let sample = biome_field.sample_surface(position.as_vec2() + Vec2::splat(0.5));

    surface_height_from_sample(position, dimension, biomes, biome_field.seed(), &sample)
}

pub(crate) fn surface_height_from_sample(
    position: IVec2,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    world_seed: u64,
    sample: &BiomeFieldSample<'_>,
) -> i32 {
    let mut height = 0.0;

    for influence in &sample.influences {
        let biome = biomes
            .get(influence.id)
            .unwrap_or_else(|| panic!("missing biome definition: {}", influence.id));
        let terrain = biome
            .terrain
            .as_ref()
            .unwrap_or_else(|| panic!("surface biome {} must define terrain", biome.id));
        height += biome_surface_height(
            position.as_vec2(),
            dimension.sea_level,
            world_seed,
            biome.id.as_str(),
            terrain,
            &biome.terrain_modifiers,
        ) * influence.weight;
    }

    height.round().max(1.0) as i32
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
    world_seed: u64,
    biome_id: &str,
    terrain: &BiomeTerrain,
    modifiers: &[BiomeTerrainModifier],
) -> f32 {
    let seed = mix_seed(world_seed ^ string_hash(biome_id));
    let sea_level = sea_level as f32;

    let base_height = match *terrain {
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
