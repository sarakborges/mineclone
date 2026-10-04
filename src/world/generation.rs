pub(super) mod biome_map;
mod columns;
mod density;
mod fluids;
mod index;
mod materials;
mod structures;
mod surface_objects;

use std::time::{Duration, Instant};

use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry, biome_density::BiomeDensityModifier, block::BlockRegistry,
        block_id::intern_block_id, dimension::DimensionDefinition, fluid::FluidRegistry,
        structure::StructureRegistry, structure_set::StructureSetRegistry,
    },
    voxel::{
        cell::VoxelCell,
        chunk::{CHUNK_SIZE, VoxelChunk},
        coordinates::chunk_origin,
        texture_rotation::TextureRotation,
    },
    world::{
        biome_field::BiomeField,
        generation_region::{generation_region_coord, generation_region_world_bounds},
        new_world::{WorldGenerationMode, WorldGenerationSettings},
        terrain::{chunk_y_bounds, surface_height_from_sample},
        world_feature_fields::WorldFeatureFields,
    },
};

pub(crate) use self::structures::{
    ResolvedConnectedPiece, fit_structure_to_ground, located_structure_origins_in_chunk,
    resolve_connected_piece_forest_with_ground_fit, resolve_connected_pieces_with_ground_fit,
    resolve_set_pieces, structure_candidate_anchor, structure_candidate_probe,
    surface_layer_placements, volume_structure_candidate_probe,
};
use self::{
    biome_map::BiomeMapTile,
    columns::{sample_flat_generation_columns_from_map, sample_generation_columns_from_map},
    density::{DensityPassContext, sample_density_field},
    fluids::{FluidPassContext, rasterize_fluid_pass},
    materials::{MaterialPassContext, rasterize_material_pass},
    structures::rasterize_structures,
    surface_objects::rasterize_surface_objects,
};
pub(crate) use self::{
    columns::{GenerationColumnSample, ocean_weight_from_surface},
    fluids::authored_surface_fluid_id_for_position,
};

const LOCAL_EMPTY_HEADROOM_CHUNKS: i32 = 2;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ChunkGenerationPassTimings {
    pub(crate) biome_map: Option<Duration>,
    pub(crate) structure_extent: Option<Duration>,
    pub(crate) terrain_columns: Option<Duration>,
    pub(crate) volume_biomes: Option<Duration>,
    pub(crate) density_field: Option<Duration>,
    pub(crate) materials: Option<Duration>,
    pub(crate) initial_fluids: Option<Duration>,
    pub(crate) structures: Option<Duration>,
    pub(crate) surface_objects: Option<Duration>,
}

fn measure_generation_pass<T>(
    slot: &mut Option<Duration>,
    operation: impl FnOnce() -> T,
) -> T {
    let started = Instant::now();
    let result = operation();
    *slot = Some(started.elapsed());
    result
}

pub(crate) struct ChunkGenerationContext<'a> {
    pub(crate) blocks: &'a BlockRegistry,
    pub(crate) fluids: &'a FluidRegistry,
    pub(crate) dimension: &'a DimensionDefinition,
    pub(crate) biomes: &'a BiomeRegistry,
    pub(crate) structures: &'a StructureRegistry,
    pub(crate) structure_sets: &'a StructureSetRegistry,
    pub(crate) world_generation: WorldGenerationSettings,
    pub(crate) biome_field: &'a BiomeField,
    pub(crate) feature_fields: &'a WorldFeatureFields,
}

pub(crate) fn generate_chunk_profiled(
    chunk_coord: IVec3,
    context: &ChunkGenerationContext<'_>,
) -> (VoxelChunk, ChunkGenerationPassTimings) {
    let mut timings = ChunkGenerationPassTimings::default();
    let chunk = generate_chunk_inner(chunk_coord, context, &mut timings);
    (chunk, timings)
}

