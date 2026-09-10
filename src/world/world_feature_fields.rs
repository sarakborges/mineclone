use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use bevy::prelude::*;

use crate::content::dimension_hydrology::DimensionHydrology;

use super::{
    cave_connectivity::CaveConnectivityField,
    generation_region::GenerationRegion,
    geology::GeologyField,
    hydrology::{HydrologyField, HydrologyRegion},
};

#[derive(Resource)]
pub struct WorldFeatureFields {
    hydrology: HydrologyField,
    cave_connectivity: CaveConnectivityField,
    geology: GeologyField,
    hydrology_cache: RwLock<HashMap<IVec2, Arc<HydrologyRegion>>>,
    region_cache: RwLock<HashMap<IVec3, Arc<GenerationRegion>>>,
}

impl WorldFeatureFields {
    pub fn new(seed: u64, sea_level: i32, hydrology: DimensionHydrology) -> Self {
        Self {
            hydrology: HydrologyField::new(seed.rotate_left(7), sea_level, hydrology),
            cave_connectivity: CaveConnectivityField::new(seed.rotate_left(23)),
            geology: GeologyField::new(seed.rotate_left(41)),
            hydrology_cache: RwLock::new(HashMap::new()),
            region_cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn hydrology(&self) -> &HydrologyField {
        &self.hydrology
    }

    pub fn cave_connectivity(&self) -> &CaveConnectivityField {
        &self.cave_connectivity
    }

    pub fn region_with_hydrology(
        &self,
        coord: IVec3,
        hydrology_factory: impl FnOnce(&HydrologyField) -> HydrologyRegion,
    ) -> Arc<GenerationRegion> {
        if let Some(region) = self
            .region_cache
            .read()
            .expect("generation region cache read lock was poisoned")
            .get(&coord)
            .cloned()
        {
            return region;
        }

        let hydrology_coord = IVec2::new(coord.x, coord.z);
        let hydrology = if let Some(region) = self
            .hydrology_cache
            .read()
            .expect("hydrology region cache read lock was poisoned")
            .get(&hydrology_coord)
            .cloned()
        {
            region
        } else {
            let region = Arc::new(hydrology_factory(&self.hydrology));
            let mut cache = self
                .hydrology_cache
                .write()
                .expect("hydrology region cache write lock was poisoned");

            cache
                .entry(hydrology_coord)
                .or_insert_with(|| region.clone())
                .clone()
        };
        let region = Arc::new(GenerationRegion {
            coord,
            hydrology,
            geology: self.geology.region(coord),
        });
        let mut cache = self
            .region_cache
            .write()
            .expect("generation region cache write lock was poisoned");

        cache.entry(coord).or_insert_with(|| region.clone()).clone()
    }

    #[cfg(test)]
    fn cached_region_count(&self) -> usize {
        self.region_cache
            .read()
            .expect("generation region cache read lock was poisoned")
            .len()
    }

    #[cfg(test)]
    fn cached_hydrology_region_count(&self) -> usize {
        self.hydrology_cache
            .read()
            .expect("hydrology region cache read lock was poisoned")
            .len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_cache_reuses_hydrology_across_vertical_regions() {
        let fields = WorldFeatureFields::new(42, 64, DimensionHydrology::default());
        let coord = IVec3::new(2, 0, -1);
        let first = fields.region_with_hydrology(coord, |hydrology| {
            hydrology.region_from_macro_terrain(IVec2::new(coord.x, coord.z), |_| {
                super::super::hydrology::HydrologySurfaceSample {
                    elevation: 64.0,
                    continentalness: 0.5,
                    biome_hydrology: crate::content::biome_hydrology::BiomeHydrology::default(),
                }
            })
        });
        let second = fields.region_with_hydrology(coord, |_| {
            panic!("cached generation region should not rebuild hydrology")
        });
        let vertical_coord = coord + IVec3::Y;
        let vertical = fields.region_with_hydrology(vertical_coord, |_| {
            panic!("vertical generation region should reuse cached 2D hydrology")
        });

        assert!(Arc::ptr_eq(&first, &second));
        assert!(Arc::ptr_eq(&first.hydrology, &vertical.hydrology));
        assert_eq!(fields.cached_region_count(), 2);
        assert_eq!(fields.cached_hydrology_region_count(), 1);
    }
}
