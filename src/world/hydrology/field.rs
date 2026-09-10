use bevy::prelude::*;

use crate::{content::dimension_hydrology::DimensionHydrology, world::feature_graph::FeatureGraph};

use super::{
    constants::{
        MACRO_SAMPLE_GRID, OCEAN_CONTINENTALNESS_THRESHOLD, RIVER_CARVE_DEPTH,
        RIVER_MAXIMUM_RADIUS, RIVER_MINIMUM_RADIUS, RIVER_SOURCE_MARGIN_CELLS,
    },
    drainage::{drainage_neighbors, drainage_node, select_downstream},
    lake::lake_for_local_basin,
    math::{cell_hash, hash_unit, lerp, ocean_strength},
    region::HydrologyRegion,
    spatial::{edge_intersects_region, macro_sample_position, water_body_intersects_region},
    types::{
        HydrologyBiomeOverlay, HydrologyMacroSample, HydrologySurfaceSample,
        HydrologyTerrainSummary,
    },
};

#[derive(Clone, Debug)]
pub struct HydrologyField {
    seed: u64,
    sea_level: i32,
    settings: DimensionHydrology,
}

impl HydrologyField {
    pub fn new(seed: u64, sea_level: i32, settings: DimensionHydrology) -> Self {
        Self {
            seed,
            sea_level,
            settings,
        }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn sea_level(&self) -> i32 {
        self.sea_level
    }

    pub fn biome_overlay(&self, continentalness: f32) -> HydrologyBiomeOverlay<'_> {
        let strength = ocean_strength(continentalness);
        let mut surface_weight = (1.0 - strength * 2.0).clamp(0.0, 1.0);
        let mut coast_weight = (1.0 - (strength * 2.0 - 1.0).abs()).clamp(0.0, 1.0);
        let mut ocean_weight = (strength * 2.0 - 1.0).clamp(0.0, 1.0);
        let coast_biome = self.settings.coast_biome.as_deref();
        let ocean_biome = self.settings.ocean_biome.as_deref();

        if coast_biome.is_none() {
            if strength < 0.5 {
                surface_weight += coast_weight;
            } else {
                ocean_weight += coast_weight;
            }
            coast_weight = 0.0;
        }

        if ocean_biome.is_none() {
            if coast_biome.is_some() {
                coast_weight += ocean_weight;
            } else {
                surface_weight += ocean_weight;
            }
            ocean_weight = 0.0;
        }

        let total = surface_weight + coast_weight + ocean_weight;
        if total > f32::EPSILON {
            surface_weight /= total;
            coast_weight /= total;
            ocean_weight /= total;
        }

        HydrologyBiomeOverlay {
            surface_weight,
            coast_biome,
            coast_weight,
            ocean_biome,
            ocean_weight,
        }
    }

    pub fn region(&self, coord: IVec2) -> HydrologyRegion {
        HydrologyRegion::empty(coord, self.sea_level as f32, self.settings.clone())
    }

    pub fn region_from_macro_terrain(
        &self,
        coord: IVec2,
        mut sample: impl FnMut(Vec2) -> HydrologySurfaceSample,
    ) -> HydrologyRegion {
        let mut minimum_elevation = f32::MAX;
        let mut maximum_elevation = f32::MIN;
        let mut elevation_sum = 0.0;
        let mut continentalness_sum = 0.0;
        let mut count = 0.0;
        let mut macro_samples = Vec::with_capacity(MACRO_SAMPLE_GRID * MACRO_SAMPLE_GRID);

        for z in 0..MACRO_SAMPLE_GRID {
            for x in 0..MACRO_SAMPLE_GRID {
                let position = macro_sample_position(coord, x, z);
                let surface = sample(position);

                minimum_elevation = minimum_elevation.min(surface.elevation);
                maximum_elevation = maximum_elevation.max(surface.elevation);
                elevation_sum += surface.elevation;
                continentalness_sum += surface.continentalness;
                count += 1.0;
                macro_samples.push(HydrologyMacroSample {
                    elevation: surface.elevation,
                    continentalness: surface.continentalness,
                });
            }
        }

        let mut river_graph = FeatureGraph::default();
        let mut water_bodies = Vec::new();

        for dz in -RIVER_SOURCE_MARGIN_CELLS..=RIVER_SOURCE_MARGIN_CELLS {
            for dx in -RIVER_SOURCE_MARGIN_CELLS..=RIVER_SOURCE_MARGIN_CELLS {
                let source_cell = coord + IVec2::new(dx, dz);
                let source = drainage_node(source_cell, self.seed, &mut sample);
                let neighbors = drainage_neighbors(source_cell, self.seed, &mut sample);

                if let Some(downstream) = select_downstream(source, &neighbors) {
                    if source.continentalness > OCEAN_CONTINENTALNESS_THRESHOLD
                        && source.biome_hydrology.can_generate_river
                        && downstream.biome_hydrology.can_generate_river
                        && edge_intersects_region(coord, source.position, downstream.position)
                    {
                        let hash = cell_hash(source_cell, self.seed ^ 0x6a09_e667_f3bc_c909);
                        let base_radius =
                            lerp(RIVER_MINIMUM_RADIUS, RIVER_MAXIMUM_RADIUS, hash_unit(hash));
                        let width_multiplier = (source.biome_hydrology.river_width_multiplier
                            + downstream.biome_hydrology.river_width_multiplier)
                            * 0.5;
                        let radius = base_radius * width_multiplier;

                        if radius > f32::EPSILON {
                            let from = river_graph.add_node(Vec3::new(
                                source.position.x,
                                (source.elevation - 0.75).max(1.0),
                                source.position.y,
                            ));
                            let to = river_graph.add_node(Vec3::new(
                                downstream.position.x,
                                (downstream.elevation - 0.75).max(1.0),
                                downstream.position.y,
                            ));
                            river_graph.add_edge(from, to, radius, radius * 1.15);
                        }
                    }
                } else if let Some(lake) = lake_for_local_basin(
                    source_cell,
                    source,
                    &neighbors,
                    self.seed,
                    self.sea_level as f32,
                    &self.settings.water_fluid,
                ) {
                    if water_body_intersects_region(coord, &lake) {
                        water_bodies.push(lake);
                    }
                }
            }
        }

        HydrologyRegion {
            coord,
            terrain: HydrologyTerrainSummary {
                minimum_elevation,
                maximum_elevation,
                mean_elevation: elevation_sum / count,
                mean_continentalness: continentalness_sum / count,
            },
            river_graph,
            river_carve_depth: RIVER_CARVE_DEPTH,
            water_bodies,
            sea_level: self.sea_level as f32,
            settings: self.settings.clone(),
            macro_samples,
        }
    }
}
