use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry,
        biome_density::BiomeDensityModifier,
        block::BlockRegistry,
        dimension::DimensionDefinition,
        fluid::{FluidId, FluidRegistry},
    },
    voxel::{
        cell::VoxelCell,
        chunk::{VoxelChunk, CHUNK_SIZE},
        fluid::FluidCell,
        texture_rotation::TextureRotation,
    },
};

use super::{
    biome_field::{BiomeField, BiomeFieldSample},
    cave_connectivity::CaveConnectivityRegion,
    density_pipeline::sample_density,
    generation_pipeline::{GenerationStage, GENERATION_STAGE_ORDER},
    generation_region::{
        generation_region_coord, generation_region_world_bounds, GenerationRegion,
    },
    material_field::solid_block_id,
    terrain::{chunk_y_bounds, surface_height, surface_height_from_sample, terrain_density},
    world_feature_fields::WorldFeatureFields,
};

const GRASS_BLOCK_ID: &str = "mineclone:grass";
const VOXELS_PER_CHUNK: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

struct GenerationColumnSample<'a> {
    surface_height: i32,
    surface_fluid: Option<FluidId>,
    surface: BiomeFieldSample<'a>,
}

pub(crate) fn generate_chunk(
    coord: IVec3,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    feature_fields: &WorldFeatureFields,
) -> VoxelChunk {
    if coord.y < 0 {
        return VoxelChunk::empty();
    }

    let (_, maximum_surface_chunk_y) = chunk_y_bounds(dimension, biomes);

    if coord.y > maximum_surface_chunk_y && !biomes.has_volume_density_modifiers() {
        return VoxelChunk::empty();
    }

    assert!(
        blocks.get(GRASS_BLOCK_ID).is_some(),
        "missing block definition: {GRASS_BLOCK_ID}"
    );
    debug_assert_eq!(GENERATION_STAGE_ORDER[0], GenerationStage::SurfaceColumns);
    debug_assert_eq!(GENERATION_STAGE_ORDER[1], GenerationStage::Density);
    debug_assert_eq!(GENERATION_STAGE_ORDER[2], GenerationStage::Materials);
    debug_assert_eq!(GENERATION_STAGE_ORDER[3], GenerationStage::Fluids);
    debug_assert_eq!(GENERATION_STAGE_ORDER[4], GenerationStage::Features);

    let chunk_origin = coord * CHUNK_SIZE as i32;
    let region_coord = generation_region_coord(coord);
    let region = feature_fields.region_with_hydrology(region_coord, |hydrology| {
        hydrology.region_from_macro_terrain(
            IVec2::new(region_coord.x, region_coord.z),
            |position| {
                let surface_position = IVec2::new(
                    position.x.floor() as i32,
                    position.y.floor() as i32,
                );
                let elevation = surface_height(
                    surface_position,
                    dimension,
                    biomes,
                    biome_field,
                ) as f32;
                let continentalness = biome_field.climate_at(position).continentalness;

                (elevation, continentalness)
            },
        )
    });
    let anchored_caves = anchored_cave_region(
        region.as_ref(),
        biome_field,
        biomes,
        feature_fields,
    );
    let columns = sample_generation_columns(
        chunk_origin,
        fluids,
        dimension,
        biomes,
        biome_field,
    );
    let density = sample_density_field(
        chunk_origin,
        &columns,
        region.as_ref(),
        anchored_caves.as_ref(),
        biomes,
        biome_field,
    );
    let mut chunk = VoxelChunk::empty();

    rasterize_material_pass(
        &mut chunk,
        chunk_origin,
        &columns,
        &density,
        blocks,
        biomes,
        biome_field,
        region.as_ref(),
    );
    rasterize_fluid_pass(
        &mut chunk,
        chunk_origin,
        &columns,
        &density,
        dimension.sea_level,
        fluids,
        region.as_ref(),
    );
    rasterize_feature_pass(
        &mut chunk,
        chunk_origin,
        region.as_ref(),
        biome_field,
    );

    chunk
}

