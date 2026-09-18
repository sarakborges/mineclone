use bevy::prelude::*;

use crate::{
    content::fluid::FluidRegistry,
    voxel::{
        chunk::{CHUNK_SIZE, VoxelChunk},
        fluid::{FluidCell, MAX_FLUID_LEVEL},
    },
    world::{
        cave_connectivity::CaveConnectivityRegion, generation::GenerationColumnSample,
        generation_region::GenerationRegion, hydrology::HydrologyWaterSample,
    },
};

use super::index::{column_index, voxel_index};

pub(super) struct FluidPassContext<'a> {
    pub(super) fluids: &'a FluidRegistry,
    pub(super) region: &'a GenerationRegion,
    pub(super) anchored_caves: Option<&'a CaveConnectivityRegion>,
    pub(super) underground_water_fluid: &'a str,
}

pub(super) fn rasterize_fluid_pass(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    columns: &[GenerationColumnSample],
    density: &[f32],
    pass: &FluidPassContext<'_>,
) {
    let underground_fluid_id = pass.anchored_caves.map(|_| {
        pass.fluids
            .id_of(pass.underground_water_fluid)
            .unwrap_or_else(|| {
                panic!(
                    "underground hydrology references missing fluid: {}",
                    pass.underground_water_fluid
                )
            })
    });

    chunk.edit_content(|chunk| {
        for local_z in 0..CHUNK_SIZE {
            for local_x in 0..CHUNK_SIZE {
                let world_x = chunk_origin.x + local_x as i32;
                let world_z = chunk_origin.z + local_z as i32;
                let horizontal = Vec2::new(world_x as f32 + 0.5, world_z as f32 + 0.5);
                let surface_height = columns[column_index(local_x, local_z)].surface_height as f32;
                // Rank only candidates with a plausible original floor. A
                // higher unsupported lake previously won water_at(), then got
                // rejected here, hiding an otherwise supported river below.
                let surface_water = pass
                    .region
                    .hydrology
                    .supported_water_at(horizontal, surface_height)
                    .filter(|water| surface_water_is_supported(*water, surface_height));
                let surface_fluid_id = surface_water.as_ref().map(|water| {
                    pass.fluids.id_of(water.fluid_id).unwrap_or_else(|| {
                        panic!("hydrology references missing fluid: {}", water.fluid_id)
                    })
                });

                for local_y in 0..CHUNK_SIZE {
                    if density[voxel_index(local_x, local_y, local_z)] > 0.0 {
                        continue;
                    }

                    let world_y = chunk_origin.y + local_y as i32;
                    if let (Some(water), Some(fluid_id)) = (surface_water.as_ref(), surface_fluid_id)
                        && world_y as f32 + 1.0 > water.bed_level
                        && let Some(level) = fluid_level_for_surface(water.water_level, world_y)
                    {
                        chunk.set_fluid(
                            local_x,
                            local_y,
                            local_z,
                            Some(FluidCell::source(fluid_id, level)),
                        );
                        continue;
                    }

                    let (Some(caves), Some(fluid_id)) = (pass.anchored_caves, underground_fluid_id)
                    else {
                        continue;
                    };
                    let position = Vec3::new(
                        world_x as f32 + 0.5,
                        world_y as f32 + 0.5,
                        world_z as f32 + 0.5,
                    );
                    let Some(water) = caves.underground_water_at(position) else {
                        continue;
                    };
                    if world_y as f32 + 1.0 <= water.bed_level {
                        continue;
                    }
                    let Some(level) = fluid_level_for_surface(water.water_level, world_y) else {
                        continue;
                    };

                    chunk.set_fluid(
                        local_x,
                        local_y,
                        local_z,
                        Some(FluidCell::source(fluid_id, level)),
                    );
                }
            }
        }
    });
}

fn surface_water_is_supported(water: HydrologyWaterSample<'_>, surface_height: f32) -> bool {
    // The original surface only decides whether there is a plausible floor.
    // Terrain above the water level is intentionally not a rejection condition:
    // hydrology may have carved that material away to create the lake/river and
    // rejecting the source afterwards leaves dry holes in otherwise valid channels.
    surface_height + 0.5 >= water.bed_level
}

