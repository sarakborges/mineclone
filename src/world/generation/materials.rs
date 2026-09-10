use bevy::prelude::*;

use crate::{
    content::{biome::BiomeRegistry, block::BlockRegistry, builtin_ids::GRASS_BLOCK_ID},
    voxel::{
        cell::VoxelCell,
        chunk::{CHUNK_SIZE, VoxelChunk},
        texture_rotation::TextureRotation,
    },
    world::{
        biome_field::BiomeField, generation_region::GenerationRegion,
        material_field::solid_block_id,
    },
};

use super::{
    columns::GenerationColumnSample,
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
    columns: &[GenerationColumnSample<'_>],
    density: &[f32],
    context: &MaterialPassContext<'_>,
) {
    let fallback = context
        .blocks
        .static_id(GRASS_BLOCK_ID)
        .unwrap_or_else(|| panic!("missing interned block definition: {GRASS_BLOCK_ID}"));

    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let column = &columns[column_index(local_x, local_z)];

            for local_y in 0..CHUNK_SIZE {
                if density[voxel_index(local_x, local_y, local_z)] <= 0.0 {
                    continue;
                }

                let world_position = IVec3::new(
                    chunk_origin.x + local_x as i32,
                    chunk_origin.y + local_y as i32,
                    chunk_origin.z + local_z as i32,
                );
                let sample_position = world_position.as_vec3() + Vec3::splat(0.5);
                let volume = context.biome_field.sample_volume(sample_position);
                let block_id = solid_block_id(
                    sample_position,
                    &column.surface,
                    volume.as_ref(),
                    &context.region.geology,
                    &context.region.hydrology,
                    context.biomes,
                    fallback,
                );
                let block = context
                    .blocks
                    .get(block_id)
                    .unwrap_or_else(|| panic!("missing block definition: {block_id}"));
                let rotation = texture_rotation_for(world_position, block.rotate_texture);

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
