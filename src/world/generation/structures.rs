mod placement;
mod restrictions;
mod set;
mod support;

use std::{cmp::Ordering, collections::HashSet};

use bevy::prelude::*;
use smallvec::SmallVec;

use crate::{
    content::{
        biome::BiomeRegistry,
        biome_structure::StructurePlacementRules,
        block::BlockRegistry,
        dimension::DimensionDefinition,
        layer::LayerFace,
        structure::{StructureDefinition, StructureRegistry, StructureRotation, StructureVoxel},
        structure_rules::{StructureFluidPolicy, StructureReplacePolicy},
    },
    voxel::{
        cell::VoxelCell,
        chunk::{CHUNK_SIZE, CHUNK_VOLUME, VoxelChunk},
        layer::LayerCell,
        texture_rotation::TextureRotation,
    },
    world::{
        terrain::surface_height,
        world_feature_fields::CachedStructureCandidate,
    },
};

use self::{
    placement::{candidate_anchor, structure_member_hash},
    restrictions::candidate_satisfies_restrictions,
    set::resolve_set_pieces,
    support::compute_structure_origin_y,
};
use super::{super::biome_field::BiomeField, ChunkGenerationContext};

const COLUMN_INDEX_MIN_VOXELS: usize = 512;
const STRUCTURE_OCCUPANCY_WORDS: usize = CHUNK_VOLUME.div_ceil(u64::BITS as usize);

#[derive(Clone, Copy)]
struct StructureCandidate<'a> {
    biome_id: &'a str,
    placement_id: &'a str,
    structure: &'a StructureDefinition,
    rotation: StructureRotation,
    placement_anchor: IVec2,
    anchor: IVec2,
    origin_y: i32,
    priority: i32,
    reserve_space: bool,
    conflict_groups: &'a [String],
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
    base_occupied: &'a [u64; STRUCTURE_OCCUPANCY_WORDS],
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

    let chunk_min_y = chunk_origin.y;
    let chunk_max_y = chunk_origin.y + CHUNK_SIZE as i32 - 1;
    let intersects_section = candidates.iter().any(|candidate| {
        let structure = context
            .structures
            .get(&candidate.structure_id)
            .unwrap_or_else(|| panic!("missing cached structure: {}", candidate.structure_id));
        let minimum_y = candidate.origin_y + structure.min_y_offset();
        let maximum_y = candidate.origin_y + structure.max_y_offset();
        maximum_y >= chunk_min_y && minimum_y <= chunk_max_y
    });
    if !intersects_section {
        return;
    }

    let mut base_occupied = [0_u64; STRUCTURE_OCCUPANCY_WORDS];
    for local_y in 0..CHUNK_SIZE {
        for local_z in 0..CHUNK_SIZE {
            for local_x in 0..CHUNK_SIZE {
                let index =
                    local_x + local_z * CHUNK_SIZE + local_y * CHUNK_SIZE * CHUNK_SIZE;
                if chunk
                    .cell_at(local_x as i32, local_y as i32, local_z as i32)
                    .is_some()
                {
                    bit_set(&mut base_occupied, index);
                }
            }
        }
    }

    let mut claimed = [0_u64; STRUCTURE_OCCUPANCY_WORDS];
    for candidate in candidates.iter() {
        let structure = context
            .structures
            .get(&candidate.structure_id)
            .unwrap_or_else(|| panic!("missing cached structure: {}", candidate.structure_id));
        let minimum_y = candidate.origin_y + structure.min_y_offset();
        let maximum_y = candidate.origin_y + structure.max_y_offset();
        if maximum_y < chunk_min_y || minimum_y > chunk_max_y {
            continue;
        }

        rasterize_structure(
            chunk,
            &mut claimed,
            &StructureRasterizationContext {
                base_occupied: &base_occupied,
                blocks: context.blocks,
                chunk_origin,
                world_seed: context.biome_field.seed(),
            },
            structure,
            candidate.rotation,
            IVec3::new(candidate.anchor.x, candidate.origin_y, candidate.anchor.y),
        );
    }
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

