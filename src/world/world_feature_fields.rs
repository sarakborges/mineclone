use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use bevy::prelude::*;

use crate::content::dimension_hydrology::DimensionHydrology;

use super::{
    biome_field::VolumeBiomeRegion,
    cave_connectivity::{CaveConnectivityField, CaveConnectivityRegion},
    generation::columns::GenerationColumnSample,
    generation_region::GenerationRegion,
    geology::GeologyField,
    hydrology::{HydrologyBiomeOverlay, HydrologyField, HydrologyRegion},
};

#[derive(Resource)]
pub(crate) struct WorldFeatureFields {
    hydrology: HydrologyField,
    cave_connectivity: CaveConnectivityField,
    geology: GeologyField,
    hydrology_cache: RwLock<HashMap<IVec2, Arc<HydrologyRegion>>>,
    generation_column_cache: RwLock<HashMap<IVec2, Arc<Vec<GenerationColumnSample>>>>,
    volume_biome_cache: RwLock<HashMap<IVec3, Arc<VolumeBiomeRegion>>>,
    cave_cache: RwLock<HashMap<IVec3, Option<Arc<CaveConnectivityRegion>>>>,
    region_cache: RwLock<HashMap<IVec3, Arc<GenerationRegion>>>,
}

impl WorldFeatureFields {
    pub(crate) fn new(seed: u64, sea_level: i32, hydrology: DimensionHydrology) -> Self {
        Self {
            hydrology: HydrologyField::new(seed.rotate_left(7), sea_level, hydrology),
            cave_connectivity: CaveConnectivityField::new(seed.rotate_left(23)),
            geology: GeologyField::new(seed.rotate_left(41)),
            hydrology_cache: RwLock::new(HashMap::new()),
            generation_column_cache: RwLock::new(HashMap::new()),
            volume_biome_cache: RwLock::new(HashMap::new()),
            cave_cache: RwLock::new(HashMap::new()),
            region_cache: RwLock::new(HashMap::new()),
        }
    }

    pub(crate) fn hydrology_biome_overlay(
        &self,
        continentalness: f32,
    ) -> HydrologyBiomeOverlay<'_> {
        self.hydrology.biome_overlay(continentalness)
    }

    pub(crate) fn generation_columns(
        &self,
        coord: IVec2,
        factory: impl FnOnce() -> Vec<GenerationColumnSample>,
    ) -> Arc<Vec<GenerationColumnSample>> {
        if let Some(cached) = self
            .generation_column_cache
            .read()
            .expect("generation column cache read lock was poisoned")
            .get(&coord)
            .cloned()
        {
            return cached;
        }

        let columns = Arc::new(factory());
        let mut cache = self
            .generation_column_cache
            .write()
            .expect("generation column cache write lock was poisoned");

        cache.entry(coord).or_insert_with(|| columns.clone()).clone()
    }

    pub(crate) fn volume_biome_region(
        &self,
        coord: IVec3,
        factory: impl FnOnce() -> VolumeBiomeRegion,
    ) -> Arc<VolumeBiomeRegion> {
        if let Some(cached) = self
            .volume_biome_cache
            .read()
            .expect("volume biome cache read lock was poisoned")
            .get(&coord)
            .cloned()
        {
            return cached;
        }

        let region = Arc::new(factory());
        let mut cache = self
            .volume_biome_cache
            .write()
            .expect("volume biome cache write lock was poisoned");

        cache.entry(coord).or_insert_with(|| region.clone()).clone()
    }

    pub(crate) fn cave_region(
        &self,
        coord: IVec3,
        factory: impl FnOnce(&CaveConnectivityField) -> Option<CaveConnectivityRegion>,
    ) -> Option<Arc<CaveConnectivityRegion>> {
        if let Some(cached) = self
            .cave_cache
            .read()
            .expect("cave region cache read lock was poisoned")
            .get(&coord)
            .cloned()
        {
            return cached;
        }

        let region = factory(&self.cave_connectivity).map(Arc::new);
        let mut cache = self
            .cave_cache
            .write()
            .expect("cave region cache write lock was poisoned");

        cache.entry(coord).or_insert_with(|| region.clone()).clone()
    }

    pub(crate) fn region_with_hydrology(
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

    #[cfg(test)]
    fn cached_generation_column_count(&self) -> usize {
        self.generation_column_cache
            .read()
            .expect("generation column cache read lock was poisoned")
            .len()
    }

    #[cfg(test)]
    fn cached_volume_biome_region_count(&self) -> usize {
        self.volume_biome_cache
            .read()
            .expect("volume biome cache read lock was poisoned")
            .len()
    }

    #[cfg(test)]
    fn cached_cave_region_count(&self) -> usize {
        self.cave_cache
            .read()
            .expect("cave region cache read lock was poisoned")
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

    #[test]
    fn generation_column_cache_reuses_horizontal_chunk_samples() {
        let fields = WorldFeatureFields::new(42, 64, DimensionHydrology::default());
        let coord = IVec2::new(3, -2);
        let first = fields.generation_columns(coord, Vec::new);
        let second = fields.generation_columns(coord, || {
            panic!("cached generation columns should not rebuild")
        });

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(fields.cached_generation_column_count(), 1);
    }

    #[test]
    fn volume_biome_cache_reuses_the_same_generation_region_result() {
        let fields = WorldFeatureFields::new(42, 64, DimensionHydrology::default());
        let coord = IVec3::new(1, 2, 3);
        let first = fields.volume_biome_region(coord, VolumeBiomeRegion::default);
        let second = fields.volume_biome_region(coord, || {
            panic!("cached volume biome region should not rebuild")
        });

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(fields.cached_volume_biome_region_count(), 1);
    }

    #[test]
    fn cave_cache_reuses_the_same_generation_region_result() {
        let fields = WorldFeatureFields::new(42, 64, DimensionHydrology::default());
        let coord = IVec3::new(1, 2, 3);
        let first = fields
            .cave_region(coord, |_| Some(CaveConnectivityRegion::default()))
            .expect("test cave region should exist");
        let second = fields
            .cave_region(coord, |_| panic!("cached cave region should not rebuild"))
            .expect("cached cave region should exist");

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(fields.cached_cave_region_count(), 1);
    }
}
