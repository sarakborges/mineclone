use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use bevy::prelude::*;

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
    region_cache: RwLock<HashMap<IVec3, Arc<GenerationRegion>>>,
}

impl WorldFeatureFields {
    pub fn new(seed: u64, sea_level: i32) -> Self {
        Self {
            hydrology: HydrologyField::new(seed.rotate_left(7), sea_level),
            cave_connectivity: CaveConnectivityField::new(seed.rotate_left(23)),
            geology: GeologyField::new(seed.rotate_left(41)),
            region_cache: RwLock::new(HashMap::new()),
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

    pub fn region(&self, coord: IVec3) -> Arc<GenerationRegion> {
        self.region_with_hydrology(coord, |field| {
            field.region(IVec2::new(coord.x, coord.z))
        })
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

        let region = Arc::new(GenerationRegion {
            coord,
            hydrology: hydrology_factory(&self.hydrology),
            cave_connectivity: self.cave_connectivity.region(coord),
            geology: self.geology.region(coord),
        });
        let mut cache = self
            .region_cache
            .write()
            .expect("generation region cache write lock was poisoned");

        cache.entry(coord).or_insert_with(|| region.clone()).clone()
    }

    pub fn cached_region_count(&self) -> usize {
        self.region_cache
            .read()
            .expect("generation region cache read lock was poisoned")
            .len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_cache_reuses_the_same_region() {
        let fields = WorldFeatureFields::new(42, 64);
        let first = fields.region(IVec3::new(2, 0, -1));
        let second = fields.region(IVec3::new(2, 0, -1));

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(fields.cached_region_count(), 1);
    }
}
