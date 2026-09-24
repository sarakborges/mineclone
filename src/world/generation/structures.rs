mod connectors;
mod geometry;
mod hash;
mod placement;
mod restrictions;
mod set;
mod support;

use std::{cmp::Ordering, collections::HashSet};

use bevy::prelude::*;
use smallvec::SmallVec;

use crate::{
    content::{
        biome_density::BiomeDensityModifier,
        biome_structure::{BiomeStructurePlacementRules, StructurePlacementRules},
        block::BlockRegistry,
        fluid::FluidRegistry,
        layer::LayerFace,
        object::ObjectPlacementFace,
        structure::{StructureDefinition, StructureRotation, StructureVoxel},
        structure_rules::{StructureFluidPolicy, StructureReplacePolicy},
    },
    voxel::{
        cell::VoxelCell,
        chunk::{CHUNK_SIZE, CHUNK_VOLUME, VoxelChunk, VoxelChunkStructureMut},
        fluid::{FluidCell, MAX_FLUID_LEVEL},
        layer::LayerCell,
        object::ObjectCell,
        texture_rotation::TextureRotation,
    },
    world::{
        world_feature_fields::CachedStructureCandidate,
    },
};

pub(crate) use self::{
    connectors::{
        ResolvedConnectedPiece, resolve_connected_piece_forest, resolve_connected_pieces,
    },
    set::resolve_set_pieces,
};

use self::{geometry::rectangles_overlap, hash::{avalanche, unit_interval}};
use self::{
    placement::{
        candidate_anchor, structure_member_hash, volume_site_is_selected,
        volume_structure_member_hash,
    },
    restrictions::candidate_satisfies_restrictions,
    support::compute_structure_origin_y,
};
use super::{ChunkGenerationContext, generation_surface_height};

const COLUMN_INDEX_MIN_VOXELS: usize = 512;
const STRUCTURE_OCCUPANCY_WORDS: usize = CHUNK_VOLUME.div_ceil(u64::BITS as usize);

#[derive(Clone, Copy)]
struct StructureCandidate<'a> {
    biome_id: &'a str,
    placement_id: &'a str,
    structure: &'a StructureDefinition,
    rotation: StructureRotation,
    placement_anchor: IVec2,
    placement_y: i32,
    anchor: IVec2,
    origin_y: i32,
    primary_placement_piece: bool,
    priority: i32,
    reserve_space: bool,
    conflict_groups: &'a [String],
    minimum: IVec2,
    maximum: IVec2,
    minimum_y: i32,
    maximum_y: i32,
}

#[derive(Clone, Copy)]
struct StructurePlacementContext<'a> {
    biome_id: &'a str,
    placement_id: &'a str,
    placement: BiomeStructurePlacementRules,
}

