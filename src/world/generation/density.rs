use bevy::prelude::*;

use crate::{
    voxel::chunk::CHUNK_SIZE,
    world::{
        biome_field::{BiomeField, VolumeBiomeRegion, VolumeBiomeSelection},
        cave_connectivity::CaveConnectivityRegion,
        density_pipeline::sample_density,
        generation_region::GenerationRegion,
        terrain::terrain_density,
    },
};

use super::{
    columns::GenerationColumnSample,
    index::{VOXELS_PER_CHUNK, column_index, voxel_index},
};

pub(super) struct DensityField {
    pub(super) values: Vec<f32>,
    pub(super) volume: Vec<Option<VolumeBiomeSelection>>,
}

pub(super) fn sample_density_field(
    chunk_origin: IVec3,
    columns: &[GenerationColumnSample<'_>],
    region: &GenerationRegion,
    volume_region: &VolumeBiomeRegion,
    anchored_caves: Option<&CaveConnectivityRegion>,
    biome_field: &BiomeField,
) -> DensityField {
    let mut field = DensityField {
        values: vec![0.0; VOXELS_PER_CHUNK],
        volume: vec![None; VOXELS_PER_CHUNK],
    };

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
                let volume = biome_field.volume_selection_in_region(sample_position, volume_region);
                let index = voxel_index(local_x, local_y, local_z);

                field.values[index] = sample_density(
                    base_density,
                    sample_position,
                    region,
                    anchored_caves,
                    volume,
                    biome_field,
                );
                field.volume[index] = volume;
            }
        }
    }

    field
}
