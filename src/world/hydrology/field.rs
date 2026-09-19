use bevy::prelude::*;

use crate::content::dimension_hydrology::DimensionHydrology;

use super::{
    constants::{MACRO_SAMPLE_GRID, RIVER_CARVE_DEPTH},
    drainage::DrainageNetwork,
    math::{ocean_continentalness_threshold, ocean_strength},
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
    ocean_weight: f32,
}

impl HydrologyField {
    pub fn new(
        seed: u64,
        sea_level: i32,
        settings: DimensionHydrology,
        ocean_weight: f32,
    ) -> Self {
        Self {
            seed,
            sea_level,
            settings,
            ocean_weight,
        }
    }

    pub fn biome_overlay(&self, continentalness: f32) -> HydrologyBiomeOverlay<'_> {
        let ocean_biome = self.settings.ocean_biome.as_deref();
        let ocean_weight = if ocean_biome.is_some() {
            ocean_strength(continentalness, self.ocean_weight)
        } else {
            0.0
        };

        HydrologyBiomeOverlay {
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

        let ocean_threshold = ocean_continentalness_threshold(self.ocean_weight);
        let mut drainage = DrainageNetwork::new(
            self.seed,
            ocean_threshold,
            self.sea_level as f32,
            self.ocean_weight,
            &mut sample,
        );
        let rivers = build_river_system(
            coord,
            self.seed,
            self.sea_level as f32,
            &self.settings.water_fluid,
            self.settings.river_weight,
            self.settings.lake_weight,
            &mut drainage,
        );

        HydrologyRegion {
            seed: self.seed,
            coord,
            river_graph: rivers.graph,
            river_carve_depth: RIVER_CARVE_DEPTH,
            water_bodies: rivers.water_bodies,
            sea_level: self.sea_level as f32,
            settings: self.settings.clone(),
            ocean_weight: self.ocean_weight,
            macro_samples,
        }
    }
}