struct StructureRasterizationContext<'a> {
    base_occupied: &'a [u64; STRUCTURE_OCCUPANCY_WORDS],
    blocks: &'a BlockRegistry,
    fluids: &'a FluidRegistry,
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
        let maximum_y = candidate.origin_y + structure.effective_max_y_offset();
        maximum_y >= chunk_min_y && minimum_y <= chunk_max_y
    });
    if !intersects_section {
        return;
    }

    let needs_base_occupied = candidates.iter().any(|candidate| {
        let structure = context
            .structures
            .get(&candidate.structure_id)
            .unwrap_or_else(|| panic!("missing cached structure: {}", candidate.structure_id));
        matches!(
            structure.generation.replace_policy,
            StructureReplacePolicy::AirOnly
        )
    });

    let mut base_occupied = [0_u64; STRUCTURE_OCCUPANCY_WORDS];
    if needs_base_occupied {
        for local_y in 0..CHUNK_SIZE {
            for local_z in 0..CHUNK_SIZE {
                for local_x in 0..CHUNK_SIZE {
                    let index =
                        local_x + local_z * CHUNK_SIZE + local_y * CHUNK_SIZE * CHUNK_SIZE;
                    if chunk
                        .cell_at(local_x as i32, local_y as i32, local_z as i32)
                        .is_some()
                        || chunk
                            .fluid_at(local_x as i32, local_y as i32, local_z as i32)
                            .is_some()
                    {
                        bit_set(&mut base_occupied, index);
                    }
                }
            }
        }
    }

    let mut claimed = [0_u64; STRUCTURE_OCCUPANCY_WORDS];
    let chunk_horizontal_minimum = IVec2::new(chunk_origin.x, chunk_origin.z);
    let chunk_horizontal_maximum =
        chunk_horizontal_minimum + IVec2::splat(CHUNK_SIZE as i32 - 1);

    chunk.edit_structure_content(|chunk| {
        for candidate in candidates.iter() {
            let structure = context
                .structures
                .get(&candidate.structure_id)
                .unwrap_or_else(|| panic!("missing cached structure: {}", candidate.structure_id));
            let (minimum_offset, maximum_offset) =
                structure.horizontal_bounds_for_rotation(candidate.rotation);
            if !rectangles_overlap(
                candidate.anchor + minimum_offset,
                candidate.anchor + maximum_offset,
                chunk_horizontal_minimum,
                chunk_horizontal_maximum,
            ) {
                continue;
            }
            let minimum_y = candidate.origin_y + structure.min_y_offset();
            let maximum_y = candidate.origin_y + structure.effective_max_y_offset();
            if maximum_y < chunk_min_y || minimum_y > chunk_max_y {
                continue;
            }

            rasterize_structure(
                chunk,
                &mut claimed,
                &StructureRasterizationContext {
                    base_occupied: &base_occupied,
                    blocks: context.blocks,
                    fluids: context.fluids,
                    chunk_origin,
                    world_seed: context.biome_field.seed(),
                },
                structure,
                candidate.rotation,
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

fn validated_structure_origin_y(
    biome_id: &str,
    structure: &StructureDefinition,
    rotation: StructureRotation,
    anchor: IVec2,
    context: &ChunkGenerationContext<'_>,
) -> Option<i32> {
    context.feature_fields.structure_origin_y(
        &structure.id,
        rotation,
        anchor,
        || {
            let origin_y = compute_structure_origin_y(anchor, structure, rotation, context)?;
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
}

pub(crate) fn structure_candidate_probe(
    biome_id: &str,
    placement_id: &str,
    target_reference: &str,
    placement_anchor: IVec2,
    context: &ChunkGenerationContext<'_>,
) -> Option<IVec2> {
    if let Some(set) = context.structure_sets.get(placement_id) {
        if context
            .biome_field
            .sample_surface(placement_anchor.as_vec2() + Vec2::splat(0.5))
            .primary_id
            != biome_id
        {
            return None;
        }

        let pieces = resolve_set_pieces(
            context.biome_field.seed(),
            set,
            placement_anchor,
            context.structures,
            |structure, rotation, anchor| {
                validated_structure_origin_y(biome_id, structure, rotation, anchor, context)
            },
        )?;

        let piece = if target_reference == placement_id {
            pieces.first()?
        } else {
            pieces.iter().find(|piece| {
                context
                    .structures
                    .reference_contains_structure(target_reference, &piece.structure.id)
            })?
        };
        let offset = piece
            .structure
            .horizontal_footprint_for_rotation(piece.rotation)
            .first()
            .copied()?;
        return Some(piece.anchor + offset);
    }

    let member_hash = structure_member_hash(
        context.biome_field.seed(),
        biome_id,
        placement_id,
        placement_anchor,
    );
    let structure = context
        .structures
        .select_for_reference(placement_id, member_hash)?;
    if placement_id != target_reference
        && !context
            .structures
            .reference_contains_structure(target_reference, &structure.id)
    {
        return None;
    }
    let rotation = structure.rotation_for_hash(member_hash);
    let offset = structure
        .horizontal_footprint_for_rotation(rotation)
        .first()
        .copied()?;
    Some(placement_anchor + offset)
}

pub(crate) fn located_structure_origins_in_chunk(
    horizontal_chunk: IVec2,
    structure_id: &str,
    placement_anchor: IVec2,
    placement_y: i32,
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
            candidate.placement_anchor == placement_anchor
                && candidate.placement_y == placement_y
                && ((candidate.placement_id == structure_id
                    && candidate.primary_placement_piece)
                    || candidate.structure_id == structure_id
                    || context
                        .structures
                        .reference_contains_structure(structure_id, &candidate.structure_id))
        })
        .map(|candidate| {
            if candidate.placement_id == structure_id
                && context.structure_sets.get(structure_id).is_some()
            {
                let surface_y = generation_surface_height(candidate.placement_anchor, context);
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
                placement_y: candidate.placement_y,
                anchor: candidate.anchor,
                origin_y: candidate.origin_y,
                primary_placement_piece: candidate.primary_placement_piece,
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
        .filter(|candidate| {
            let (minimum_offset, maximum_offset) =
                candidate.structure.horizontal_bounds_for_rotation(candidate.rotation);
            rectangles_overlap(
                candidate.anchor + minimum_offset,
                candidate.anchor + maximum_offset,
                chunk_min,
                chunk_max,
            )
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
    let chunk_minimum = IVec2::new(chunk_origin.x, chunk_origin.z);
    let chunk_maximum = chunk_minimum + IVec2::splat(chunk_size - 1);
    resolved_structure_candidates(chunk_origin, context)
        .iter()
        .filter_map(|candidate| {
            let structure = context.structures.get(&candidate.structure_id)?;
            let (minimum_offset, maximum_offset) =
                structure.horizontal_bounds_for_rotation(candidate.rotation);
            let minimum = candidate.anchor + minimum_offset;
            let maximum = candidate.anchor + maximum_offset;
            rectangles_overlap(minimum, maximum, chunk_minimum, chunk_maximum)
                .then_some(candidate.origin_y + structure.effective_max_y_offset())
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
    match placement_context.placement {
        BiomeStructurePlacementRules::Surface(_) => {
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
        BiomeStructurePlacementRules::Volume(placement) => {
            debug_assert!(
                context
                    .structure_sets
                    .get(placement_context.placement_id)
                    .is_none(),
                "validated volume placement cannot reference a Structure Set"
            );
            collect_volume_structure_candidates(
                target_min,
                target_max,
                context,
                placement_context.biome_id,
                placement_context.placement_id,
                placement,
                candidates,
            );
        }
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
        .feature_fields
        .structure_placement_bounds(placement_id, || {
            connectors::connected_horizontal_bounds_for_reference(
                context.structures,
                placement_id,
            )
        })
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

            let surface_sample = context
                .biome_field
                .sample_surface(anchor.as_vec2() + Vec2::splat(0.5));
            if surface_sample.primary_id != biome_id {
                return;
            }

            let Some(origin_y) =
                validated_structure_origin_y(biome_id, structure, rotation, anchor, context)
            else {
                return;
            };
            let root_origin = IVec3::new(anchor.x, origin_y, anchor.y);
            let pieces = connectors::resolve_connected_pieces(
                context.biome_field.seed(),
                structure,
                rotation,
                root_origin,
                context.structures,
            );
            let Some((minimum, maximum, minimum_y, maximum_y)) =
                connected_piece_bounds(&pieces)
            else {
                return;
            };
            if !rectangles_overlap(minimum, maximum, target_min, target_max) {
                return;
            }

            for (piece_index, piece) in pieces.into_iter().enumerate() {
                candidates.push(StructureCandidate {
                    biome_id,
                    placement_id,
                    structure: piece.structure,
                    rotation: piece.rotation,
                    placement_anchor: anchor,
                    placement_y: 0,
                    anchor: piece.origin.xz(),
                    origin_y: piece.origin.y,
                    primary_placement_piece: piece_index == 0,
                    priority: structure.priority,
                    reserve_space: structure.generation.reserve_space,
                    conflict_groups: &structure.conflict_groups,
                    minimum,
                    maximum,
                    minimum_y,
                    maximum_y,
                });
            }
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
    let bounds = context
        .feature_fields
        .structure_placement_bounds(placement_id, || {
            set.horizontal_bounds_with(|reference| {
                connectors::connected_horizontal_bounds_for_reference(
                    context.structures,
                    reference,
                )
            })
        })
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
                    validated_structure_origin_y(biome_id, structure, rotation, anchor, context)
                },
            ) else {
                return;
            };

            let connected = connectors::resolve_connected_piece_forest(
                context.biome_field.seed(),
                pieces.iter().map(|piece| connectors::ResolvedConnectedPiece {
                    structure: piece.structure,
                    rotation: piece.rotation,
                    origin: IVec3::new(piece.anchor.x, piece.origin_y, piece.anchor.y),
                }),
                context.structures,
            );
            let Some((minimum, maximum, minimum_y, maximum_y)) =
                connected_piece_bounds(&connected)
            else {
                return;
            };
            if !rectangles_overlap(minimum, maximum, target_min, target_max) {
                return;
            }

            for (piece_index, piece) in connected.into_iter().enumerate() {
                candidates.push(StructureCandidate {
                    biome_id,
                    placement_id,
                    structure: piece.structure,
                    rotation: piece.rotation,
                    placement_anchor,
                    placement_y: 0,
                    anchor: piece.origin.xz(),
                    origin_y: piece.origin.y,
                    primary_placement_piece: piece_index == 0,
                    priority: set.priority,
                    reserve_space: set.reserve_space,
                    conflict_groups: &set.conflict_groups,
                    minimum,
                    maximum,
                    minimum_y,
                    maximum_y,
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
    let BiomeStructurePlacementRules::Surface(placement) = placement else {
        unreachable!("surface candidate visitor requires surface placement rules");
    };
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

fn collect_volume_structure_candidates<'a>(
    target_min: IVec2,
    target_max: IVec2,
    context: &'a ChunkGenerationContext<'_>,
    biome_id: &'a str,
    placement_id: &'a str,
    placement: crate::content::biome_structure::VolumeStructurePlacementRules,
    candidates: &mut Vec<StructureCandidate<'a>>,
) {
    let biome = context
        .biomes
        .get(biome_id)
        .unwrap_or_else(|| panic!("missing volume biome definition: {biome_id}"));
    if matches!(biome.density_modifier, Some(BiomeDensityModifier::Cavern { .. }))
        && !context.world_generation.spawn_caves()
    {
        return;
    }
    let range = biome
        .vertical_range
        .unwrap_or_else(|| panic!("volume biome {biome_id} with structures must define verticalRange"));
    let bounds = context
        .feature_fields
        .structure_placement_bounds(placement_id, || {
            connectors::connected_horizontal_bounds_for_reference(
                context.structures,
                placement_id,
            )
        })
        .unwrap_or_else(|| {
            panic!(
                "volume biome {biome_id} references missing structure or structure group: {placement_id}"
            )
        });

    let candidate_min = target_min - bounds.1;
    let candidate_max = target_max - bounds.0;
    let region = context.biome_field.volume_region_in_bounds(
        Vec3::new(candidate_min.x as f32, range.min, candidate_min.y as f32),
        Vec3::new(
            candidate_max.x.saturating_add(1) as f32,
            range.max + 1.0,
            candidate_max.y.saturating_add(1) as f32,
        ),
    );

    for volume_anchor in context.biome_field.volume_anchors_in_region(&region) {
        if volume_anchor.id != biome_id {
            continue;
        }
        let anchor = volume_anchor.position.floor().as_ivec3();
        if anchor.x < candidate_min.x
            || anchor.x > candidate_max.x
            || anchor.z < candidate_min.y
            || anchor.z > candidate_max.y
            || (anchor.y as f32) < range.min
            || (anchor.y as f32) > range.max
        {
            continue;
        }

        let Some(selection) = context
            .biome_field
            .volume_selection_in_region(volume_anchor.position, &region)
        else {
            continue;
        };
        if context.biome_field.volume_biome_id(selection) != biome_id
            || !volume_site_is_selected(
                context.biome_field.seed(),
                biome_id,
                placement_id,
                placement,
                anchor,
            )
        {
            continue;
        }

        let member_hash = volume_structure_member_hash(
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
                    "volume biome {biome_id} references empty structure group: {placement_id}"
                )
            });
        let rotation = structure.rotation_for_hash(member_hash.rotate_left(23));
        if !volume_root_satisfies_restrictions(
            biome_id,
            structure,
            rotation,
            anchor,
            context,
            &region,
        ) {
            continue;
        }

        let pieces = connectors::resolve_connected_pieces(
            context.biome_field.seed(),
            structure,
            rotation,
            anchor,
            context.structures,
        );
        let Some((minimum, maximum, minimum_y, maximum_y)) =
            connected_piece_bounds(&pieces)
        else {
            continue;
        };
        if !rectangles_overlap(minimum, maximum, target_min, target_max) {
            continue;
        }

        for (piece_index, piece) in pieces.into_iter().enumerate() {
            candidates.push(StructureCandidate {
                biome_id,
                placement_id,
                structure: piece.structure,
                rotation: piece.rotation,
                placement_anchor: anchor.xz(),
                placement_y: anchor.y,
                anchor: piece.origin.xz(),
                origin_y: piece.origin.y,
                primary_placement_piece: piece_index == 0,
                priority: structure.priority,
                reserve_space: structure.generation.reserve_space,
                conflict_groups: &structure.conflict_groups,
                minimum,
                maximum,
                minimum_y,
                maximum_y,
            });
        }
    }
}

fn volume_root_satisfies_restrictions(
    biome_id: &str,
    structure: &StructureDefinition,
    rotation: StructureRotation,
    origin: IVec3,
    context: &ChunkGenerationContext<'_>,
    region: &crate::world::biome_field::VolumeBiomeRegion,
) -> bool {
    let restrictions = &structure.restrictions;
    let base_y = origin.y + structure.min_y_offset();
    if restrictions.min_y.is_some_and(|minimum| base_y < minimum)
        || restrictions.max_y.is_some_and(|maximum| base_y > maximum)
        || !restrictions.ground_blocks.is_empty()
        || restrictions.requires_dry_ground
        || !restrictions.proximity.is_empty()
    {
        return false;
    }

    if restrictions.required_biome_coverage <= 0.0 {
        return true;
    }

    let voxels = structure.voxels();
    let matching = voxels
        .iter()
        .filter(|voxel| {
            let position = origin + rotation.rotate_offset(voxel.offset);
            context
                .biome_field
                .volume_selection_in_region(position.as_vec3() + Vec3::splat(0.5), region)
                .is_some_and(|selection| context.biome_field.volume_biome_id(selection) == biome_id)
        })
        .count();
    let coverage = matching as f32 / voxels.len().max(1) as f32;
    coverage + f32::EPSILON >= restrictions.required_biome_coverage
}

fn connected_piece_bounds(
    pieces: &[connectors::ResolvedConnectedPiece<'_>],
) -> Option<(IVec2, IVec2, i32, i32)> {
    pieces.iter().fold(None, |bounds, piece| {
        let (minimum_offset, maximum_offset) =
            piece.structure.horizontal_bounds_for_rotation(piece.rotation);
        let minimum = piece.origin.xz() + minimum_offset;
        let maximum = piece.origin.xz() + maximum_offset;
        let minimum_y = piece.origin.y + piece.structure.min_y_offset();
        let maximum_y = piece.origin.y + piece.structure.effective_max_y_offset();
        Some(match bounds {
            Some((current_minimum, current_maximum, current_min_y, current_max_y)) => (
                current_minimum.min(minimum),
                current_maximum.max(maximum),
                current_min_y.min(minimum_y),
                current_max_y.max(maximum_y),
            ),
            None => (minimum, maximum, minimum_y, maximum_y),
        })
    })
}

fn candidate_identity<'a>(
    candidate: &StructureCandidate<'a>,
) -> (
    &'a str,
    &'a str,
    IVec2,
    i32,
    &'a str,
    IVec2,
    StructureRotation,
) {
    (
        candidate.biome_id,
        candidate.placement_id,
        candidate.placement_anchor,
        candidate.placement_y,
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
        .then_with(|| left.placement_y.cmp(&right.placement_y))
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
        && left.placement_y == right.placement_y
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
    ) && higher.maximum_y >= lower.minimum_y
        && higher.minimum_y <= lower.maximum_y
        && candidates_may_conflict(higher, lower)
}

fn bit_get(bits: &[u64; STRUCTURE_OCCUPANCY_WORDS], index: usize) -> bool {
    bits[index / u64::BITS as usize] & (1_u64 << (index % u64::BITS as usize)) != 0
}

fn bit_set(bits: &mut [u64; STRUCTURE_OCCUPANCY_WORDS], index: usize) {
    bits[index / u64::BITS as usize] |= 1_u64 << (index % u64::BITS as usize);
}

fn rasterize_structure(
    chunk: &mut VoxelChunkStructureMut<'_>,
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

            if let Some(object_id) = structure.object_for_voxel(voxel) {
                let max_rise = structure.restrictions.max_slope.max(0) as usize;
                let Some(support_y) = (local_y..=local_y.saturating_add(max_rise))
                    .rev()
                    .find(|&candidate_y| {
                        candidate_y < CHUNK_SIZE
                            && chunk.cell_at(local_x, candidate_y, local_z).is_some()
                    })
                else {
                    return false;
                };
                let object_y = support_y + 1;
                if object_y >= CHUNK_SIZE
                    || chunk.fluid_at(local_x, support_y, local_z).is_some()
                    || chunk.fluid_at(local_x, object_y, local_z).is_some()
                {
                    return false;
                }

                let support_world_position =
                    context.chunk_origin
                        + IVec3::new(local_x as i32, support_y as i32, local_z as i32);
                let object = ObjectCell::new(
                    object_id,
                    ObjectPlacementFace::Top,
                    TextureRotation::for_position(support_world_position, true),
                );
                let _ = chunk.set_object(local_x, support_y, local_z, object);
                return false;
            }

            if structure.layers_only_voxel(voxel) {
                let max_rise = structure.restrictions.max_slope.max(0) as usize;
                let Some(support_y) = (local_y..=local_y.saturating_add(max_rise))
                    .rev()
                    .find(|&candidate_y| {
                        candidate_y < CHUNK_SIZE
                            && chunk.cell_at(local_x, candidate_y, local_z).is_some()
                    })
                else {
                    return false;
                };
                let support_world_position =
                    context.chunk_origin
                        + IVec3::new(local_x as i32, support_y as i32, local_z as i32);
                for (face, layer) in surface_layer_placements(
                    context.world_seed,
                    structure,
                    rotation,
                    voxel,
                    support_world_position,
                ) {
                    let _ = chunk.add_layer(local_x, support_y, local_z, face, layer);
                }
                return false;
            }

            if let Some(block_id) = voxel.block_id {
                let block = context.blocks.get(block_id).unwrap_or_else(|| {
                    panic!(
                        "structure {} references missing block: {}",
                        structure.id, block_id
                    )
                });
                let texture_rotation =
                    TextureRotation::for_position(world_position, block.rotate_texture.any());

                chunk.set_block(
                    local_x,
                    local_y,
                    local_z,
                    VoxelCell::oriented(
                        block_id,
                        texture_rotation,
                        rotation.rotate_orientation(voxel.orientation),
                    ),
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
                    chunk.clear_fluid(local_x, local_y, local_z);
                }
            } else if let Some(fluid_reference) = structure.fluid_for_voxel(voxel) {
                let fluid_id = context.fluids.id_of(fluid_reference).unwrap_or_else(|| {
                    panic!(
                        "structure {} references missing fluid: {}",
                        structure.id, fluid_reference
                    )
                });
                chunk.clear_block(local_x, local_y, local_z);
                chunk.set_fluid(
                    local_x,
                    local_y,
                    local_z,
                    FluidCell::source(fluid_id, MAX_FLUID_LEVEL),
                );
            } else {
                debug_assert!(
                    structure.clears_voxel(voxel),
                    "validated structure voxel must reference block, fluid, object, or clear"
                );
                chunk.clear_block(local_x, local_y, local_z);
                chunk.clear_fluid(local_x, local_y, local_z);
            }
            bit_set(claimed, index);
            false
        },
    );

    let chunk_size = CHUNK_SIZE as i32;
    for world_position in structure.clear_above_positions(rotation, origin) {
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
        if bit_get(claimed, index) {
            continue;
        }

        chunk.clear_block(local_x, local_y, local_z);
        chunk.clear_fluid(local_x, local_y, local_z);
        bit_set(claimed, index);
    }
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
            let rotation_hash = surface_layer_hash(
                world_seed,
                structure.runtime_hash(),
                surface.runtime_rotation_hash(),
                world_position,
                face,
            );
            let rotation =
                TextureRotation::from_quarter_turn(((rotation_hash >> 32) & 3) as u8);
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
