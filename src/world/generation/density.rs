use bevy::prelude::*;

use crate::{
    content::biome::BiomeRegistry,
    voxel::chunk::{CHUNK_SIZE, CHUNK_VOLUME},
    world::{
        biome_field::{BiomeField, VolumeBiomeRegion, VolumeBiomeSelection},
        cave_connectivity::CaveConnectivityRegion,
        density_sampling::{
            DensitySampleContext, sample_density_column_hydrology, sample_density_with_hydrology,
        },
        generation_region::GenerationRegion,
        hydrology::HydrologyWaterSample,
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

const SURFACE_CARVER_WATER_MARGIN: f32 = 4.0;
const SURFACE_CARVER_WATER_ROOF: f32 = 3.0;
const SURFACE_CARVER_WATER_FADE_DEPTH: f32 = 4.0;

pub(super) struct DensityField {
    pub(super) values: Vec<f32>,
    pub(super) volume: Vec<Option<VolumeBiomeSelection>>,
}

pub(super) struct DensityPassContext<'a> {
    pub(super) region: &'a GenerationRegion,
    pub(super) volume_region: &'a VolumeBiomeRegion,
    pub(super) anchored_caves: Option<&'a CaveConnectivityRegion>,
    pub(super) biome_field: &'a BiomeField,
    pub(super) biomes: &'a BiomeRegistry,
    pub(super) sea_level: f32,
}

pub(super) fn sample_density_field(
    chunk_origin: IVec3,
    columns: &[GenerationColumnSample],
    pass: &DensityPassContext<'_>,
) -> DensityField {
    let context = DensitySampleContext::new(pass.region, pass.anchored_caves, pass.biome_field);
    let mut field = DensityField {
        values: vec![0.0; CHUNK_VOLUME],
        volume: vec![None; CHUNK_VOLUME],
    };
    let chunk_minimum_y = chunk_origin.y as f32 + 0.5;
    let chunk_maximum_y = chunk_origin.y as f32 + CHUNK_SIZE as f32 - 0.5;
    let surface_carver_context = SurfaceCarverResolveContext {
        biomes: pass.biomes,
        biome_field: pass.biome_field,
        world_seed: pass.biome_field.seed(),
        sea_level: pass.sea_level,
        minimum_y: chunk_minimum_y,
        maximum_y: chunk_maximum_y,
    };
    let mut surface_carvers = SurfaceCarverColumn::default();

    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let column = &columns[column_index(local_x, local_z)];
            let horizontal = Vec2::new(
                chunk_origin.x as f32 + local_x as f32 + 0.5,
                chunk_origin.z as f32 + local_z as f32 + 0.5,
            );
            let column_hydrology = sample_density_column_hydrology(horizontal, pass.region);
            let hydrology_deltas = pass
                .region
                .hydrology
                .density_deltas_for_column::<CHUNK_SIZE>(horizontal, chunk_origin.y as f32 + 0.5);
            let nearby_water = pass
                .region
                .hydrology
                .water_near(horizontal, SURFACE_CARVER_WATER_MARGIN);

            // Resolve the tunnel continuously through hydrology instead of turning the
            // entire carver off at the first wet column. Water protection is applied per
            // voxel below, preserving a solid bed/roof while allowing deep tunnel volume
            // to continue under rivers, lakes and oceans.
            resolve_surface_carver_column(
                &mut surface_carvers,
                horizontal,
                &column.surface_influences,
                &surface_carver_context,
            );

            for (local_y, hydrology_delta) in hydrology_deltas.iter().copied().enumerate() {
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
                let sampled_density = sample_density_with_hydrology(
                    base_density,
                    sample_position,
                    volume,
                    column_hydrology,
                    hydrology_delta,
                    &context,
                );
                let carver_delta = surface_carver_density_delta(
                    sampled_density,
                    sample_position,
                    &surface_carvers,
                ) * surface_carver_water_factor(sample_position.y, nearby_water);

                field.values[index] = sampled_density + carver_delta;
                field.volume[index] = volume;
            }
        }
    }

    field
}

fn surface_carver_water_factor(
    sample_y: f32,
    water: Option<HydrologyWaterSample<'_>>,
) -> f32 {
    let Some(water) = water else {
        return 1.0;
    };

    let full_protection_y = water.bed_level - SURFACE_CARVER_WATER_ROOF;
    let deep_progress = ((full_protection_y - sample_y) / SURFACE_CARVER_WATER_FADE_DEPTH)
        .clamp(0.0, 1.0);
    let deep_factor = deep_progress * deep_progress * (3.0 - 2.0 * deep_progress);
    let water_strength = water.strength.clamp(0.0, 1.0);

    1.0 - water_strength * (1.0 - deep_factor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::hydrology::HydrologyWaterKind;

    fn water(strength: f32) -> HydrologyWaterSample<'static> {
        HydrologyWaterSample {
            fluid_id: "asteria:test/water",
            water_level: 64.0,
            bed_level: 58.0,
            strength,
            kind: HydrologyWaterKind::Lake,
        }
    }

    #[test]
    fn surface_carver_is_fully_protected_near_strong_water_bed() {
        assert_eq!(surface_carver_water_factor(57.0, Some(water(1.0))), 0.0);
    }

    #[test]
    fn deep_surface_carver_continues_below_water() {
        assert_eq!(surface_carver_water_factor(50.0, Some(water(1.0))), 1.0);
    }

    #[test]
    fn weak_water_edge_blends_carver_instead_of_binary_cutoff() {
        let factor = surface_carver_water_factor(57.0, Some(water(0.25)));
        assert!(factor > 0.0 && factor < 1.0);
    }
}
