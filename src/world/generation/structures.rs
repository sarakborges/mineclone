mod placement;
mod restrictions;
mod support;

use std::cmp::Ordering;

use bevy::prelude::*;

use crate::{
    content::{
        biome_structure::StructurePlacementRules,
        block::BlockRegistry,
        structure::{StructureDefinition, StructureVoxel},
        structure_rules::{StructureFluidPolicy, StructureReplacePolicy},
    },
    voxel::{
        cell::VoxelCell,
        chunk::{CHUNK_SIZE, CHUNK_VOLUME, VoxelChunk, VoxelChunkContentMut},
        texture_rotation::TextureRotation,
    },
};

pub(crate) use self::support::fit_structure_to_ground;
use self::{
    placement::candidate_anchor,
    restrictions::candidate_satisfies_restrictions,
    support::compute_structure_origin_y,
};
use super::ChunkGenerationContext;

#[derive(Clone, Copy)]
struct StructureCandidate<'a> {
    biome_id: &'a str,
    structure: &'a StructureDefinition,
    anchor: IVec2,
    origin_y: i32,
    minimum: IVec2,
    maximum: IVec2,
}

impl StructureCandidate<'_> {
    fn intersects(&self, minimum: IVec2, maximum: IVec2) -> bool {
        rectangles_overlap(self.minimum, self.maximum, minimum, maximum)
    }
}

struct StructureRasterizationContext<'a> {
    base_chunk: &'a VoxelChunk,
    blocks: &'a BlockRegistry,
    chunk_origin: IVec3,
}

pub(super) fn rasterize_structures(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    context: &ChunkGenerationContext<'_>,
) {
    let candidates = resolved_structure_candidates(chunk_origin, context);
    if candidates.is_empty() {
        return;
    }

    let base_chunk = chunk.clone();
    let mut claimed = vec![false; CHUNK_VOLUME];
    chunk.edit_content(|chunk| {
        for candidate in candidates {
            rasterize_structure(
                chunk,
                &mut claimed,
                &StructureRasterizationContext {
                    base_chunk: &base_chunk,
                    blocks: context.blocks,
                    chunk_origin,
                },
                candidate.structure,
                candidate.structure.voxels(),
                IVec3::new(candidate.anchor.x, candidate.origin_y, candidate.anchor.y),
            );
        }
    });
}

fn resolved_structure_candidates<'a>(
    chunk_origin: IVec3,
    context: &'a ChunkGenerationContext<'_>,
) -> Vec<StructureCandidate<'a>> {
    let chunk_size = CHUNK_SIZE as i32;
    let chunk_min = IVec2::new(chunk_origin.x, chunk_origin.z);
    let chunk_max = chunk_min + IVec2::splat(chunk_size - 1);
    let resolution_margin = context.structures.max_horizontal_extent_from_anchor();
    let query_min = chunk_min - IVec2::splat(resolution_margin);
    let query_max = chunk_max + IVec2::splat(resolution_margin);
    let mut candidates = Vec::new();

    for biome_structure in context.biomes.structure_placements() {
        let structure = context
            .structures
            .get(&biome_structure.structure_id)
            .unwrap_or_else(|| {
                panic!(
                    "biome {} references missing structure: {}",
                    biome_structure.biome_id, biome_structure.structure_id
                )
            });

        collect_structure_candidates(
            query_min,
            query_max,
            context,
            &biome_structure.biome_id,
            structure,
            biome_structure.placement,
            &mut candidates,
        );
    }

    let mut accepted = candidates
        .iter()
        .copied()
        .filter(|candidate| candidate.intersects(chunk_min, chunk_max))
        .filter(|candidate| {
            !candidates.iter().any(|other| {
                !same_candidate(other, candidate)
                    && candidate_outranks(other, candidate)
                    && candidates_conflict(other, candidate)
            })
        })
        .collect::<Vec<_>>();
    accepted.sort_by(candidate_order);
    accepted
}

fn collect_structure_candidates<'a>(
    query_min: IVec2,
    query_max: IVec2,
    context: &'a ChunkGenerationContext<'_>,
    biome_id: &'a str,
    structure: &'a StructureDefinition,
    placement: StructurePlacementRules,
    candidates: &mut Vec<StructureCandidate<'a>>,
) {
    let (minimum_offset, maximum_offset) = structure.horizontal_bounds();
    let minimum_candidate = query_min - maximum_offset;
    let maximum_candidate = query_max - minimum_offset;
    let spacing = placement.spacing;
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
                || {
                    let origin_y = compute_structure_origin_y(anchor, structure, context)?;
                    candidate_satisfies_restrictions(
                        biome_id,
                        structure,
                        anchor,
                        origin_y,
                        context,
                    )
                    .then_some(origin_y)
                },
            ) else {
                continue;
            };

            candidates.push(StructureCandidate {
                biome_id,
                structure,
                anchor,
                origin_y,
                minimum: anchor + minimum_offset,
                maximum: anchor + maximum_offset,
            });
        }
    }
}