fn generate_chunk_inner(
    chunk_coord: IVec3,
    context: &ChunkGenerationContext<'_>,
    timings: &mut ChunkGenerationPassTimings,
) -> VoxelChunk {
    if chunk_coord.y < 0 {
        return VoxelChunk::empty();
    }

    if context.world_generation.mode() == WorldGenerationMode::Void {
        return generate_void_chunk(chunk_coord, context);
    }

    // Stage 1: resolve the horizontal biome map once. Every vertical chunk at
    // this X/Z reuses the same immutable map and one-block neighbor halo.
    let horizontal_chunk = chunk_coord.xz();
    let biome_map = measure_generation_pass(&mut timings.biome_map, || {
        context
            .feature_fields
            .surface_biome_map(horizontal_chunk, || {
                BiomeMapTile::sample(horizontal_chunk, context.biome_field)
            })
    });

    // Structure reach is resolved before terrain columns so sections above the
    // terrain surface can still be retained when authored structures reach into
    // them. Keep this separate from the later structure raster pass because it
    // may resolve/cache substantial connected-structure metadata on its own.
    let structure_top_chunk = measure_generation_pass(&mut timings.structure_extent, || {
        maximum_structure_top_chunk_for_horizontal_chunk(horizontal_chunk, context)
    });
    let maximum_surface_chunk_y = match context.world_generation.mode() {
        WorldGenerationMode::Normal => chunk_y_bounds(context.dimension, context.biomes).1,
        WorldGenerationMode::Flat => {
            (flat_surface_height(context.dimension) - 1).div_euclid(CHUNK_SIZE as i32)
        }
        WorldGenerationMode::Void => unreachable!(),
    };
    let allow_solid_volume = context.world_generation.mode() == WorldGenerationMode::Normal;
    let has_solid_volume = context.biomes.has_volume_solid_density_modifiers()
        || context.biomes.iter().any(|biome| {
            matches!(
                biome.density_modifier,
                Some(BiomeDensityModifier::FloatingIsland { .. })
            )
        });
    if chunk_coord.y > maximum_surface_chunk_y.max(structure_top_chunk)
        && (!allow_solid_volume || !context.biomes.has_volume_density_modifiers())
    {
        return VoxelChunk::empty();
    }

    // Stage 2: convert biome-map samples into terrain columns. This is still a
    // horizontal artifact and is reused by every vertical chunk section.
    let chunk_origin = chunk_origin(chunk_coord);
    let (columns, local_surface_chunk) =
        measure_generation_pass(&mut timings.terrain_columns, || {
            let columns = context.feature_fields.generation_columns(horizontal_chunk, || {
                match context.world_generation.mode() {
                    WorldGenerationMode::Normal => sample_generation_columns_from_map(
                        horizontal_chunk,
                        context.dimension,
                        context.biomes,
                        context.biome_field,
                        biome_map.as_ref(),
                    ),
                    WorldGenerationMode::Flat => sample_flat_generation_columns_from_map(
                        flat_surface_height(context.dimension),
                        biome_map.as_ref(),
                    ),
                    WorldGenerationMode::Void => unreachable!(),
                }
            });
            let local_surface_chunk = columns
                .iter()
                .map(|column| column.surface_height)
                .max()
                .unwrap_or(1)
                .max(context.dimension.sea_level)
                .div_euclid(CHUNK_SIZE as i32);
            (columns, local_surface_chunk)
        });

    // Most volume modifiers only carve existing terrain. Avoid constructing a
    // cave/volume region for chunks that are well above any local surface,
    // while retaining two full chunks of headroom for high lake water, biome
    // transitions and structures reaching in from neighboring columns. Solid
    // volume modifiers, including floating islands, must remain eligible here.
    if chunk_coord.y > local_surface_chunk.max(structure_top_chunk) + LOCAL_EMPTY_HEADROOM_CHUNKS
        && (!allow_solid_volume || !has_solid_volume)
    {
        return VoxelChunk::empty();
    }

    // Stage 3: resolve 3D/volume biomes for this generation region.
    let chunk_volume_region = measure_generation_pass(&mut timings.volume_biomes, || {
        let region_coord = generation_region_coord(chunk_coord);
        let volume_region = context
            .feature_fields
            .volume_biome_region(region_coord, || {
                let (minimum, maximum) = generation_region_world_bounds(region_coord);
                context
                    .biome_field
                    .volume_region_in_bounds(minimum, maximum)
            });
        let chunk_minimum = chunk_origin.as_vec3();
        let chunk_maximum = chunk_minimum + Vec3::splat(CHUNK_SIZE as f32);
        volume_region.restricted_to_bounds(chunk_minimum, chunk_maximum)
    });

    // Stage 4: turn terrain columns + volume biomes into a density field.
    let density = measure_generation_pass(&mut timings.density_field, || {
        sample_density_field(
            chunk_origin,
            columns.as_ref(),
            &DensityPassContext {
                volume_region: &chunk_volume_region,
                biome_field: context.biome_field,
                allow_caverns: context.world_generation.spawn_caves(),
                allow_solid_volume,
            },
        )
    });
    let mut chunk = VoxelChunk::empty();

    // Stages 5-8: authoritative voxel content. Rendering consumes the result
    // later and is intentionally absent from this pipeline.
    measure_generation_pass(&mut timings.materials, || {
        rasterize_material_pass(
            &mut chunk,
            chunk_origin,
            columns.as_ref(),
            &density,
            &MaterialPassContext {
                blocks: context.blocks,
                biomes: context.biomes,
                biome_field: context.biome_field,
            },
        );
    });
    measure_generation_pass(&mut timings.initial_fluids, || {
        rasterize_fluid_pass(
            &mut chunk,
            chunk_origin,
            columns.as_ref(),
            &density.values,
            &FluidPassContext {
                fluids: context.fluids,
                biomes: context.biomes,
                biome_field: context.biome_field,
                sea_level: context.dimension.sea_level,
                sea_fluid: &context.dimension.sea_fluid,
            },
        );
    });
    if context.world_generation.spawn_structures() {
        measure_generation_pass(&mut timings.structures, || {
            rasterize_structures(&mut chunk, chunk_origin, context);
        });
    }
    measure_generation_pass(&mut timings.surface_objects, || {
        rasterize_surface_objects(&mut chunk, chunk_origin, columns.as_ref(), context);
    });

    chunk
}

