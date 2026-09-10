use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry,
        block::BlockRegistry,
        block_id::intern_block_id,
        dimension::DimensionDefinition,
        structure::{StructureDefinition, StructureRegistry},
    },
    voxel::{
        cell::VoxelCell,
        chunk::{CHUNK_SIZE, VoxelChunk},
        texture_rotation::TextureRotation,
    },
    world::{biome_field::BiomeField, terrain::surface_height},
};

pub(super) fn rasterize_structures(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    blocks: &BlockRegistry,
    biome_field: &BiomeField,
    structures: &StructureRegistry,
) {
    for structure in structures.iter() {
        rasterize_structure_candidates(
            chunk,
            chunk_origin,
            dimension,
            biomes,
            blocks,
            biome_field,
            structure,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn rasterize_structure_candidates(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    blocks: &BlockRegistry,
    biome_field: &BiomeField,
    structure: &StructureDefinition,
) {
    let chunk_size = CHUNK_SIZE as i32;
    let chunk_min = IVec2::new(chunk_origin.x, chunk_origin.z);
    let chunk_max = chunk_min + IVec2::splat(chunk_size - 1);
    let (minimum_offset, maximum_offset) = structure.horizontal_bounds();
    let spacing = structure.placement.spacing;

    let minimum_candidate = chunk_min - maximum_offset;
    let maximum_candidate = chunk_max - minimum_offset;
    let minimum_cell = IVec2::new(
        minimum_candidate.x.div_euclid(spacing) - 1,
        minimum_candidate.y.div_euclid(spacing) - 1,
    );
    let maximum_cell = IVec2::new(
        maximum_candidate.x.div_euclid(spacing) + 1,
        maximum_candidate.y.div_euclid(spacing) + 1,
    );

    for cell_z in minimum_cell.y..=maximum_cell.y {
        for cell_x in minimum_cell.x..=maximum_cell.x {
            let cell = IVec2::new(cell_x, cell_z);
            let Some(anchor) = candidate_anchor(biome_field.seed(), structure, cell) else {
                continue;
            };
            let surface_sample = biome_field.sample_surface(anchor.as_vec2() + Vec2::splat(0.5));
            let biome = biomes.get(surface_sample.primary_id).unwrap_or_else(|| {
                panic!("missing biome definition: {}", surface_sample.primary_id)
            });

            if !biome
                .structures
                .iter()
                .any(|structure_id| structure_id == &structure.id)
            {
                continue;
            }

            let ground_y = surface_height(anchor, dimension, biomes, biome_field) - 1;
            let origin = IVec3::new(anchor.x, ground_y, anchor.y);
            rasterize_structure(chunk, chunk_origin, blocks, structure, origin);
        }
    }
}

fn candidate_anchor(
    world_seed: u64,
    structure: &StructureDefinition,
    cell: IVec2,
) -> Option<IVec2> {
    let hash = placement_hash(world_seed, &structure.id, cell);
    let chance = unit_interval(hash);
    if chance >= structure.placement.chance {
        return None;
    }

    let spacing = structure.placement.spacing;
    let center = cell * spacing + IVec2::splat(spacing / 2);
    let jitter = structure.placement.jitter;
    let jitter_x = signed_jitter(hash ^ 0x517c_c1b7_2722_0a95, jitter);
    let jitter_z = signed_jitter(hash ^ 0x6eed_0e9d_a4d9_4a4f, jitter);

    Some(center + IVec2::new(jitter_x, jitter_z))
}

fn rasterize_structure(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    blocks: &BlockRegistry,
    structure: &StructureDefinition,
    origin: IVec3,
) {
    let chunk_size = CHUNK_SIZE as i32;

    for voxel in structure.voxels() {
        let world_position = origin + voxel.offset;
        let local = world_position - chunk_origin;
        if local.x < 0
            || local.y < 0
            || local.z < 0
            || local.x >= chunk_size
            || local.y >= chunk_size
            || local.z >= chunk_size
        {
            continue;
        }

        let block = blocks.get(voxel.block_id).unwrap_or_else(|| {
            panic!(
                "structure {} references missing block: {}",
                structure.id, voxel.block_id
            )
        });
        let block_id = intern_block_id(voxel.block_id);
        let texture_rotation =
            TextureRotation::for_position(world_position, block.rotate_texture.any());
        let local_x = local.x as usize;
        let local_y = local.y as usize;
        let local_z = local.z as usize;

        chunk.set_block(
            local_x,
            local_y,
            local_z,
            Some(VoxelCell::oriented(
                block_id,
                texture_rotation,
                voxel.orientation,
            )),
        );
        chunk.set_fluid(local_x, local_y, local_z, None);
    }
}

fn placement_hash(world_seed: u64, structure_id: &str, cell: IVec2) -> u64 {
    let mut hash = world_seed ^ string_hash(structure_id);
    hash ^= (cell.x as i64 as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
    hash ^= (cell.y as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    avalanche(hash)
}

fn string_hash(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;

    for byte in value.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    hash
}

fn signed_jitter(hash: u64, maximum: i32) -> i32 {
    if maximum == 0 {
        return 0;
    }

    let range = (maximum * 2 + 1) as u64;
    (avalanche(hash) % range) as i32 - maximum
}

fn unit_interval(hash: u64) -> f32 {
    let value = hash >> 11;
    (value as f64 * (1.0 / (1_u64 << 53) as f64)) as f32
}

fn avalanche(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
