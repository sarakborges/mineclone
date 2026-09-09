use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry,
        block::BlockRegistry,
        dimension::DimensionDefinition,
        fluid::{FluidId, FluidRegistry},
    },
    voxel::{
        cell::VoxelCell,
        chunk::{VoxelChunk, CHUNK_SIZE},
        fluid::FluidCell,
        texture_rotation::TextureRotation,
    },
};

use super::{
    biome_field::{BiomeField, BiomeFieldSample},
    terrain::{chunk_y_bounds, surface_height_from_sample, terrain_density},
};

const GRASS_BLOCK_ID: &str = "mineclone:grass";

struct GenerationColumnSample {
    surface_height: i32,
    surface_fluid: Option<FluidId>,
}

pub(crate) fn generate_chunk(
    coord: IVec3,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
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
    let chunk_origin = coord * CHUNK_SIZE as i32;
    let columns = sample_generation_columns(
        chunk_origin,
        fluids,
        dimension,
        biomes,
        biome_field,
    );
    let mut chunk = VoxelChunk::empty();

    rasterize_solid_pass(
        &mut chunk,
        chunk_origin,
        &columns,
        grass.rotate_texture,
    );
    rasterize_fluid_pass(&mut chunk, chunk_origin, &columns, dimension.sea_level);

    chunk
}

fn sample_generation_columns(
    chunk_origin: IVec3,
    fluids: &FluidRegistry,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> Vec<GenerationColumnSample> {
    let mut columns = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE);

    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let world_x = chunk_origin.x + local_x as i32;
            let world_z = chunk_origin.z + local_z as i32;
            let position = IVec2::new(world_x, world_z);
            let sample = biome_field.sample(position.as_vec2() + Vec2::splat(0.5));

            columns.push(GenerationColumnSample {
                surface_height: surface_height_from_sample(
                    position,
                    dimension,
                    biomes,
                    biome_field.seed(),
                    &sample,
                ),
                surface_fluid: surface_fluid_from_sample(&sample, biomes, fluids),
            });
        }
    }

    columns
}

fn rasterize_solid_pass(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    columns: &[GenerationColumnSample],
    rotate_texture: bool,
) {
    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let column = &columns[column_index(local_x, local_z)];
            let world_x = chunk_origin.x + local_x as i32;
            let world_z = chunk_origin.z + local_z as i32;

            for local_y in 0..CHUNK_SIZE {
                let world_y = chunk_origin.y + local_y as i32;

                if terrain_density(column.surface_height, world_y) <= 0.0 {
                    continue;
                }

                let world_position = IVec3::new(world_x, world_y, world_z);
                let rotation = texture_rotation_for(world_position, rotate_texture);
                chunk.set_block(
                    local_x,
                    local_y,
                    local_z,
                    Some(VoxelCell::new(GRASS_BLOCK_ID, rotation)),
                );
            }
        }
    }
}

fn rasterize_fluid_pass(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    columns: &[GenerationColumnSample],
    sea_level: i32,
) {
    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let column = &columns[column_index(local_x, local_z)];
            let Some(fluid_id) = column.surface_fluid else {
                continue;
            };

            for local_y in 0..CHUNK_SIZE {
                let world_y = chunk_origin.y + local_y as i32;

                if world_y >= sea_level
                    || terrain_density(column.surface_height, world_y) > 0.0
                {
                    continue;
                }

                chunk.set_fluid(
                    local_x,
                    local_y,
                    local_z,
                    Some(FluidCell::source(fluid_id)),
                );
            }
        }
    }
}

fn surface_fluid_from_sample(
    sample: &BiomeFieldSample<'_>,
    biomes: &BiomeRegistry,
    fluids: &FluidRegistry,
) -> Option<FluidId> {
    sample
        .influences
        .iter()
        .filter_map(|influence| {
            let biome = biomes
                .get(influence.id)
                .unwrap_or_else(|| panic!("missing biome definition: {}", influence.id));
            let fluid_id = biome.surface_fluid.as_deref()?;
            let fluid = fluids.id_of(fluid_id).unwrap_or_else(|| {
                panic!(
                    "biome {} references missing surface fluid: {fluid_id}",
                    biome.id
                )
            });

            Some((fluid, influence.weight))
        })
        .max_by(|(_, left_weight), (_, right_weight)| left_weight.total_cmp(right_weight))
        .map(|(fluid_id, _)| fluid_id)
}

fn column_index(x: usize, z: usize) -> usize {
    x + z * CHUNK_SIZE
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
