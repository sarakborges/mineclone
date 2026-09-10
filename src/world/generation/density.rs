use bevy::prelude::*;

use crate::{
    content::biome::BiomeRegistry,
    voxel::chunk::CHUNK_SIZE,
    world::{
        biome_field::BiomeField, cave_connectivity::CaveConnectivityRegion,
        density_pipeline::sample_density, generation_region::GenerationRegion,
        terrain::terrain_density,
    },
};

use super::{
    columns::GenerationColumnSample,
    index::{VOXELS_PER_CHUNK, column_index, voxel_index},
};

pub(super) fn sample_density_field(
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
