mod placement;
mod support;

use bevy::prelude::*;

use crate::{
    content::{
        biome_structure::StructurePlacementRules,
        block::BlockRegistry,
        block_id::intern_block_id,
        structure::{StructureDefinition, StructureVoxel},
    },
    voxel::{
        cell::VoxelCell,
        chunk::{CHUNK_SIZE, VoxelChunk},
        texture_rotation::TextureRotation,
    },
};

use self::{placement::candidate_anchor, support::compute_structure_origin_y};
use super::ChunkGenerationContext;

pub(super) fn rasterize_structures(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    context: &ChunkGenerationContext<'_>,
) {
    let mut placements = context
        .biomes
        .iter()
        .flat_map(|biome| {
            biome
                .structures
                .iter()
                .map(move |biome_structure| (biome, biome_structure))
        })
        .collect::<Vec<_>>();

    placements.sort_by(|(left_biome, left_structure), (right_biome, right_structure)| {
        left_biome
            .id
            .cmp(&right_biome.id)
            .then_with(|| left_structure.id.cmp(&right_structure.id))
    });

    for (biome, biome_structure) in placements {
        let structure = context
            .structures
            .get(&biome_structure.id)
            .unwrap_or_else(|| {
                panic!(
                    "biome {} references missing structure: {}",
                    biome.id, biome_structure.id
                )
            });

        rasterize_structure_candidates(
            chunk,
            chunk_origin,
            context,
            &biome.id,
            structure,
            biome_structure.placement,
        );
    }
}

fn rasterize_structure_candidates(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    context: &ChunkGenerationContext<'_>,
    biome_id: &str,
    structure: &StructureDefinition,
    placement: StructurePlacementRules,
) {
    let chunk_size = CHUNK_SIZE as i32;
    let chunk_min = IVec2::new(chunk_origin.x, chunk_origin.z);
    let chunk_max = chunk_min + IVec2::splat(chunk_size - 1);
    let (minimum_offset, maximum_offset) = structure.horizontal_bounds();
    let voxels = structure.voxels();
    let spacing = placement.spacing;

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
            let Some(anchor) = candidate_anchor(
                context.biome_field.seed(),
                biome_id,
                structure,
                placement,
                cell,
            ) else {
                continue;
            };
            let surface_sample = context
                .biome_field
                .sample_surface(anchor.as_vec2() + Vec2::splat(0.5));

            if surface_sample.primary_id != biome_id {
                continue;
            }

            let Some(origin_y) = context.feature_fields.structure_origin_y(
                &structure.id,
                anchor,
                || compute_structure_origin_y(anchor, &voxels, context),
            ) else {
                continue;
            };
            let origin = IVec3::new(anchor.x, origin_y, anchor.y);
            rasterize_structure(
                chunk,
                chunk_origin,
                context.blocks,
                structure,
                &voxels,
                origin,
            );
        }
    }
}

fn rasterize_structure(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    blocks: &BlockRegistry,
    structure: &StructureDefinition,
    voxels: &[StructureVoxel<'_>],
    origin: IVec3,
) {
    let chunk_size = CHUNK_SIZE as i32;

    for voxel in voxels {
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
