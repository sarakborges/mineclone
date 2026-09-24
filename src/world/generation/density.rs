use bevy::prelude::*;

use crate::{
    content::{biome::BiomeRegistry, dimension::DimensionDefinition},
    voxel::chunk::{CHUNK_SIZE, CHUNK_VOLUME},
    world::{
        biome_field::{BiomeField, VolumeBiomeRegion, VolumeBiomeSelection},
        cave_connectivity::CaveConnectivityRegion,
        density_sampling::{DensitySampleContext, sample_density},
        generation_region::GenerationRegion,
        terrain::terrain_density,
    },
};

use super::{
    columns::GenerationColumnSample,
    index::{column_index, voxel_index},
    surface_carvers::{
        SurfaceCarverColumn, SurfaceCarverResolveContext, resolve_surface_carver_column,
        surface_carver_density_delta,
    },
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
    pub(super) region: &'a GenerationRegion,
    pub(super) volume_region: &'a VolumeBiomeRegion,
    pub(super) anchored_caves: Option<&'a CaveConnectivityRegion>,
    pub(super) biome_field: &'a BiomeField,
    pub(super) biomes: &'a BiomeRegistry,
    pub(super) dimension: &'a DimensionDefinition,
    pub(super) allow_caverns: bool,
    pub(super) allow_solid_volume: bool,
}

pub(super) fn sample_density_field(
    chunk_origin: IVec3,
    columns: &[GenerationColumnSample],
    pass: &DensityPassContext<'_>,
) -> DensityField {
    let context = DensitySampleContext::new(pass.anchored_caves, pass.biome_field)
        .with_volume_rules(pass.allow_caverns, pass.allow_solid_volume);
    let mut field = DensityField {
        values: vec![0.0; CHUNK_VOLUME],
        volume: vec![PackedVolumeBiomeSelection::NONE; CHUNK_VOLUME],
    };
    let chunk_minimum_y = chunk_origin.y as f32 + 0.5;
    let chunk_maximum_y = chunk_origin.y as f32 + CHUNK_SIZE as f32 - 0.5;
    let surface_carver_context = SurfaceCarverResolveContext {
        dimension: pass.dimension,
        biomes: pass.biomes,
        biome_field: pass.biome_field,
        world_seed: pass.biome_field.seed(),
        minimum_y: chunk_minimum_y,
        maximum_y: chunk_maximum_y,
        surface_height_override: (!pass.allow_solid_volume)
            .then_some(columns.first().map_or(1.0, |column| column.surface_height as f32)),
        cave_graph: pass
            .anchored_caves
            .map(|caves| &caves.connector_graph),
    };
    let mut surface_carvers = SurfaceCarverColumn::default();
    let surface_carver_cache = pass.region.surface_carvers.as_ref();

    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let column = &columns[column_index(local_x, local_z)];
            let horizontal = Vec2::new(
                chunk_origin.x as f32 + local_x as f32 + 0.5,
                chunk_origin.z as f32 + local_z as f32 + 0.5,
            );

            if pass.allow_caverns {
                resolve_surface_carver_column(
                    &mut surface_carvers,
                    surface_carver_cache,
                    horizontal,
                    column.surface_height as f32,
                    column.identity_surface_index,
                    &column.surface_influences,
                    &surface_carver_context,
                );
            }

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
                let sampled_density =
                    sample_density(base_density, sample_position, volume, &context);
                let carver_delta = surface_carver_density_delta(
                    sampled_density,
                    sample_position,
                    column.surface_height as f32,
                    &surface_carvers,
                );

                field.values[index] = sampled_density + carver_delta;
                field.volume[index] = PackedVolumeBiomeSelection::from_selection(volume);
            }
        }
    }

    field
}