pub(crate) fn flat_surface_height(dimension: &DimensionDefinition) -> i32 {
    dimension.sea_level.max(1)
}

pub(crate) fn generation_surface_height(
    position: IVec2,
    context: &ChunkGenerationContext<'_>,
) -> i32 {
    match context.world_generation.mode() {
        WorldGenerationMode::Normal => {
            let chunk_size = CHUNK_SIZE as i32;
            let horizontal_chunk = IVec2::new(
                position.x.div_euclid(chunk_size),
                position.y.div_euclid(chunk_size),
            );
            let chunk_origin = horizontal_chunk * chunk_size;
            let local = position - chunk_origin;
            let biome_map = context
                .feature_fields
                .surface_biome_map(horizontal_chunk, || {
                    BiomeMapTile::sample(horizontal_chunk, context.biome_field)
                });
            let surface = biome_map
                .sample_at(local)
                .as_field_sample(context.biome_field);
            surface_height_from_sample(position, context.dimension, context.biome_field, &surface)
        }
        WorldGenerationMode::Flat => flat_surface_height(context.dimension),
        WorldGenerationMode::Void => 1,
    }
}

fn generate_void_chunk(chunk_coord: IVec3, context: &ChunkGenerationContext<'_>) -> VoxelChunk {
    if chunk_coord != IVec3::ZERO {
        return VoxelChunk::empty();
    }

    let block_id = intern_block_id("asteria:stone");
    let block = context
        .blocks
        .get(block_id)
        .unwrap_or_else(|| panic!("missing void spawn block definition: {block_id}"));
    let mut chunk = VoxelChunk::empty();
    let rotation = TextureRotation::for_position(IVec3::new(8, 0, 8), block.rotate_texture.any());
    chunk.edit_initial_blocks(|blocks| {
        blocks.set_block(8, 0, 8, VoxelCell::new(block_id, rotation));
    });
    chunk
}

pub(crate) fn maximum_structure_top_chunk_for_horizontal_chunk(
    horizontal_chunk: IVec2,
    context: &ChunkGenerationContext<'_>,
) -> i32 {
    if !context.world_generation.spawn_structures()
        || context.world_generation.mode() == WorldGenerationMode::Void
    {
        return -1;
    }

    let top_y = context
        .feature_fields
        .structure_top_y(horizontal_chunk, || {
            self::structures::maximum_potential_structure_top_y_for_chunk(horizontal_chunk, context)
        });
    top_y.div_euclid(CHUNK_SIZE as i32)
}