pub(crate) fn structure_candidate_member_hash(
    world_seed: u64,
    biome_id: &str,
    structure_reference: &str,
    anchor: IVec2,
) -> u64 {
    structure_member_hash(world_seed, biome_id, structure_reference, anchor)
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

    resolved_structure_candidates(chunk_origin, context)
        .iter()
        .filter(|candidate| {
            candidate.placement_id == structure_id || candidate.structure_id == structure_id
        })
        .map(|candidate| {
            if candidate.placement_id == structure_id
                && context.structure_sets.get(structure_id).is_some()
            {
                let surface_y = surface_height(
                    candidate.placement_anchor,
                    context.dimension,
                    context.biomes,
                    context.biome_field,
                );
                IVec3::new(
                    candidate.placement_anchor.x,
                    surface_y,
                    candidate.placement_anchor.y,
                )
            } else {
                IVec3::new(
                    candidate.anchor.x,
                    candidate.origin_y,
                    candidate.anchor.y,
                )
            }
        })
        .collect()
}

fn resolved_structure_candidates(
    chunk_origin: IVec3,
    context: &ChunkGenerationContext<'_>,
) -> std::sync::Arc<Vec<CachedStructureCandidate>> {
    let chunk_size = CHUNK_SIZE as i32;
    let horizontal_chunk = IVec2::new(
        chunk_origin.x.div_euclid(chunk_size),
        chunk_origin.z.div_euclid(chunk_size),
    );
    context.feature_fields.structure_candidates(horizontal_chunk, || {
        resolve_structure_candidates_uncached(chunk_origin, context)
            .into_iter()
            .map(|candidate| CachedStructureCandidate {
                placement_id: candidate.placement_id.to_owned(),
                structure_id: candidate.structure.id.clone(),
                rotation: candidate.rotation,
                placement_anchor: candidate.placement_anchor,
                anchor: candidate.anchor,
                origin_y: candidate.origin_y,
            })
            .collect()
    })
}

fn resolve_structure_candidates_uncached<'a>(
    chunk_origin: IVec3,
    context: &'a ChunkGenerationContext<'_>,
) -> Vec<StructureCandidate<'a>> {
    let chunk_size = CHUNK_SIZE as i32;
    let chunk_min = IVec2::new(chunk_origin.x, chunk_origin.z);
    let chunk_max = chunk_min + IVec2::splat(chunk_size - 1);
    let mut direct_candidates = Vec::new();

    for biome_structure in context.biomes.structure_placements() {
        collect_structure_candidates(
            chunk_min,
            chunk_max,
            context,
            StructurePlacementContext {
                biome_id: &biome_structure.biome_id,
                placement_id: &biome_structure.structure_id,
                placement: biome_structure.placement,
            },
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
                &mut overlapping,
            );
            for candidate in overlapping {
                if candidate.priority < direct.priority
                    || !candidates_may_conflict(&candidate, &direct)
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
    context: &ChunkGenerationContext<'_>,
) -> i32 {
    let chunk_size = CHUNK_SIZE as i32;
    let chunk_origin = IVec3::new(
        horizontal_chunk.x * chunk_size,
        0,
        horizontal_chunk.y * chunk_size,
    );

    resolved_structure_candidates(chunk_origin, context)
        .iter()
        .filter_map(|candidate| {
            let structure = context.structures.get(&candidate.structure_id)?;
            Some(candidate.origin_y + structure.max_y_offset())
        })
        .max()
        .unwrap_or(0)
}

fn collect_structure_candidates<'a>(
    target_min: IVec2,
    target_max: IVec2,
    context: &'a ChunkGenerationContext<'_>,
    placement_context: StructurePlacementContext<'a>,
    candidates: &mut Vec<StructureCandidate<'a>>,
) {
    if context
        .structure_sets
        .get(placement_context.placement_id)
        .is_some()
    {
        collect_structure_set_candidates(
            target_min,
            target_max,
            context,
            placement_context,
            candidates,
        );
    } else {
        collect_direct_structure_candidates(
            target_min,
            target_max,
            context,
            placement_context,
            candidates,
        );
    }
}

