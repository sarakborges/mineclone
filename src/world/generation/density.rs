use bevy::prelude::*;

use crate::{
    content::{biome::BiomeRegistry, dimension::DimensionDefinition},
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
const SURFACE_CARVER_WATER_FADE_HEIGHT: f32 = 4.0;

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
    pub(super) dimension: &'a DimensionDefinition,
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
        dimension: pass.dimension,
        biomes: pass.biomes,
        biome_field: pass.biome_field,
        world_seed: pass.biome_field.seed(),
        minimum_y: chunk_minimum_y,
        maximum_y: chunk_maximum_y,
        cave_graph: pass
            .anchored_caves
            .map(|caves| &caves.connector_graph),
    };
    let mut surface_carvers = SurfaceCarverColumn::default();

    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let column = &columns[column_index(local_x, local_z)];
            let horizontal = Vec2::new(
                chunk_origin.x as f32 + local_x as f32 + 0.5,
                chunk_origin.z as f32 + local_z as f32 + 0.5,
            );
            // The density and the subsequent fluid pass must select the same
            // physically supported water candidate for this exact column.
            let column_hydrology = sample_density_column_hydrology(
                horizontal,
                pass.region,
                Some(column.surface_height as f32),
            );
            let hydrology_deltas = pass.region.hydrology.density_deltas_for_column::<CHUNK_SIZE>(
                horizontal,
                chunk_origin.y as f32 + 0.5,
                column.surface_height as f32,
            );

            // Resolve carvers first. Water protection is only needed if a voxel
            // is actually carved: a water_near scan on every column duplicated
            // expensive lake/river/ocean sampling even in carver-free terrain.
            resolve_surface_carver_column(
                &mut surface_carvers,
                horizontal,
                column.surface_height as f32,
                &column.surface_influences,
                &surface_carver_context,
            );
            let mut nearby_water = None;

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
                    column.surface_height as f32,
                    &surface_carvers,
                );
                let protected_carver_delta = apply_carver_water_protection(
                    carver_delta,
                    sample_position.y,
                    &mut nearby_water,
                    || {
                        pass.region
                            .hydrology
                            .water_near(horizontal, SURFACE_CARVER_WATER_MARGIN)
                    },
                );

                field.values[index] = sampled_density + protected_carver_delta;
                field.volume[index] = volume;
            }
        }
    }

    field
}

fn apply_carver_water_protection<'a>(
    carver_delta: f32,
    sample_y: f32,
    nearby_water: &mut Option<Option<HydrologyWaterSample<'a>>>,
    load_water: impl FnOnce() -> Option<HydrologyWaterSample<'a>>,
) -> f32 {
    if carver_delta == 0.0 {
        return 0.0;
    }

    let water = *nearby_water.get_or_insert_with(load_water);
    carver_delta * surface_carver_water_factor(sample_y, water)
}

fn surface_carver_water_factor(
    sample_y: f32,
    water: Option<HydrologyWaterSample<'_>>,
) -> f32 {
    let Some(water) = water else {
        return 1.0;
    };

    // Protect the wet bed and a small roof above the waterline, not the entire
    // column above a nearby river or lake. Both ends fade back into the tunnel
    // so protection does not create a hard vertical wall at either boundary.
    let lower_protection_y = water.bed_level - SURFACE_CARVER_WATER_ROOF;
    let deep_progress = ((lower_protection_y - sample_y) / SURFACE_CARVER_WATER_FADE_DEPTH)
        .clamp(0.0, 1.0);
    let deep_factor = deep_progress * deep_progress * (3.0 - 2.0 * deep_progress);
    let upper_protection_y = water.water_level + SURFACE_CARVER_WATER_ROOF;
    let high_progress = ((sample_y - upper_protection_y) / SURFACE_CARVER_WATER_FADE_HEIGHT)
        .clamp(0.0, 1.0);
    let high_factor = high_progress * high_progress * (3.0 - 2.0 * high_progress);
    let water_strength = water.strength.clamp(0.0, 1.0);

    1.0 - water_strength * (1.0 - deep_factor.max(high_factor))
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
        assert_eq!(surface_carver_water_factor(64.0, Some(water(1.0))), 0.0);
        assert_eq!(surface_carver_water_factor(67.0, Some(water(1.0))), 0.0);
    }

    #[test]
    fn deep_surface_carver_continues_below_water() {
        assert_eq!(surface_carver_water_factor(50.0, Some(water(1.0))), 1.0);
    }

    #[test]
    fn surface_carver_resumes_above_bounded_water_roof() {
        let fade = surface_carver_water_factor(69.0, Some(water(1.0)));
        assert!(fade > 0.0 && fade < 1.0);
        assert_eq!(surface_carver_water_factor(71.0, Some(water(1.0))), 1.0);
        assert_eq!(surface_carver_water_factor(100.0, Some(water(1.0))), 1.0);
        assert_eq!(surface_carver_water_factor(64.0, None), 1.0);
    }

    #[test]
    fn weak_water_edge_blends_carver_instead_of_binary_cutoff() {
        let factor = surface_carver_water_factor(57.0, Some(water(0.25)));
        assert!(factor > 0.0 && factor < 1.0);
    }

    #[test]
    fn water_scan_is_skipped_without_carving_and_cached_once_per_column() {
        let mut nearby_water = None;
        let mut scans = 0;

        let no_carve = apply_carver_water_protection(0.0, 57.0, &mut nearby_water, || {
            scans += 1;
            Some(water(1.0))
        });
        assert_eq!(no_carve, 0.0);
        assert_eq!(scans, 0);

        let protected = apply_carver_water_protection(-4.0, 57.0, &mut nearby_water, || {
            scans += 1;
            Some(water(1.0))
        });
        let deep = apply_carver_water_protection(-4.0, 50.0, &mut nearby_water, || {
            scans += 1;
            Some(water(1.0))
        });
        let high = apply_carver_water_protection(-4.0, 71.0, &mut nearby_water, || {
            scans += 1;
            Some(water(1.0))
        });
        assert_eq!(protected, 0.0);
        assert_eq!(deep, -4.0);
        assert_eq!(high, -4.0);
        assert_eq!(scans, 1);
    }
}