fn candidate_order(
    left: &StructureCandidate<'_>,
    right: &StructureCandidate<'_>,
) -> Ordering {
    right
        .structure
        .priority
        .cmp(&left.structure.priority)
        .then_with(|| left.structure.id.cmp(&right.structure.id))
        .then_with(|| left.biome_id.cmp(right.biome_id))
        .then_with(|| left.anchor.x.cmp(&right.anchor.x))
        .then_with(|| left.anchor.y.cmp(&right.anchor.y))
}

fn candidate_outranks(
    left: &StructureCandidate<'_>,
    right: &StructureCandidate<'_>,
) -> bool {
    candidate_order(left, right) == Ordering::Less
}

fn same_candidate(
    left: &StructureCandidate<'_>,
    right: &StructureCandidate<'_>,
) -> bool {
    left.structure.id == right.structure.id
        && left.biome_id == right.biome_id
        && left.anchor == right.anchor
}

fn candidates_conflict(
    higher: &StructureCandidate<'_>,
    lower: &StructureCandidate<'_>,
) -> bool {
    if !rectangles_overlap(
        higher.minimum,
        higher.maximum,
        lower.minimum,
        lower.maximum,
    ) {
        return false;
    }

    higher.structure.generation.reserve_space
        || higher.structure.conflict_groups.iter().any(|group| {
            lower
                .structure
                .conflict_groups
                .iter()
                .any(|candidate| candidate == group)
        })
}

fn rectangles_overlap(
    left_min: IVec2,
    left_max: IVec2,
    right_min: IVec2,
    right_max: IVec2,
) -> bool {
    left_min.x <= right_max.x
        && left_max.x >= right_min.x
        && left_min.y <= right_max.y
        && left_max.y >= right_min.y
}

fn rasterize_structure(
    chunk: &mut VoxelChunkContentMut<'_>,
    claimed: &mut [bool],
    context: &StructureRasterizationContext<'_>,
    structure: &StructureDefinition,
    voxels: &[StructureVoxel],
    origin: IVec3,
) {
    let chunk_size = CHUNK_SIZE as i32;

    if structure.generation.fluid_policy == StructureFluidPolicy::Forbid
        && voxels.iter().any(|voxel| {
            let local = origin + voxel.offset - context.chunk_origin;
            local.x >= 0
                && local.y >= 0
                && local.z >= 0
                && local.x < chunk_size
                && local.y < chunk_size
                && local.z < chunk_size
                && context.base_chunk.fluid_at(local.x, local.y, local.z).is_some()
        })
    {
        return;
    }

    for voxel in voxels {
        let world_position = origin + voxel.offset;
        let local = world_position - context.chunk_origin;
        if local.x < 0
            || local.y < 0
            || local.z < 0
            || local.x >= chunk_size
            || local.y >= chunk_size
            || local.z >= chunk_size
        {
            continue;
        }

        let local_x = local.x as usize;
        let local_y = local.y as usize;
        let local_z = local.z as usize;
        let index = local_x + local_z * CHUNK_SIZE + local_y * CHUNK_SIZE * CHUNK_SIZE;
        let can_replace = match structure.generation.replace_policy {
            StructureReplacePolicy::Any => true,
            StructureReplacePolicy::AirOnly => {
                !claimed[index] && context.base_chunk.cell_at(local.x, local.y, local.z).is_none()
            }
            StructureReplacePolicy::Terrain => !claimed[index],
        };
        if !can_replace {
            continue;
        }

        let block = context.blocks.get(voxel.block_id).unwrap_or_else(|| {
            panic!(
                "structure {} references missing block: {}",
                structure.id, voxel.block_id
            )
        });
        let texture_rotation =
            TextureRotation::for_position(world_position, block.rotate_texture.any());

        chunk.set_block(
            local_x,
            local_y,
            local_z,
            Some(VoxelCell::oriented(
                voxel.block_id,
                texture_rotation,
                voxel.orientation,
            )),
        );
        if structure.generation.fluid_policy == StructureFluidPolicy::Displace {
            chunk.set_fluid(local_x, local_y, local_z, None);
        }
        claimed[index] = true;
    }
}
