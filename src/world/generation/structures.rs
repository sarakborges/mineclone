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
        dimension::DimensionDefinition,
        layer::LayerFace,
        structure::{StructureDefinition, StructureRegistry, StructureVoxel},
        structure_rules::{StructureFluidPolicy, StructureReplacePolicy},
    },
    voxel::{
        cell::VoxelCell,
        chunk::{CHUNK_SIZE, CHUNK_VOLUME, VoxelChunk, VoxelChunkContentMut},
        layer::LayerCell,
        texture_rotation::TextureRotation,
    },
    world::terrain::surface_height,
};

pub(crate) use self::support::fit_structure_to_ground;
use self::{
    placement::{candidate_anchor, structure_member_hash},
    restrictions::candidate_satisfies_restrictions,
    support::{MAX_STRUCTURE_GROUND_RISE, compute_structure_origin_y},
};
use super::{super::biome_field::BiomeField, ChunkGenerationContext};

const COLUMN_INDEX_MIN_VOXELS: usize = 512;

#[derive(Clone, Copy)]
struct StructureCandidate<'a> {
    biome_id: &'a str,
    placement_id: &'a str,
    structure: &'a StructureDefinition,
    anchor: IVec2,
    origin_y: i32,
    minimum: IVec2,
    maximum: IVec2,
}

#[derive(Clone, Copy)]
struct StructurePlacementContext<'a> {
    biome_id: &'a str,
    placement_id: &'a str,
    placement: StructurePlacementRules,
}

struct StructureRasterizationContext<'a> {
    base_chunk: &'a VoxelChunk,
    blocks: &'a BlockRegistry,
    chunk_origin: IVec3,
    world_seed: u64,
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
                    world_seed: context.biome_field.seed(),
                },
                candidate.structure,
                IVec3::new(candidate.anchor.x, candidate.origin_y, candidate.anchor.y),
            );
        }
    });
}

pub(crate) fn structure_candidate_anchor(
    world_seed: u64,
    biome_id: &str,
    structure_reference: &str,
    placement: StructurePlacementRules,
    cell: IVec2,
) -> Option<IVec2> {
    candidate_anchor(
        world_seed,
        biome_id,
        structure_reference,
        placement,
        cell,
    )
}

pub(crate) fn located_structure_origins_in_chunk(
    horizontal_chunk: IVec2,
    structure_id: &str,
    context: &ChunkGenerationContext<'_>,
) -> Vec<IVec3> {
    let chunk_size = CHUNK_SIZE as i32;
    let chunk_origin = IVec3::new(
        horizontal_chunk.x * chunk_size,
        0,
        horizontal_chunk.y * chunk_size,
    );

    resolved_structure_candidates_matching(chunk_origin, context, Some(structure_id))
        .into_iter()
        .map(|candidate| {
            IVec3::new(
                candidate.anchor.x,
                candidate.origin_y,
                candidate.anchor.y,
            )
        })
        .collect()
}

fn resolved_structure_candidates<'a>(
    chunk_origin: IVec3,
    context: &'a ChunkGenerationContext<'_>,
) -> Vec<StructureCandidate<'a>> {
    resolved_structure_candidates_matching(chunk_origin, context, None)
}

