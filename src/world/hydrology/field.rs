use bevy::prelude::*;

use crate::content::dimension_hydrology::DimensionHydrology;

use super::{
    constants::RIVER_CARVE_DEPTH,
    drainage::DrainageNetwork,
    region::HydrologyRegion,
    river::build_river_system,
    types::HydrologySurfaceSample,
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

    pub fn region_from_macro_terrain(
        &self,
        coord: IVec2,
        spawn_rivers: bool,
        spawn_lakes: bool,
        mut sample: impl FnMut(Vec2) -> HydrologySurfaceSample,
    ) -> HydrologyRegion {
        let mut drainage =
            DrainageNetwork::new(self.seed, self.sea_level as f32, &mut sample);
        let rivers = build_river_system(
            coord,
            self.seed,
            self.sea_level as f32,
            &self.settings.water_fluid,
            if spawn_rivers { self.settings.river_weight } else { 0.0 },
            if spawn_lakes { self.settings.lake_weight } else { 0.0 },
            spawn_lakes,
            &mut drainage,
        );

        HydrologyRegion {
            river_graph: rivers.graph,
            river_carve_depth: RIVER_CARVE_DEPTH,
            water_bodies: rivers.water_bodies,
            settings: self.settings.clone(),
        }
    }
}