fn collect_direct_structure_candidates<'a>(
    target_min: IVec2,
    target_max: IVec2,
    context: &'a ChunkGenerationContext<'_>,
    placement_context: StructurePlacementContext<'a>,
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
            let rotation = structure.rotation_for_hash(member_hash);
            let (minimum_offset, maximum_offset) =
                structure.horizontal_bounds_for_rotation(rotation);
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
                rotation,
                anchor,
                || {
                    let origin_y =
                        compute_structure_origin_y(anchor, structure, rotation, context)?;
                    candidate_satisfies_restrictions(
                        biome_id,
                        structure,
                        rotation,
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
                rotation,
                placement_anchor: anchor,
                anchor,
                origin_y,
                priority: structure.priority,
                reserve_space: structure.generation.reserve_space,
                conflict_groups: &structure.conflict_groups,
                minimum,
                maximum,
            });
        },
    );
}

fn collect_structure_set_candidates<'a>(
    target_min: IVec2,
    target_max: IVec2,
    context: &'a ChunkGenerationContext<'_>,
    placement_context: StructurePlacementContext<'a>,
    candidates: &mut Vec<StructureCandidate<'a>>,
) {
    let biome_id = placement_context.biome_id;
    let placement_id = placement_context.placement_id;
    let set = context
        .structure_sets
        .get(placement_id)
        .expect("validated structure set placement must resolve");
    let bounds = set
        .horizontal_bounds(context.structures)
        .unwrap_or_else(|| panic!("structure set {placement_id} has no resolvable bounds"));

    visit_candidate_anchors_intersecting(
        target_min,
        target_max,
        context.biome_field.seed(),
        placement_context,
        bounds,
        |placement_anchor| {
            if context
                .biome_field
                .sample_surface(placement_anchor.as_vec2() + Vec2::splat(0.5))
                .primary_id
                != biome_id
            {
                return;
            }

            let Some(pieces) = resolve_set_pieces(
                context.biome_field.seed(),
                set,
                placement_anchor,
                context.structures,
                |structure, rotation, anchor| {
                    context.feature_fields.structure_origin_y(
                        &structure.id,
                        rotation,
                        anchor,
                        || {
                            let origin_y =
                                compute_structure_origin_y(anchor, structure, rotation, context)?;
                            candidate_satisfies_restrictions(
                                biome_id,
                                structure,
                                rotation,
                                anchor,
                                origin_y,
                                context,
                            )
                            .then_some(origin_y)
                        },
                    )
                },
            ) else {
                return;
            };

            let minimum = pieces
                .iter()
                .fold(IVec2::splat(i32::MAX), |minimum, piece| {
                    minimum.min(piece.minimum)
                });
            let maximum = pieces
                .iter()
                .fold(IVec2::splat(i32::MIN), |maximum, piece| {
                    maximum.max(piece.maximum)
                });
            if !rectangles_overlap(minimum, maximum, target_min, target_max) {
                return;
            }

            for piece in pieces {
                candidates.push(StructureCandidate {
                    biome_id,
                    placement_id,
                    structure: piece.structure,
                    rotation: piece.rotation,
                    placement_anchor,
                    anchor: piece.anchor,
                    origin_y: piece.origin_y,
                    priority: set.priority,
                    reserve_space: set.reserve_space,
                    conflict_groups: &set.conflict_groups,
                    minimum,
                    maximum,
                });
            }
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
) -> (
    &'a str,
    &'a str,
    IVec2,
    &'a str,
    IVec2,
    StructureRotation,
) {
    (
        candidate.biome_id,
        candidate.placement_id,
        candidate.placement_anchor,
        candidate.structure.id.as_str(),
        candidate.anchor,
        candidate.rotation,
    )
}

fn candidates_may_conflict(
    higher: &StructureCandidate<'_>,
    lower: &StructureCandidate<'_>,
) -> bool {
    higher.reserve_space
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
        .priority
        .cmp(&left.priority)
        .then_with(|| left.placement_id.cmp(right.placement_id))
        .then_with(|| left.biome_id.cmp(right.biome_id))
        .then_with(|| left.placement_anchor.x.cmp(&right.placement_anchor.x))
        .then_with(|| left.placement_anchor.y.cmp(&right.placement_anchor.y))
        .then_with(|| left.structure.id.cmp(&right.structure.id))
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
        && left.placement_anchor == right.placement_anchor
}

fn candidates_conflict(
    higher: &StructureCandidate<'_>,
    lower: &StructureCandidate<'_>,
) -> bool {
    rectangles_overlap(
        higher.minimum,
        higher.maximum,
        lower.minimum,
        lower.maximum,
    ) && candidates_may_conflict(higher, lower)
}

fn bit_get(bits: &[u64; STRUCTURE_OCCUPANCY_WORDS], index: usize) -> bool {
    bits[index / u64::BITS as usize] & (1_u64 << (index % u64::BITS as usize)) != 0
}

fn bit_set(bits: &mut [u64; STRUCTURE_OCCUPANCY_WORDS], index: usize) {
    bits[index / u64::BITS as usize] |= 1_u64 << (index % u64::BITS as usize);
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
    chunk: &mut VoxelChunk,
    claimed: &mut [u64; STRUCTURE_OCCUPANCY_WORDS],
    context: &StructureRasterizationContext<'_>,
    structure: &StructureDefinition,
    rotation: StructureRotation,
    origin: IVec3,
) {
    visit_structure_voxels_in_chunk(
        structure,
        rotation,
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
                    !bit_get(claimed, index)
                        && !bit_get(context.base_occupied, index)
                }
                StructureReplacePolicy::Terrain => !bit_get(claimed, index),
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
                    rotation.rotate_orientation(voxel.orientation),
                )),
            );
            for (face, layer) in surface_layer_placements(
                context.world_seed,
                structure,
                rotation,
                voxel,
                world_position,
            ) {
                let _ = chunk.add_layer(
                    local_x,
                    local_y,
                    local_z,
                    face,
                    layer,
                );
            }
            if structure.generation.fluid_policy == StructureFluidPolicy::Displace {
                chunk.set_fluid(local_x, local_y, local_z, None);
            }
            bit_set(claimed, index);
            false
        },
    );
}