fn resolved_structure_candidates_matching<'a>(
    chunk_origin: IVec3,
    context: &'a ChunkGenerationContext<'_>,
    only_structure_id: Option<&str>,
) -> Vec<StructureCandidate<'a>> {
    let chunk_size = CHUNK_SIZE as i32;
    let chunk_min = IVec2::new(chunk_origin.x, chunk_origin.z);
    let chunk_max = chunk_min + IVec2::splat(chunk_size - 1);
    let mut direct_candidates = Vec::new();

    for biome_structure in context.biomes.structure_placements() {
        if only_structure_id.is_some_and(|structure_id| {
            !context
                .structures
                .reference_contains_structure(&biome_structure.structure_id, structure_id)
        }) {
            continue;
        }
        collect_structure_candidates(
            chunk_min,
            chunk_max,
            context,
            StructurePlacementContext {
                biome_id: &biome_structure.biome_id,
                placement_id: &biome_structure.structure_id,
                placement: biome_structure.placement,
            },
            only_structure_id,
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
            let mut overlapping = Vec::new();
            collect_structure_candidates(
                direct.minimum,
                direct.maximum,
                context,
                StructurePlacementContext {
                    biome_id: &biome_structure.biome_id,
                    placement_id: &biome_structure.structure_id,
                    placement: biome_structure.placement,
                },
                None,
                &mut overlapping,
            );
            for candidate in overlapping {
                if candidate.structure.priority < direct.structure.priority
                    || !structures_may_conflict(candidate.structure, direct.structure)
                {
                    continue;
                }
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

pub(super) fn maximum_potential_structure_top_y_for_chunk(
    horizontal_chunk: IVec2,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    structures: &StructureRegistry,
    biome_field: &BiomeField,
) -> i32 {
    let chunk_size = CHUNK_SIZE as i32;
    let chunk_min = horizontal_chunk * chunk_size;
    let chunk_max = chunk_min + IVec2::splat(chunk_size - 1);
    let mut maximum_top_y = 0;

    for biome_structure in biomes.structure_placements() {
        let placement_id = biome_structure.structure_id.as_str();
        let bounds = structures
            .bounds_for_reference(placement_id)
            .unwrap_or_else(|| {
                panic!(
                    "biome {} references missing structure or structure group: {}",
                    biome_structure.biome_id, placement_id
                )
            });

        visit_candidate_anchors_intersecting(
            chunk_min,
            chunk_max,
            biome_field.seed(),
            StructurePlacementContext {
                biome_id: &biome_structure.biome_id,
                placement_id,
                placement: biome_structure.placement,
            },
            bounds,
            |anchor| {
                let member_hash = structure_member_hash(
                    biome_field.seed(),
                    &biome_structure.biome_id,
                    placement_id,
                    anchor,
                );
                let structure = structures
                    .select_for_reference(placement_id, member_hash)
                    .unwrap_or_else(|| {
                        panic!(
                            "biome {} references empty structure group: {}",
                            biome_structure.biome_id, placement_id
                        )
                    });
                let (minimum_offset, maximum_offset) = structure.horizontal_bounds();
                if !rectangles_overlap(
                    anchor + minimum_offset,
                    anchor + maximum_offset,
                    chunk_min,
                    chunk_max,
                ) {
                    return;
                }
                if biome_field
                    .sample_surface(anchor.as_vec2() + Vec2::splat(0.5))
                    .primary_id
                    != biome_structure.biome_id
                {
                    return;
                }

                let support_offset = *structure
                    .support_offsets()
                    .first()
                    .expect("validated non-empty structure must have a support offset");
                let support_surface = surface_height(
                    anchor + support_offset,
                    dimension,
                    biomes,
                    biome_field,
                );
                // fit_structure_to_ground chooses the minimum ground among all
                // support points. Therefore any single support gives a safe
                // upper bound for the origin. Add the maximum density rise used
                // by support fitting, then convert that bound into an absolute
                // structure top Y. This stays conservative without resolving
                // every expensive placement restriction during streaming.
                let conservative_ground_y =
                    support_surface - 1 + MAX_STRUCTURE_GROUND_RISE;
                let conservative_origin_y =
                    conservative_ground_y - structure.min_y_offset();
                maximum_top_y = maximum_top_y.max(
                    conservative_origin_y + structure.max_y_offset(),
                );
            },
        );
    }

    maximum_top_y
}

fn collect_structure_candidates<'a>(
    target_min: IVec2,
    target_max: IVec2,
    context: &'a ChunkGenerationContext<'_>,
    placement_context: StructurePlacementContext<'a>,
    only_structure_id: Option<&str>,
    candidates: &mut Vec<StructureCandidate<'a>>,
) {
    let biome_id = placement_context.biome_id;
    let placement_id = placement_context.placement_id;
    let bounds = context
        .structures
        .bounds_for_reference(placement_id)
        .unwrap_or_else(|| {
            panic!(
                "biome {biome_id} references missing structure or structure group: {placement_id}"
            )
        });

    visit_candidate_anchors_intersecting(
        target_min,
        target_max,
        context.biome_field.seed(),
        placement_context,
        bounds,
        |anchor| {
            let member_hash = structure_member_hash(
                context.biome_field.seed(),
                biome_id,
                placement_id,
                anchor,
            );
            let structure = context
                .structures
                .select_for_reference(placement_id, member_hash)
                .unwrap_or_else(|| {
                    panic!(
                        "biome {biome_id} references empty structure group: {placement_id}"
                    )
                });
            if only_structure_id.is_some_and(|structure_id| structure.id != structure_id) {
                return;
            }

            let (minimum_offset, maximum_offset) = structure.horizontal_bounds();
            let minimum = anchor + minimum_offset;
            let maximum = anchor + maximum_offset;
            if !rectangles_overlap(minimum, maximum, target_min, target_max) {
                return;
            }

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
                placement_id,
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
    placement_context: StructurePlacementContext<'_>,
    bounds: (IVec2, IVec2),
    mut visit: impl FnMut(IVec2),
) {
    let StructurePlacementContext {
        biome_id,
        placement_id: structure_reference,
        placement,
    } = placement_context;
    let (minimum_offset, maximum_offset) = bounds;
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
            let Some(anchor) = candidate_anchor(
                world_seed,
                biome_id,
                structure_reference,
                placement,
                cell,
            )
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
    (candidate.biome_id, candidate.placement_id, candidate.anchor)
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
    left.placement_id == right.placement_id
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
            rasterize_structure_surface_layers(
                chunk,
                structure,
                voxel,
                world_position,
                local,
                context.world_seed,
            );
            if structure.generation.fluid_policy == StructureFluidPolicy::Displace {
                chunk.set_fluid(local_x, local_y, local_z, None);
            }
            claimed[index] = true;
            false
        },
    );
}

fn rasterize_structure_surface_layers(
    chunk: &mut VoxelChunkContentMut<'_>,
    structure: &StructureDefinition,
    voxel: &StructureVoxel,
    world_position: IVec3,
    local: IVec3,
    world_seed: u64,
) {
    // Structure generation is deterministic across chunk order. Use the
    // world seed plus world-space position and structure/layer/face identity
    // so the same seed and structure always choose the same layer patches.
    for surface in structure.surface_layers_for_voxel(voxel) {
        for &face in &surface.faces {
            let hash = surface_layer_hash(
                world_seed,
                &structure.id,
                &surface.layer,
                world_position,
                face,
            );
            if unit_interval(hash) >= surface.chance {
                continue;
            }
            let rotation = TextureRotation::from_quarter_turn(((hash >> 32) & 3) as u8);
            let _ = chunk.add_layer(
                local.x as usize,
                local.y as usize,
                local.z as usize,
                face,
                LayerCell::new(&surface.layer, rotation),
            );
        }
    }
}

fn surface_layer_hash(
    world_seed: u64,
    structure_id: &str,
    layer_id: &str,
    position: IVec3,
    face: LayerFace,
) -> u64 {
    let mut hash = world_seed
        ^ stable_string_hash(structure_id).rotate_left(11)
        ^ stable_string_hash(layer_id).rotate_left(37);
    hash ^= (position.x as i64 as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
    hash ^= (position.y as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    hash ^= (position.z as i64 as u64).wrapping_mul(0x1656_67b1_9e37_79f9);
    hash ^= (face.index() as u64 + 1).wrapping_mul(0xd6e8_feb8_6659_fd93);
    avalanche(hash)
}

fn stable_string_hash(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in value.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn avalanche(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn unit_interval(hash: u64) -> f32 {
    let value = hash >> 11;
    (value as f64 * (1.0 / (1_u64 << 53) as f64)) as f32
}

fn visit_structure_voxels_in_chunk(
    structure: &StructureDefinition,
    origin: IVec3,
    chunk_origin: IVec3,
    mut visit: impl FnMut(&StructureVoxel, IVec3, IVec3) -> bool,
) -> bool {
    let chunk_size = CHUNK_SIZE as i32;
    if structure.voxels().len() < COLUMN_INDEX_MIN_VOXELS {
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
            if visit(voxel, world_position, local) {
                return true;
            }
        }
        return false;
    }

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
