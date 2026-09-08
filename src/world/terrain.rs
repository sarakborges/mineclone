use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry,
        biome_terrain::BiomeTerrain,
        block::BlockRegistry,
        dimension::DimensionDefinition,
    },
    voxel::{
        cell::VoxelCell,
        chunk::{VoxelChunk, CHUNK_SIZE},
        texture_rotation::TextureRotation,
    },
};

use super::biome_field::BiomeField;

const GRASS_BLOCK_ID: &str = "mineclone:grass";
const TERRAIN_MIN_CHUNK_Y: i32 = 0;
const NOISE_OCTAVES: usize = 4;

pub fn build_chunk(
    coord: IVec3,
    blocks: &BlockRegistry,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> VoxelChunk {
    let (min_chunk_y, max_chunk_y) = chunk_y_bounds(dimension, biomes);

    if coord.y < min_chunk_y || coord.y > max_chunk_y {
        return VoxelChunk::empty();
    }

    let grass = blocks
        .get(GRASS_BLOCK_ID)
        .unwrap_or_else(|| panic!("missing block definition: {GRASS_BLOCK_ID}"));
    let mut chunk = VoxelChunk::empty();
    let chunk_size = CHUNK_SIZE as i32;
    let chunk_origin = coord * chunk_size;

    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let world_x = chunk_origin.x + local_x as i32;
            let world_z = chunk_origin.z + local_z as i32;
            let column_height = surface_height(
                IVec2::new(world_x, world_z),
                dimension,
                biomes,
                biome_field,
            );

            for local_y in 0..CHUNK_SIZE {
                let world_y = chunk_origin.y + local_y as i32;

                if world_y >= column_height {
                    continue;
                }

                let world_position = IVec3::new(world_x, world_y, world_z);
                let rotation = texture_rotation_for(world_position, grass.rotate_texture);
                chunk.set_block(
                    local_x,
                    local_y,
                    local_z,
                    Some(VoxelCell::new(GRASS_BLOCK_ID, rotation)),
                );
            }
        }
    }

    chunk
}

pub fn surface_height(
    position: IVec2,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> i32 {
    let sample = biome_field.sample(position.as_vec2() + Vec2::splat(0.5));
    let mut height = 0.0;

    for influence in sample.influences {
        let biome = biomes
            .get(influence.id)
            .unwrap_or_else(|| panic!("missing biome definition: {}", influence.id));
        height += biome_surface_height(position.as_vec2(), dimension.sea_level, biome.id.as_str(), &biome.terrain)
            * influence.weight;
    }

    height.round().max(1.0) as i32
}

pub fn chunk_y_bounds(
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
) -> (i32, i32) {
    let maximum_offset = dimension
        .biomes
        .iter()
        .map(|biome_id| {
            biomes
                .get(biome_id)
                .unwrap_or_else(|| panic!("missing biome definition: {biome_id}"))
                .terrain
                .maximum_height_offset()
        })
        .fold(0.0_f32, f32::max)
        .max(0.0);
    let maximum_surface = dimension.sea_level as f32 + maximum_offset;
    let maximum_block_y = maximum_surface.ceil().max(1.0) as i32 - 1;
    let maximum_chunk_y = maximum_block_y.div_euclid(CHUNK_SIZE as i32);

    (TERRAIN_MIN_CHUNK_Y, maximum_chunk_y)
}

fn biome_surface_height(
    position: Vec2,
    sea_level: i32,
    biome_id: &str,
    terrain: &BiomeTerrain,
) -> f32 {
    let seed = string_hash(biome_id);
    let sea_level = sea_level as f32;

    match *terrain {
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
        BiomeTerrain::Ocean {
            floor_depth,
            amplitude,
            scale,
        } => {
            let floor = fractal_noise(position * scale, seed);
            sea_level - floor_depth + floor * amplitude
        }
    }
}

fn fractal_noise(position: Vec2, seed: u64) -> f32 {
    let mut value = 0.0;
    let mut normalization = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;

    for octave in 0..NOISE_OCTAVES {
        let octave_seed = seed.wrapping_add((octave as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
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
    hash ^= (x as i64 as u64).wrapping_mul(0x9E37_79B1_85EB_CA87);
    hash ^= (z as i64 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    hash ^= hash >> 33;
    let normalized = (hash & 0xFFFF) as f32 / u16::MAX as f32;

    normalized * 2.0 - 1.0
}

fn string_hash(value: &str) -> u64 {
    let mut hash = 0xCBF2_9CE4_8422_2325_u64;

    for byte in value.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }

    hash
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

fn lerp(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
}

fn texture_rotation_for(position: IVec3, enabled: bool) -> TextureRotation {
    if !enabled {
        return TextureRotation::default();
    }

    let mut hash = position.x as u32;
    hash ^= (position.y as u32).wrapping_mul(0x9e37_79b9);
    hash = hash.rotate_left(13);
    hash ^= (position.z as u32).wrapping_mul(0x85eb_ca6b);
    hash ^= hash >> 16;

    TextureRotation::from_quarter_turn((hash & 3) as u8)
}