pub(crate) fn surface_layer_placements(
    world_seed: u64,
    structure: &StructureDefinition,
    structure_rotation: StructureRotation,
    voxel: &StructureVoxel,
    world_position: IVec3,
) -> SmallVec<[(LayerFace, LayerCell); 6]> {
    // Structure generation is deterministic across chunk order. Use the
    // world seed plus world-space position and structure/layer/face identity
    // so the same seed and structure always choose the same layer patches.
    let mut placements = SmallVec::new();
    for surface in structure.surface_layers_for_voxel(voxel) {
        for &face in &surface.faces {
            let face = structure_rotation.rotate_face(face);
            let hash = surface_layer_hash(
                world_seed,
                structure.runtime_hash(),
                surface.runtime_hash(),
                world_position,
                face,
            );
            if unit_interval(hash) >= surface.chance {
                continue;
            }
            let rotation = TextureRotation::from_quarter_turn(((hash >> 32) & 3) as u8);
            placements.push((face, LayerCell::new(&surface.layer, rotation)));
        }
    }
    placements
}

fn surface_layer_hash(
    world_seed: u64,
    structure_hash: u64,
    layer_hash: u64,
    position: IVec3,
    face: LayerFace,
) -> u64 {
    let mut hash =
        world_seed ^ structure_hash.rotate_left(11) ^ layer_hash.rotate_left(37);
    hash ^= (position.x as i64 as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
    hash ^= (position.y as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    hash ^= (position.z as i64 as u64).wrapping_mul(0x1656_67b1_9e37_79f9);
    hash ^= (face.index() as u64 + 1).wrapping_mul(0xd6e8_feb8_6659_fd93);
    avalanche(hash)
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
    rotation: StructureRotation,
    origin: IVec3,
    chunk_origin: IVec3,
    mut visit: impl FnMut(&StructureVoxel, IVec3, IVec3) -> bool,
) -> bool {
    let chunk_size = CHUNK_SIZE as i32;
    if structure.voxels().len() < COLUMN_INDEX_MIN_VOXELS {
        for voxel in structure.voxels() {
            let world_position = origin + rotation.rotate_offset(voxel.offset);
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
            let rotated_offset = world_horizontal - origin_horizontal;
            let structure_offset =
                rotation.inverse().rotate_horizontal(rotated_offset);
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
