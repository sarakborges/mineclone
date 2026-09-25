use bevy::prelude::*;

use crate::{
    voxel::chunk::{CHUNK_SIZE, CHUNK_VOLUME},
    world::{
        biome_field::{BiomeField, VolumeBiomeRegion, VolumeBiomeSelection},
        density_sampling::{DensitySampleContext, sample_density},
        terrain::terrain_density,
    },
};

use super::{
    columns::GenerationColumnSample,
    index::{column_index, voxel_index},
};

#[derive(Clone, Copy)]
struct PackedVolumeBiomeSelection {
    biome_index: u16,
    strength: f32,
}

impl PackedVolumeBiomeSelection {
    const NONE_INDEX: u16 = u16::MAX;
    const NONE: Self = Self {
        biome_index: Self::NONE_INDEX,
        strength: 0.0,
    };

    fn from_selection(selection: Option<VolumeBiomeSelection>) -> Self {
        let Some(selection) = selection else {
            return Self::NONE;
        };
        Self {
            biome_index: u16::try_from(selection.biome_index)
                .expect("volume biome index must fit in u16"),
            strength: selection.strength,
        }
    }

    fn unpack(self) -> Option<VolumeBiomeSelection> {
        (self.biome_index != Self::NONE_INDEX).then_some(VolumeBiomeSelection {
            biome_index: usize::from(self.biome_index),
            strength: self.strength,
            // Material resolution only needs biome identity. Density shaping
            // consumes the full selection before it is packed into the field.
            local_position: Vec3::ZERO,
        })
    }
}

pub(super) struct DensityField {
    pub(super) values: Vec<f32>,
    volume: Vec<PackedVolumeBiomeSelection>,
}

impl DensityField {
    pub(super) fn volume_at(&self, index: usize) -> Option<VolumeBiomeSelection> {
        self.volume[index].unpack()
    }
}

pub(super) struct DensityPassContext<'a> {
    pub(super) volume_region: &'a VolumeBiomeRegion,
    pub(super) biome_field: &'a BiomeField,
    pub(super) allow_caverns: bool,
    pub(super) allow_solid_volume: bool,
}

pub(super) fn sample_density_field(
    chunk_origin: IVec3,
    columns: &[GenerationColumnSample],
    pass: &DensityPassContext<'_>,
) -> DensityField {
    let context = DensitySampleContext::new(pass.biome_field)
        .with_volume_rules(pass.allow_caverns, pass.allow_solid_volume);
    let mut field = DensityField {
        values: vec![0.0; CHUNK_VOLUME],
        volume: vec![PackedVolumeBiomeSelection::NONE; CHUNK_VOLUME],
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
                let volume = pass
                    .biome_field
                    .volume_selection_in_region(sample_position, pass.volume_region);
                let index = voxel_index(local_x, local_y, local_z);

                field.values[index] = sample_density(base_density, sample_position, volume, &context);
                field.volume[index] = PackedVolumeBiomeSelection::from_selection(volume);
            }
        }
    }

    field
}
