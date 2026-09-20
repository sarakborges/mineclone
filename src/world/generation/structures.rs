mod placement;
mod restrictions;
mod support;

use std::{cmp::Ordering, collections::HashSet};

use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry,
        biome_structure::StructurePlacementRules,
        block::BlockRegistry,
        structure::{StructureDefinition, StructureRegistry, StructureVoxel},
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
use super::{super::biome_field::BiomeField, ChunkGenerationContext};

#[derive(Clone, Copy)]
struct StructureCandidate<'a> {
    biome_id: &'a str,
    structure: &'a StructureDefinition,
    anchor: IVec2,
    origin_y: i32,
    minimum: IVec2,
    maximum: IVec2,
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
    let mut direct_candidates = Vec::new();

    for biome_structure in context.biomes.structure_placements() {
        let structure = structure_for_placement(context.structures, biome_structure);
        collect_structure_candidates(
            chunk_min,
            chunk_max,
            context,
            &biome_structure.biome_id,
            structure,
            biome_structure.placement,
            &mut direct_candidates,
        );
    }

    if direct_candidates.is_empty() {
        return Vec::new();
    }

    // Conflict resolution must see a higher-priority candidate even when that
    // candidate itself lies outside this chunk. Search only structures that can
    // actually overlap each direct candidate instead of expanding every
    // placement by the registry's largest structure.
    let mut competitors = direct_candidates.clone();
    let mut seen = competitors
        .iter()
        .map(candidate_identity)
        .collect::<HashSet<_>>();
    for direct in direct_candidates.iter().copied() {
        for biome_structure in context.biomes.structure_placements() {
            let structure = structure_for_placement(context.structures, biome_structure);
            if structure.priority < direct.structure.priority
                || !structures_may_conflict(structure, direct.structure)
            {
                continue;
            }

            let mut overlapping = Vec::new();
            collect_structure_candidates(
                direct.minimum,
                direct.maximum,
                context,
                &biome_structure.biome_id,
                structure,
                biome_structure.placement,
                &mut overlapping,
            );
            for candidate in overlapping {
                if seen.insert(candidate_identity(&candidate)) {
                    competitors.push(candidate);
                }
            }
        }
    }

    let mut accepted = direct_candidates
        .into_iter()
        .filter(|candidate| {
            !competitors.iter().any(|other| {
                !same_candidate(other, candidate)
                    && candidate_outranks(other, candidate)
                    && candidates_conflict(other, candidate)
            })
        })
        .collect::<Vec<_>>();
    accepted.sort_by(candidate_order);
    accepted
}

pub(super) fn maximum_potential_structure_height_for_chunk(
    horizontal_chunk: IVec2,
    biomes: &BiomeRegistry,
    structures: &StructureRegistry,
    biome_field: &BiomeField,
) -> i32 {
    let chunk_size = CHUNK_SIZE as i32;
    let chunk_min = horizontal_chunk * chunk_size;
    let chunk_max = chunk_min + IVec2::splat(chunk_size - 1);
    let mut maximum_height = 0;

    for biome_structure in biomes.structure_placements() {
        let structure = structures
            .get(&biome_structure.structure_id)
            .unwrap_or_else(|| {
                panic!(
                    "biome {} references missing structure: {}",
                    biome_structure.biome_id, biome_structure.structure_id
                )
            });

        visit_candidate_anchors_intersecting(
            chunk_min,
            chunk_max,
            biome_field.seed(),
            &biome_structure.biome_id,
            structure,
            biome_structure.placement,
            |anchor| {
                if biome_field
                    .sample_surface(anchor.as_vec2() + Vec2::splat(0.5))
                    .primary_id
                    == biome_structure.biome_id
                {
                    maximum_height = maximum_height.max(structure.max_y_offset().max(0));
                }
            },
        );
    }

    maximum_height
}

fn structure_for_placement<'a>(
    structures: &'a StructureRegistry,
    placement: &crate::content::biome::BiomeStructurePlacement,
) -> &'a StructureDefinition {
    structures.get(&placement.structure_id).unwrap_or_else(|| {
        panic!(
            "biome {} references missing structure: {}",
            placement.biome_id, placement.structure_id
        )
    })
}

