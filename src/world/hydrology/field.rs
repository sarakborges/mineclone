use bevy::prelude::*;

use crate::content::dimension_hydrology::DimensionHydrology;

use super::{
    constants::{MACRO_SAMPLE_GRID, RIVER_CARVE_DEPTH},
    drainage::DrainageNetwork,
    math::ocean_strength,
    region::HydrologyRegion,
    river::build_river_system,
    spatial::macro_sample_position,
    types::{HydrologyBiomeOverlay, HydrologyMacroSample, HydrologySurfaceSample},
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

    pub fn region_from_macro_terrain(
        &self,
        coord: IVec2,
        mut sample: impl FnMut(Vec2) -> HydrologySurfaceSample,
    ) -> HydrologyRegion {
        let mut macro_samples = Vec::with_capacity(MACRO_SAMPLE_GRID * MACRO_SAMPLE_GRID);

        for z in 0..MACRO_SAMPLE_GRID {
            for x in 0..MACRO_SAMPLE_GRID {
                let position = macro_sample_position(coord, x, z);
                let surface = sample(position);
                macro_samples.push(HydrologyMacroSample {
                    elevation: surface.elevation,
                    continentalness: surface.continentalness,
                });
            }
        }

        let mut drainage = DrainageNetwork::new(self.seed, &mut sample);
        let rivers = build_river_system(
            coord,
            self.seed,
            self.sea_level as f32,
            &self.settings.water_fluid,
            &mut drainage,
        );

        HydrologyRegion {
            coord,
            river_graph: rivers.graph,
            river_carve_depth: RIVER_CARVE_DEPTH,
            water_bodies: rivers.water_bodies,
            sea_level: self.sea_level as f32,
            settings: self.settings.clone(),
            macro_samples,
        }
    }
}
