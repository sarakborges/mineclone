use bevy::prelude::*;

use super::{
    cave_connectivity::CaveConnectivityField,
    generation_region::GenerationRegion,
    geology::GeologyField,
    hydrology::HydrologyField,
};

#[derive(Resource)]
pub struct WorldFeatureFields {
    hydrology: HydrologyField,
    cave_connectivity: CaveConnectivityField,
    geology: GeologyField,
}

impl WorldFeatureFields {
    pub fn new(seed: u64, sea_level: i32) -> Self {
        Self {
            hydrology: HydrologyField::new(seed.rotate_left(7), sea_level),
            cave_connectivity: CaveConnectivityField::new(seed.rotate_left(23)),
            geology: GeologyField::new(seed.rotate_left(41)),
        }
    }

    pub fn hydrology(&self) -> &HydrologyField {
        &self.hydrology
    }

    pub fn cave_connectivity(&self) -> &CaveConnectivityField {
        &self.cave_connectivity
    }

    pub fn geology(&self) -> &GeologyField {
        &self.geology
    }

    pub fn region(&self, coord: IVec3) -> GenerationRegion {
        GenerationRegion {
            coord,
            hydrology: self.hydrology.region(IVec2::new(coord.x, coord.z)),
            cave_connectivity: self.cave_connectivity.region(coord),
            geology: self.geology.region(coord),
        }
    }
}
