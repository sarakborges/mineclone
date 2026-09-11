use std::sync::Arc;

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
    world::{
        cave_connectivity::CaveConnectivityRegion,
        density_pipeline::{DensitySampleContext, sample_density},
        generation_region::{GenerationRegion, generation_region_coord},
        terrain::{surface_height, terrain_density},
    },
};

use super::{
    ChunkGenerationContext,
    surface_carvers::{resolve_surface_carver_column, surface_carver_density_delta},
};

const MAX_STRUCTURE_GROUND_VARIATION: i32 = 1;
const MAX_STRUCTURE_GROUND_RISE: i32 = 3;
const SURFACE_CARVER_WATER_CLEARANCE: f32 = 12.0;

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

            let Some(origin_y) = structure_origin_y(anchor, &voxels, context) else {
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

fn structure_origin_y(
    anchor: IVec2,
    voxels: &[StructureVoxel<'_>],
    context: &ChunkGenerationContext<'_>,
) -> Option<i32> {
    let (region, anchored_caves) = structure_support_context(anchor, context);
    let minimum_offset_y = voxels.iter().map(|voxel| voxel.offset.y).min()?;
    let mut minimum_ground_y = i32::MAX;
    let mut maximum_ground_y = i32::MIN;

    for voxel in voxels
        .iter()
        .filter(|voxel| voxel.offset.y == minimum_offset_y)
    {
        let horizontal_offset = IVec2::new(voxel.offset.x, voxel.offset.z);
        let position = anchor + horizontal_offset;
        let sample_position = position.as_vec2() + Vec2::splat(0.5);

        if region.hydrology.water_at(sample_position).is_some() {
            return None;
        }

        let ground_y = supported_surface_ground_y(
            position,
            region.as_ref(),
            anchored_caves.as_deref(),
            context,
        )?;
        minimum_ground_y = minimum_ground_y.min(ground_y);
        maximum_ground_y = maximum_ground_y.max(ground_y);
    }

    if minimum_ground_y == i32::MAX
        || maximum_ground_y - minimum_ground_y > MAX_STRUCTURE_GROUND_VARIATION
    {
        return None;
    }

    Some(minimum_ground_y - minimum_offset_y)
}

fn structure_support_context(
    anchor: IVec2,
    context: &ChunkGenerationContext<'_>,
) -> (Arc<GenerationRegion>, Option<Arc<CaveConnectivityRegion>>) {
    let surface_y = surface_height(
        anchor,
        context.dimension,
        context.biomes,
        context.biome_field,
    );
    let chunk_size = CHUNK_SIZE as i32;
    let anchor_chunk = IVec3::new(
        anchor.x.div_euclid(chunk_size),
        (surface_y - 1).div_euclid(chunk_size).max(0),
        anchor.y.div_euclid(chunk_size),
    );
    let region = context.region(generation_region_coord(anchor_chunk));
    let anchored_caves = context.anchored_caves(region.as_ref());

    (region, anchored_caves)
}

fn supported_surface_ground_y(
    position: IVec2,
    region: &GenerationRegion,
    anchored_caves: Option<&CaveConnectivityRegion>,
    context: &ChunkGenerationContext<'_>,
) -> Option<i32> {
    let horizontal = position.as_vec2() + Vec2::splat(0.5);
    let surface = context.biome_field.sample_surface(horizontal);
    let raw_surface_height = surface_height(
        position,
        context.dimension,
        context.biomes,
        context.biome_field,
    );
    let raw_ground_y = raw_surface_height - 1;
    let influences = surface
        .influences
        .iter()
        .map(|influence| {
            (
                context.biome_field.surface_biome_index(influence.id),
                influence.weight,
            )
        })
        .collect::<Vec<_>>();
    let surface_carvers = region
        .hydrology
        .water_near(horizontal, SURFACE_CARVER_WATER_CLEARANCE)
        .is_none()
        .then(|| {
            resolve_surface_carver_column(
                horizontal,
                &influences,
                context.biomes,
                context.biome_field,
                context.biome_field.seed(),
                context.dimension.sea_level as f32,
            )
        });
    let density_context = DensitySampleContext::new(region, anchored_caves, context.biome_field);

    let density_at = |world_y: i32| {
        let sample_position = Vec3::new(horizontal.x, world_y as f32 + 0.5, horizontal.y);
        let base_density = terrain_density(raw_surface_height, world_y);
        let sampled_density = sample_density(base_density, sample_position, None, &density_context);
        let carver_delta = surface_carvers.as_ref().map_or(0.0, |carvers| {
            surface_carver_density_delta(sampled_density, sample_position, carvers)
        });

        sampled_density + carver_delta
    };

    if density_at(raw_ground_y) <= 0.0 {
        return None;
    }

    let mut ground_y = raw_ground_y;
    for candidate_y in (raw_ground_y + 1)..=(raw_ground_y + MAX_STRUCTURE_GROUND_RISE) {
        if density_at(candidate_y) <= 0.0 {
            break;
        }
        ground_y = candidate_y;
    }

    Some(ground_y)
}

fn candidate_anchor(
    world_seed: u64,
    biome_id: &str,
    structure: &StructureDefinition,
    placement: StructurePlacementRules,
    cell: IVec2,
) -> Option<IVec2> {
    let hash = placement_hash(world_seed, biome_id, &structure.id, cell);
    let chance = unit_interval(hash);
    if chance >= placement.chance {
        return None;
    }

    let spacing = placement.spacing;
    let center = cell * spacing + IVec2::splat(spacing / 2);
    let jitter = placement.jitter;
    let jitter_x = signed_jitter(hash ^ 0x517c_c1b7_2722_0a95, jitter);
    let jitter_z = signed_jitter(hash ^ 0x6eed_0e9d_a4d9_4a4f, jitter);

    Some(center + IVec2::new(jitter_x, jitter_z))
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

fn placement_hash(world_seed: u64, biome_id: &str, structure_id: &str, cell: IVec2) -> u64 {
    let mut hash = world_seed ^ string_hash(structure_id);
    hash ^= string_hash(biome_id).rotate_left(29);
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
