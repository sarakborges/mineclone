use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry,
        block::BlockRegistry,
        dimension::DimensionDefinition,
    },
    voxel::{
        cell::VoxelCell,
        chunk::{CHUNK_SIZE, VoxelChunk},
        texture_rotation::TextureRotation,
    },
    world::{
        biome_field::BiomeField,
        generation_region::GenerationRegion,
        hydrology::HydrologyMaterialSet,
        material_field::{SurfaceMaterialColumn, resolve_surface_material_column, solid_block_id},
    },
};

use super::{
    columns::GenerationColumnSample,
    density::DensityField,
    index::{column_index, voxel_index},
};

pub(super) struct MaterialPassContext<'a> {
    pub blocks: &'a BlockRegistry,
    pub biomes: &'a BiomeRegistry,
    pub dimension: &'a DimensionDefinition,
    pub biome_field: &'a BiomeField,
    pub region: &'a GenerationRegion,
}

fn hydrology_materials<'a>(
    column: &GenerationColumnSample,
    context: &'a MaterialPassContext<'a>,
) -> HydrologyMaterialSet<'a> {
    let surface = column
        .surface_influences
        .iter()
        .copied()
        .max_by(|left, right| {
            left.1
                .total_cmp(&right.1)
                .then_with(|| left.0.cmp(&right.0))
        })
        .map(|(index, _)| {
            let biome_id = context.biome_field.surface_biome_id(index);
            context
                .biomes
                .get(biome_id)
                .unwrap_or_else(|| panic!("missing surface biome definition: {biome_id}"))
        });

    let coast = context
        .dimension
        .hydrology
        .coast_biome
        .as_deref()
        .and_then(|id| context.biomes.get(id));
    let ocean = context
        .dimension
        .hydrology
        .ocean_biome
        .as_deref()
        .and_then(|id| context.biomes.get(id));

    HydrologyMaterialSet {
        river_bed_block: surface.and_then(|biome| biome.hydrology.river_bed_block.as_deref()),
        lake_bed_block: surface.and_then(|biome| biome.hydrology.lake_bed_block.as_deref()),
        inland_shore_block: surface.and_then(|biome| biome.hydrology.shore_block.as_deref()),
        ocean_bed_block: ocean.and_then(|biome| biome.hydrology.ocean_bed_block.as_deref()),
        coast_shore_block: coast
            .and_then(|biome| biome.hydrology.shore_block.as_deref())
            .or_else(|| ocean.and_then(|biome| biome.hydrology.shore_block.as_deref())),
    }
}

pub(super) fn rasterize_material_pass(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    columns: &[GenerationColumnSample],
    density: &DensityField,
    context: &MaterialPassContext<'_>,
) {
    let mut surface_materials = SurfaceMaterialColumn::default();

    chunk.edit_content(|chunk| {
        for local_z in 0..CHUNK_SIZE {
            for local_x in 0..CHUNK_SIZE {
                let column = &columns[column_index(local_x, local_z)];
                let horizontal = Vec2::new(
                    chunk_origin.x as f32 + local_x as f32 + 0.5,
                    chunk_origin.z as f32 + local_z as f32 + 0.5,
                );
                let hydrology_materials = hydrology_materials(column, context);
                let hydrology_blocks = context
                    .region
                    .hydrology
                    .solid_blocks_for_column::<CHUNK_SIZE>(
                        horizontal,
                        chunk_origin.y as f32 + 0.5,
                        hydrology_materials,
                    );
                resolve_surface_material_column(
                    &column.surface_influences,
                    column.surface_margin_index,
                    context.biome_field,
                    context.biomes,
                    &mut surface_materials,
                );

                for (local_y, hydrology_block) in hydrology_blocks.iter().copied().enumerate() {
                    let index = voxel_index(local_x, local_y, local_z);
                    if density.values[index] <= 0.0 {
                        continue;
                    }

                    let world_position = IVec3::new(
                        chunk_origin.x + local_x as i32,
                        chunk_origin.y + local_y as i32,
                        chunk_origin.z + local_z as i32,
                    );
                    let sample_position = world_position.as_vec3() + Vec3::splat(0.5);
                    let surface_depth = (column.surface_height - world_position.y - 1).max(0) as u32;
                    let block_id = solid_block_id(
                        sample_position,
                        surface_depth,
                        density.volume[index],
                        hydrology_block,
                        &surface_materials,
                        context.biome_field,
                    );
                    let block = context
                        .blocks
                        .get(block_id)
                        .unwrap_or_else(|| panic!("missing block definition: {block_id}"));
                    let rotation =
                        TextureRotation::for_position(world_position, block.rotate_texture.any());

                    chunk.set_block(
                        local_x,
                        local_y,
                        local_z,
                        Some(VoxelCell::new(block_id, rotation)),
                    );
                }
            }
        }
    });
}