fn fluid_level_for_surface(water_level: f32, world_y: i32) -> Option<u8> {
    let coverage = water_level - world_y as f32;
    if coverage <= 0.0 {
        return None;
    }

    let level = (coverage.clamp(0.0, 1.0) * MAX_FLUID_LEVEL as f32).ceil() as u8;
    Some(level.clamp(1, MAX_FLUID_LEVEL))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use smallvec::SmallVec;

    use crate::{
        content::{
            biome::{BiomeDefinition, BiomeRegistry}, biome_hydrology::BiomeHydrologyRules,
            builtin_ids::WATER_FLUID_ID, dimension::DimensionDefinition,
            dimension_hydrology::DimensionHydrology, fluid::FluidDefinition,
        },
        world::{
            biome_field::{BiomeField, VolumeBiomeRegion},
            generation::density::{DensityPassContext, sample_density_field},
            hydrology::{HydrologyField, HydrologySurfaceSample, HydrologyWaterKind},
            terrain::terrain_density,
        },
    };

    fn sample(
        kind: HydrologyWaterKind,
        water_level: f32,
        bed_level: f32,
    ) -> HydrologyWaterSample<'static> {
        HydrologyWaterSample {
            fluid_id: "asteria:test/water",
            water_level,
            bed_level,
            strength: 1.0,
            kind,
        }
    }

    #[test]
    fn fractional_water_surface_produces_partial_fluid_level() {
        assert_eq!(fluid_level_for_surface(10.25, 10), Some(2));
        assert_eq!(fluid_level_for_surface(10.75, 10), Some(6));
        assert_eq!(fluid_level_for_surface(10.0, 9), Some(MAX_FLUID_LEVEL));
        assert_eq!(fluid_level_for_surface(10.0, 10), None);
    }

    #[test]
    fn surface_water_rejects_columns_without_a_supporting_floor() {
        assert!(!surface_water_is_supported(
            sample(HydrologyWaterKind::Lake, 80.0, 72.0),
            60.0,
        ));
    }

    #[test]
    fn carved_surface_above_water_does_not_remove_valid_sources() {
        assert!(surface_water_is_supported(
            sample(HydrologyWaterKind::Lake, 80.0, 70.0),
            90.0,
        ));
        assert!(surface_water_is_supported(
            sample(HydrologyWaterKind::River, 80.0, 74.0),
            90.0,
        ));
    }

    #[test]
    fn generated_river_crossing_carves_and_fills_both_sides_of_region_seam() {
        // Production drainage and river generation over a deterministic
        // synthetic slope. Search only inside each region's own Z bounds:
        // scanning distant coordinates against a region at Z=0 would test
        // the wrong region and could produce a false seam regression.
        let field = HydrologyField::new(42, 64, DimensionHydrology::default(), 1.0, 1.0);
        let terrain = |position: Vec2| HydrologySurfaceSample {
            elevation: if position.x >= 256.0 {
                40.0
            } else {
                110.0 - position.x * 0.04
            },
            continentalness: if position.x >= 256.0 { 0.0 } else { 0.8 },
            biome_hydrology: BiomeHydrologyRules::default(),
        };
        let original_surface = 104.0;
        let crossing = (-2..=2).find_map(|region_z: i32| {
            let left = field.region_from_macro_terrain(IVec2::new(0, region_z), terrain);
            let right = field.region_from_macro_terrain(IVec2::new(1, region_z), terrain);
            (region_z * 128..(region_z + 1) * 128)
                .find_map(|world_z| {
                    let z = world_z as f32 + 0.5;
                    let left_water =
                        left.supported_water_at(Vec2::new(127.5, z), original_surface)?;
                    let right_water =
                        right.supported_water_at(Vec2::new(128.5, z), original_surface)?;
                    if left_water.kind != HydrologyWaterKind::River
                        || right_water.kind != HydrologyWaterKind::River
                    {
                        return None;
                    }
                    let y = left_water.water_level.min(right_water.water_level).floor() as i32 - 1;
                    // Prove the normal density pass really excavates terrain;
                    // an already-empty voxel cannot demonstrate carving.
                    if y < 0
                        || y >= original_surface as i32
                        || y as f32 + 1.0 <= left_water.bed_level.max(right_water.bed_level)
                    {
                        return None;
                    }
                    Some((world_z, y))
                })
                .map(|(world_z, world_y)| (left, right, world_z, world_y))
        });
        let (left_hydrology, right_hydrology, world_z, world_y) =
            crossing.expect("synthetic drainage must cross the region seam below original terrain");

        // A real Plains definition supplies the density pass's biome context.
        // The base columns remain a controlled flat 104-block fixture; unlike
        // v0.15.50, density is no longer a fabricated all-air vector.
        let mut dimension: DimensionDefinition = serde_json::from_str(include_str!(
            "../../../data/dimensions/overworld/dimension.json"
        ))
        .unwrap();
        dimension.biomes.retain(|biome| biome.id == "asteria:overworld/plains");
        dimension.hydrology.coast_biome = None;
        dimension.hydrology.ocean_biome = None;
        dimension.sea_level = 64;
        let mut biomes = BiomeRegistry::default();
        let plains: BiomeDefinition = serde_json::from_str(include_str!(
            "../../../data/dimensions/overworld/biomes/plains.json"
        ))
        .unwrap();
        biomes.insert(plains);
        let biome_field = BiomeField::from_dimension(&dimension, &biomes, 42, 1.0);
        let volume_region = VolumeBiomeRegion::default();

        let mut fluids = FluidRegistry::default();
        let water: FluidDefinition =
            serde_json::from_str(include_str!("../../../data/fluids/water.json")).unwrap();
        fluids.insert(water);
        let fluid_id = fluids.id_of(WATER_FLUID_ID).unwrap();
        let columns = (0..CHUNK_SIZE * CHUNK_SIZE)
            .map(|_| {
                let mut surface_influences = SmallVec::new();
                surface_influences.push((0, 1.0));
                GenerationColumnSample {
                    surface_height: original_surface as i32,
                    surface_influences,
                }
            })
            .collect::<Vec<_>>();
        let chunk_z = world_z.div_euclid(CHUNK_SIZE as i32);
        let chunk_y = world_y.div_euclid(CHUNK_SIZE as i32);
        let local_z = world_z.rem_euclid(CHUNK_SIZE as i32);
        let local_y = world_y.rem_euclid(CHUNK_SIZE as i32);

        for (chunk_x, hydrology, local_x) in
            [(7, left_hydrology, CHUNK_SIZE as i32 - 1), (8, right_hydrology, 0)]
        {
            let chunk_coord = IVec3::new(chunk_x, chunk_y, chunk_z);
            let chunk_origin = chunk_coord * CHUNK_SIZE as i32;
            let region = GenerationRegion {
                coord: IVec3::new(chunk_x.div_euclid(8), chunk_y.div_euclid(8), chunk_z.div_euclid(8)),
                hydrology: Arc::new(hydrology),
            };
            assert_eq!(region.coord.xz(), region.hydrology.coord);
            let density = sample_density_field(
                chunk_origin,
                &columns,
                &DensityPassContext {
                    region: &region,
                    volume_region: &volume_region,
                    anchored_caves: None,
                    biome_field: &biome_field,
                    biomes: &biomes,
                    dimension: &dimension,
                    sea_level: dimension.sea_level as f32,
                },
            );
            let density_index = voxel_index(local_x as usize, local_y as usize, local_z as usize);
            assert!(terrain_density(original_surface as i32, world_y) > 0.0);
            assert!(
                density.values[density_index] <= 0.0,
                "the production density pass must excavate the river at {chunk_coord:?}",
            );

            let mut chunk = VoxelChunk::empty();
            rasterize_fluid_pass(
                &mut chunk,
                chunk_origin,
                &columns,
                &density.values,
                &FluidPassContext {
                    fluids: &fluids,
                    region: &region,
                    anchored_caves: None,
                    underground_water_fluid: WATER_FLUID_ID,
                },
            );
            assert_eq!(
                chunk.fluid_at(local_x, local_y, local_z),
                Some(FluidCell::source(fluid_id, MAX_FLUID_LEVEL)),
                "physical river must fill both sides of the x=128 seam at {chunk_coord:?}",
            );
        }
    }
}