fn anchored_cave_region(
    region: &GenerationRegion,
    biome_field: &BiomeField,
    biomes: &BiomeRegistry,
    feature_fields: &WorldFeatureFields,
) -> Option<CaveConnectivityRegion> {
    let (minimum, maximum) = generation_region_world_bounds(region.coord);
    let anchors = biome_field
        .volume_anchors_in_bounds(minimum, maximum)
        .into_iter()
        .filter(|anchor| {
            let biome = biomes
                .get(anchor.id)
                .unwrap_or_else(|| panic!("missing biome definition: {}", anchor.id));
            matches!(biome.density_modifier, Some(BiomeDensityModifier::Cavern { .. }))
        })
        .map(|anchor| anchor.position)
        .collect::<Vec<_>>();

    (!anchors.is_empty()).then(|| {
        feature_fields
            .cave_connectivity()
            .region_with_anchors(&region.cave_connectivity, &anchors)
    })
}

fn sample_generation_columns<'a>(
    chunk_origin: IVec3,
    fluids: &FluidRegistry,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &'a BiomeField,
) -> Vec<GenerationColumnSample<'a>> {
    let mut columns = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE);

    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let world_x = chunk_origin.x + local_x as i32;
            let world_z = chunk_origin.z + local_z as i32;
            let position = IVec2::new(world_x, world_z);
            let surface = biome_field.sample_surface(position.as_vec2() + Vec2::splat(0.5));
            let surface_height = surface_height_from_sample(
                position,
                dimension,
                biomes,
                biome_field.seed(),
                &surface,
            );
            let surface_fluid = surface_fluid_from_sample(&surface, biomes, fluids);

            columns.push(GenerationColumnSample {
                surface_height,
                surface_fluid,
                surface,
            });
        }
    }

    columns
}

fn sample_density_field(
    chunk_origin: IVec3,
    columns: &[GenerationColumnSample<'_>],
    region: &GenerationRegion,
    anchored_caves: Option<&CaveConnectivityRegion>,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> Vec<f32> {
    let mut density = vec![0.0; VOXELS_PER_CHUNK];

    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let column = &columns[column_index(local_x, local_z)];

            for local_y in 0..CHUNK_SIZE {
                let world_position = IVec3::new(
                    chunk_origin.x + local_x as i32,
                    chunk_origin.y + local_y as i32,
                    chunk_origin.z + local_z as i32,
                );
                let sample_position = world_position.as_vec3() + Vec3::splat(0.5);
                let base_density = terrain_density(column.surface_height, world_position.y);
                let sample = sample_density(
                    base_density,
                    sample_position,
                    region,
                    anchored_caves,
                    biomes,
                    biome_field,
                );

                density[voxel_index(local_x, local_y, local_z)] = sample.final_density;
            }
        }
    }

    density
}

fn rasterize_material_pass(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    columns: &[GenerationColumnSample<'_>],
    density: &[f32],
    blocks: &BlockRegistry,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    region: &GenerationRegion,
) {
    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let column = &columns[column_index(local_x, local_z)];

            for local_y in 0..CHUNK_SIZE {
                if density[voxel_index(local_x, local_y, local_z)] <= 0.0 {
                    continue;
                }

                let world_position = IVec3::new(
                    chunk_origin.x + local_x as i32,
                    chunk_origin.y + local_y as i32,
                    chunk_origin.z + local_z as i32,
                );
                let sample_position = world_position.as_vec3() + Vec3::splat(0.5);
                let volume = biome_field.sample_volume(sample_position);
                let block_id = solid_block_id(
                    sample_position,
                    &column.surface,
                    volume.as_ref(),
                    &region.geology,
                    biomes,
                    GRASS_BLOCK_ID,
                );
                let block = blocks
                    .get(block_id)
                    .unwrap_or_else(|| panic!("missing block definition: {block_id}"));
                let rotation = texture_rotation_for(world_position, block.rotate_texture);

                chunk.set_block(
                    local_x,
                    local_y,
                    local_z,
                    Some(VoxelCell::new(block_id, rotation)),
                );
            }
        }
    }
}

