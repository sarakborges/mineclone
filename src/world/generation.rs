mod columns;
mod density;
mod fluids;
mod index;
mod materials;
mod structures;
mod surface_objects;

use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry, block::BlockRegistry, block_id::intern_block_id,
        dimension::DimensionDefinition, fluid::FluidRegistry, structure::StructureRegistry,
        structure_set::StructureSetRegistry,
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
        terrain::{chunk_y_bounds, surface_height},
        world_feature_fields::WorldFeatureFields,
    },
};

pub(crate) use self::{
    columns::{
        GenerationColumnSample, ocean_weight_from_surface, sample_flat_generation_columns,
        sample_generation_columns,
    },
    fluids::authored_surface_fluid_id_for_position,
};
pub(crate) use self::structures::{
    ResolvedConnectedPiece, fit_structure_to_ground, located_structure_origins_in_chunk,
    resolve_connected_piece_forest_with_ground_fit, resolve_connected_pieces_with_ground_fit,
    resolve_set_pieces,
    structure_candidate_anchor, structure_candidate_probe, surface_layer_placements,
    volume_structure_candidate_probe,
};
use self::{
    density::{DensityPassContext, sample_density_field},
    fluids::{FluidPassContext, rasterize_fluid_pass},
    materials::{MaterialPassContext, rasterize_material_pass},
    structures::rasterize_structures,
    surface_objects::rasterize_surface_objects,
};

const LOCAL_EMPTY_HEADROOM_CHUNKS: i32 = 2;

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

pub(crate) fn generate_chunk(
    chunk_coord: IVec3,
    context: &ChunkGenerationContext<'_>,
) -> VoxelChunk {
    if chunk_coord.y < 0 {
        return VoxelChunk::empty();
    }

    if context.world_generation.mode() == WorldGenerationMode::Void {
        return generate_void_chunk(chunk_coord, context);
    }

    let horizontal_chunk = chunk_coord.xz();
    let structure_top_chunk =
        maximum_structure_top_chunk_for_horizontal_chunk(horizontal_chunk, context);
    let maximum_surface_chunk_y = match context.world_generation.mode() {
        WorldGenerationMode::Normal => chunk_y_bounds(context.dimension, context.biomes).1,
        WorldGenerationMode::Flat => {
            (flat_surface_height(context.dimension) - 1).div_euclid(CHUNK_SIZE as i32)
        }
        WorldGenerationMode::Void => unreachable!(),
    };
    let allow_solid_volume = context.world_generation.mode() == WorldGenerationMode::Normal;
    if chunk_coord.y > maximum_surface_chunk_y.max(structure_top_chunk)
        && (!allow_solid_volume || !context.biomes.has_volume_density_modifiers())
    {
        return VoxelChunk::empty();
    }

    let chunk_origin = chunk_origin(chunk_coord);
    let columns = context
        .feature_fields
        .generation_columns(horizontal_chunk, || match context.world_generation.mode() {
            WorldGenerationMode::Normal => sample_generation_columns(
                horizontal_chunk,
                context.dimension,
                context.biomes,
                context.biome_field,
            ),
            WorldGenerationMode::Flat => sample_flat_generation_columns(
                horizontal_chunk,
                flat_surface_height(context.dimension),
                context.biome_field,
            ),
            WorldGenerationMode::Void => unreachable!(),
        });
    let local_surface_chunk = columns
        .iter()
        .map(|column| column.surface_height)
        .max()
        .unwrap_or(1)
        .max(context.dimension.sea_level)
        .div_euclid(CHUNK_SIZE as i32);

    // Most volume modifiers only carve existing terrain. Avoid constructing a
    // cave/volume region for chunks that are well above any local
    // surface, while retaining two full chunks of headroom for high lake water,
    // biome transitions and structures reaching in from neighboring columns.
    if chunk_coord.y
        > local_surface_chunk.max(structure_top_chunk) + LOCAL_EMPTY_HEADROOM_CHUNKS
        && (!allow_solid_volume || !context.biomes.has_volume_solid_density_modifiers())
    {
        return VoxelChunk::empty();
    }

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
    let chunk_volume_region = volume_region.restricted_to_bounds(chunk_minimum, chunk_maximum);
    let density = sample_density_field(
        chunk_origin,
        columns.as_ref(),
        &DensityPassContext {
            volume_region: &chunk_volume_region,
            biome_field: context.biome_field,
            allow_caverns: context.world_generation.spawn_caves(),
            allow_solid_volume,
        },
    );
    let mut chunk = VoxelChunk::empty();

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
    if context.world_generation.spawn_structures() {
        rasterize_structures(&mut chunk, chunk_origin, context);
    }
    rasterize_surface_objects(&mut chunk, chunk_origin, columns.as_ref(), context);

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
        WorldGenerationMode::Normal => surface_height(
            position,
            context.dimension,
            context.biomes,
            context.biome_field,
        ),
        WorldGenerationMode::Flat => flat_surface_height(context.dimension),
        WorldGenerationMode::Void => 1,
    }
}

fn generate_void_chunk(
    chunk_coord: IVec3,
    context: &ChunkGenerationContext<'_>,
) -> VoxelChunk {
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

    let top_y = context.feature_fields.structure_top_y(horizontal_chunk, || {
        self::structures::maximum_potential_structure_top_y_for_chunk(
            horizontal_chunk,
            context,
        )
    });
    top_y.div_euclid(CHUNK_SIZE as i32)
}
