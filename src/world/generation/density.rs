use bevy::prelude::*;

use crate::{
    content::biome::BiomeRegistry,
    voxel::chunk::CHUNK_SIZE,
    world::{
        biome_field::{BiomeField, VolumeBiomeRegion, VolumeBiomeSelection},
        cave_connectivity::CaveConnectivityRegion,
        density_pipeline::{
            DensitySampleContext, sample_density_column_hydrology, sample_density_with_hydrology,
        },
        generation_region::GenerationRegion,
        terrain::terrain_density,
    },
};

use super::{
    columns::GenerationColumnSample,
    index::{VOXELS_PER_CHUNK, column_index, voxel_index},
    surface_carvers::{resolve_surface_carver_column, surface_carver_density_delta},
};

const SURFACE_CARVER_WATER_CLEARANCE: f32 = 12.0;

pub(super) struct DensityField {
    pub(super) values: Vec<f32>,
    pub(super) volume: Vec<Option<VolumeBiomeSelection>>,
}

pub(super) fn sample_density_field(
    chunk_origin: IVec3,
    columns: &[GenerationColumnSample],
    region: &GenerationRegion,
    volume_region: &VolumeBiomeRegion,
    anchored_caves: Option<&CaveConnectivityRegion>,
    biome_field: &BiomeField,
    biomes: &BiomeRegistry,
    sea_level: f32,
) -> DensityField {
    let context = DensitySampleContext::new(region, anchored_caves, biome_field);
    let mut field = DensityField {
        values: vec![0.0; VOXELS_PER_CHUNK],
        volume: vec![None; VOXELS_PER_CHUNK],
    };

    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let column = &columns[column_index(local_x, local_z)];
            let horizontal = Vec2::new(
                chunk_origin.x as f32 + local_x as f32 + 0.5,
                chunk_origin.z as f32 + local_z as f32 + 0.5,
            );
            let column_hydrology = sample_density_column_hydrology(horizontal, region);
            let hydrology_deltas = region.hydrology.density_deltas_for_column::<CHUNK_SIZE>(
                horizontal,
                chunk_origin.y as f32 + 0.5,
            );
            let surface_carver_allowed = region
                .hydrology
                .water_near(horizontal, SURFACE_CARVER_WATER_CLEARANCE)
                .is_none();
            let surface_carvers = surface_carver_allowed.then(|| {
                resolve_surface_carver_column(
                    horizontal,
                    &column.surface_influences,
                    biomes,
                    biome_field,
                    biome_field.seed(),
                    sea_level,
                )
            });

            for local_y in 0..CHUNK_SIZE {
                let world_position = IVec3::new(
                    chunk_origin.x + local_x as i32,
                    chunk_origin.y + local_y as i32,
                    chunk_origin.z + local_z as i32,
                );
                let sample_position = world_position.as_vec3() + Vec3::splat(0.5);
                let base_density = terrain_density(column.surface_height, world_position.y);
                let volume = biome_field.volume_selection_in_region(sample_position, volume_region);
                let index = voxel_index(local_x, local_y, local_z);
                let sampled_density = sample_density_with_hydrology(
                    base_density,
                    sample_position,
                    volume,
                    column_hydrology,
                    hydrology_deltas[local_y],
                    &context,
                );
                let carver_delta = surface_carvers.as_ref().map_or(0.0, |carvers| {
                    surface_carver_density_delta(sampled_density, sample_position, carvers)
                });

                field.values[index] = sampled_density + carver_delta;
                field.volume[index] = volume;
            }
        }
    }

    field
}