fn rasterize_fluid_pass(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    columns: &[GenerationColumnSample<'_>],
    density: &[f32],
    sea_level: i32,
    fluids: &FluidRegistry,
    region: &GenerationRegion,
) {
    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let column = &columns[column_index(local_x, local_z)];
            let world_x = chunk_origin.x + local_x as i32;
            let world_z = chunk_origin.z + local_z as i32;
            let horizontal = Vec2::new(world_x as f32 + 0.5, world_z as f32 + 0.5);
            let hydrology = region.hydrology.water_at(horizontal);

            for local_y in 0..CHUNK_SIZE {
                if density[voxel_index(local_x, local_y, local_z)] > 0.0 {
                    continue;
                }

                let world_y = chunk_origin.y + local_y as i32;
                let fluid_id = if let Some(water) = hydrology {
                    if world_y as f32 >= water.water_level {
                        continue;
                    }

                    Some(fluids.id_of(water.fluid_id).unwrap_or_else(|| {
                        panic!("hydrology references missing fluid: {}", water.fluid_id)
                    }))
                } else {
                    if world_y >= sea_level {
                        continue;
                    }

                    column.surface_fluid
                };
                let Some(fluid_id) = fluid_id else {
                    continue;
                };

                chunk.set_fluid(
                    local_x,
                    local_y,
                    local_z,
                    Some(FluidCell::source(fluid_id)),
                );
            }
        }
    }
}

fn rasterize_feature_pass(
    _chunk: &mut VoxelChunk,
    _chunk_origin: IVec3,
    _region: &GenerationRegion,
    _biome_field: &BiomeField,
) {
    // Feature placement intentionally runs after geometry and fluids. Concrete
    // trees, crystals, roots, structures, and similar content are authored later.
}

fn surface_fluid_from_sample(
    sample: &BiomeFieldSample<'_>,
    biomes: &BiomeRegistry,
    fluids: &FluidRegistry,
) -> Option<FluidId> {
    sample
        .influences
        .iter()
        .filter_map(|influence| {
            let biome = biomes
                .get(influence.id)
                .unwrap_or_else(|| panic!("missing biome definition: {}", influence.id));
            let fluid_id = biome.surface_fluid.as_deref()?;
            let fluid = fluids.id_of(fluid_id).unwrap_or_else(|| {
                panic!(
                    "biome {} references missing surface fluid: {fluid_id}",
                    biome.id
                )
            });

            Some((fluid, influence.weight))
        })
        .max_by(|(_, left_weight), (_, right_weight)| left_weight.total_cmp(right_weight))
        .map(|(fluid_id, _)| fluid_id)
}

fn column_index(x: usize, z: usize) -> usize {
    x + z * CHUNK_SIZE
}

fn voxel_index(x: usize, y: usize, z: usize) -> usize {
    x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE
}

fn texture_rotation_for(position: IVec3, enabled: bool) -> TextureRotation {
    if !enabled {
        return TextureRotation::default();
    }

    let mut hash = position.x as u32;
    hash ^= (position.y as u32).wrapping_mul(0x9e37_79b9);
    hash = hash.rotate_left(13);
    hash ^= (position.z as u32).wrapping_mul(0x85eb_ca6b);
    hash ^= hash >> 16;

    TextureRotation::from_quarter_turn((hash & 3) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voxel_index_covers_the_chunk_without_collisions() {
        let mut seen = vec![false; VOXELS_PER_CHUNK];

        for z in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let index = voxel_index(x, y, z);
                    assert!(!seen[index]);
                    seen[index] = true;
                }
            }
        }

        assert!(seen.into_iter().all(|value| value));
    }
}