fn collect_structure_candidates<'a>(
    target_min: IVec2,
    target_max: IVec2,
    context: &'a ChunkGenerationContext<'_>,
    biome_id: &'a str,
    structure: &'a StructureDefinition,
    placement: StructurePlacementRules,
    candidates: &mut Vec<StructureCandidate<'a>>,
) {
    let (minimum_offset, maximum_offset) = structure.horizontal_bounds();
    visit_candidate_anchors_intersecting(
        target_min,
        target_max,
        context.biome_field.seed(),
        biome_id,
        structure,
        placement,
        |anchor| {
            let minimum = anchor + minimum_offset;
            let maximum = anchor + maximum_offset;
            let surface_sample = context
                .biome_field
                .sample_surface(anchor.as_vec2() + Vec2::splat(0.5));
            if surface_sample.primary_id != biome_id {
                return;
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
                return;
            };

            candidates.push(StructureCandidate {
                biome_id,
                structure,
                anchor,
                origin_y,
                minimum,
                maximum,
            });
        },
    );
}

fn visit_candidate_anchors_intersecting(
    target_min: IVec2,
    target_max: IVec2,
    world_seed: u64,
    biome_id: &str,
    structure: &StructureDefinition,
    placement: StructurePlacementRules,
    mut visit: impl FnMut(IVec2),
) {
    let (minimum_offset, maximum_offset) = structure.horizontal_bounds();
    let minimum_candidate = target_min - maximum_offset;
    let maximum_candidate = target_max - minimum_offset;
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
            let Some(anchor) =
                candidate_anchor(world_seed, biome_id, structure, placement, cell)
            else {
                continue;
            };
            let minimum = anchor + minimum_offset;
            let maximum = anchor + maximum_offset;
            if rectangles_overlap(minimum, maximum, target_min, target_max) {
                visit(anchor);
            }
        }
    }
}

fn candidate_identity<'a>(
    candidate: &StructureCandidate<'a>,
) -> (&'a str, &'a str, IVec2) {
    (candidate.biome_id, candidate.structure.id.as_str(), candidate.anchor)
}

fn structures_may_conflict(
    higher: &StructureDefinition,
    lower: &StructureDefinition,
) -> bool {
    higher.generation.reserve_space
        || higher.conflict_groups.iter().any(|group| {
            lower
                .conflict_groups
                .iter()
                .any(|candidate| candidate == group)
        })
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
    origin: IVec3,
) {
    if structure.generation.fluid_policy == StructureFluidPolicy::Forbid
        && visit_structure_voxels_in_chunk(
            structure,
            origin,
            context.chunk_origin,
            |_, _, local| {
                context
                    .base_chunk
                    .fluid_at(local.x, local.y, local.z)
                    .is_some()
            },
        )
    {
        return;
    }

    visit_structure_voxels_in_chunk(
        structure,
        origin,
        context.chunk_origin,
        |voxel, world_position, local| {
            let local_x = local.x as usize;
            let local_y = local.y as usize;
            let local_z = local.z as usize;
            let index = local_x + local_z * CHUNK_SIZE + local_y * CHUNK_SIZE * CHUNK_SIZE;
            let can_replace = match structure.generation.replace_policy {
                StructureReplacePolicy::Any => true,
                StructureReplacePolicy::AirOnly => {
                    !claimed[index]
                        && context
                            .base_chunk
                            .cell_at(local.x, local.y, local.z)
                            .is_none()
                }
                StructureReplacePolicy::Terrain => !claimed[index],
            };
            if !can_replace {
                return false;
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
            false
        },
    );
}

fn visit_structure_voxels_in_chunk(
    structure: &StructureDefinition,
    origin: IVec3,
    chunk_origin: IVec3,
    mut visit: impl FnMut(&StructureVoxel, IVec3, IVec3) -> bool,
) -> bool {
    let chunk_size = CHUNK_SIZE as i32;
    let origin_horizontal = origin.xz();
    let chunk_horizontal = chunk_origin.xz();

    for local_z in 0..chunk_size {
        for local_x in 0..chunk_size {
            let world_horizontal = chunk_horizontal + IVec2::new(local_x, local_z);
            let structure_offset = world_horizontal - origin_horizontal;
            for voxel in structure.column_voxels(structure_offset) {
                let world_y = origin.y + voxel.offset.y;
                let local_y = world_y - chunk_origin.y;
                if local_y < 0 {
                    continue;
                }
                if local_y >= chunk_size {
                    break;
                }

                let local = IVec3::new(local_x, local_y, local_z);
                let world_position =
                    IVec3::new(world_horizontal.x, world_y, world_horizontal.y);
                if visit(voxel, world_position, local) {
                    return true;
                }
            }
        }
    }

    false
}
