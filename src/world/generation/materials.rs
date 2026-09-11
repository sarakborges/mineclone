use bevy::prelude::*;

use crate::{
    content::{biome::BiomeRegistry, block::BlockRegistry},
    voxel::{
        cell::VoxelCell,
        chunk::{CHUNK_SIZE, VoxelChunk},
        texture_rotation::TextureRotation,
    },
    world::{
        biome_field::BiomeField,
        generation_region::GenerationRegion,
        material_field::{MaterialFieldContext, solid_block_id},
    },
};

use super::{
    columns::GenerationColumnSample,
    density::DensityField,
    index::{column_index, voxel_index},
};

pub(super) struct MaterialPassContext<'a> {
    pub blocks: &'a BlockRegistry,
    pub biomes: &'a BiomeRegistry,
    pub biome_field: &'a BiomeField,
    pub region: &'a GenerationRegion,
}

pub(super) fn rasterize_material_pass(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    columns: &[GenerationColumnSample],
    density: &DensityField,
    context: &MaterialPassContext<'_>,
) {
    let material_field = MaterialFieldContext {
        biome_field: context.biome_field,
        geology: &context.region.geology,
        hydrology: &context.region.hydrology,
        biomes: context.biomes,
    };

    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let column = &columns[column_index(local_x, local_z)];

            for local_y in 0..CHUNK_SIZE {
                let index = voxel_index(local_x, local_y, local_z);
                if density.values[index] <= 0.0 {
                    continue;
                }

                let world_position = IVec3::new(
                    chunk_origin.x + local_x as i32,
                    chunk_origin.y + local_y as i32,
                    chunk_origin.z + local_z as i32,
                );
                let sample_position = world_position.as_vec3() + Vec3::splat(0.5);
                let surface_depth = (column.surface_height - world_position.y - 1).max(0) as u32;
                let block_id = solid_block_id(
                    sample_position,
                    &column.surface_influences,
                    surface_depth,
                    density.volume[index],
                    &material_field,
                );
                let block = context
                    .blocks
                    .get(block_id)
                    .unwrap_or_else(|| panic!("missing block definition: {block_id}"));
                let rotation =
                    TextureRotation::for_position(world_position, block.rotate_texture.any());

                chunk.set_block(
                    local_x,
                    local_y,
                    local_z,
                    Some(VoxelCell::new(block_id, rotation)),
                );
            }
        }
    }
}
