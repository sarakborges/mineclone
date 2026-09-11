use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    sync::{Arc, RwLock},
};

use bevy::prelude::*;

use crate::voxel::chunk::CHUNK_SIZE;

use super::super::{
    biome_field::VolumeBiomeRegion,
    cave_connectivity::CaveConnectivityRegion,
    generation::columns::GenerationColumnSample,
    generation_region::{GenerationRegion, generation_region_coord},
    hydrology::HydrologyRegion,
};

const CACHE_REGION_MARGIN: i32 = 1;

struct ConcurrentCache<K, V> {
    name: &'static str,
    entries: RwLock<HashMap<K, V>>,
}

impl<K, V> ConcurrentCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    fn new(name: &'static str) -> Self {
        Self {
            name,
            entries: RwLock::new(HashMap::new()),
        }
    }

    fn get_or_insert_with(&self, key: K, factory: impl FnOnce() -> V) -> V {
        if let Some(cached) = self
            .entries
            .read()
            .unwrap_or_else(|_| panic!("{} read lock was poisoned", self.name))
            .get(&key)
            .cloned()
        {
            return cached;
        }

        let value = factory();
        let mut entries = self
            .entries
            .write()
            .unwrap_or_else(|_| panic!("{} write lock was poisoned", self.name));

        entries
            .entry(key)
            .or_insert_with(|| value.clone())
            .clone()
    }

    fn retain(&self, mut predicate: impl FnMut(&K, &V) -> bool) {
        self.entries
            .write()
            .unwrap_or_else(|_| panic!("{} write lock was poisoned", self.name))
            .retain(|key, value| predicate(key, value));
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries
            .read()
            .unwrap_or_else(|_| panic!("{} read lock was poisoned", self.name))
            .len()
    }
}

pub(super) struct FeatureCaches {
    hydrology: ConcurrentCache<IVec2, Arc<HydrologyRegion>>,
    generation_columns: ConcurrentCache<IVec2, Arc<Vec<GenerationColumnSample>>>,
    volume_biomes: ConcurrentCache<IVec3, Arc<VolumeBiomeRegion>>,
    caves: ConcurrentCache<IVec3, Option<Arc<CaveConnectivityRegion>>>,
    regions: ConcurrentCache<IVec3, Arc<GenerationRegion>>,
    structure_origins: ConcurrentCache<(String, IVec2), Option<i32>>,
}

impl FeatureCaches {
    pub(super) fn new() -> Self {
        Self {
            hydrology: ConcurrentCache::new("hydrology region cache"),
            generation_columns: ConcurrentCache::new("generation column cache"),
            volume_biomes: ConcurrentCache::new("volume biome cache"),
            caves: ConcurrentCache::new("cave region cache"),
            regions: ConcurrentCache::new("generation region cache"),
            structure_origins: ConcurrentCache::new("structure origin cache"),
        }
    }

    pub(super) fn generation_columns(
        &self,
        coord: IVec2,
        factory: impl FnOnce() -> Vec<GenerationColumnSample>,
    ) -> Arc<Vec<GenerationColumnSample>> {
        self.generation_columns
            .get_or_insert_with(coord, || Arc::new(factory()))
    }

    pub(super) fn volume_biome_region(
        &self,
        coord: IVec3,
        factory: impl FnOnce() -> VolumeBiomeRegion,
    ) -> Arc<VolumeBiomeRegion> {
        self.volume_biomes
            .get_or_insert_with(coord, || Arc::new(factory()))
    }

    pub(super) fn cave_region(
        &self,
        coord: IVec3,
        factory: impl FnOnce() -> Option<CaveConnectivityRegion>,
    ) -> Option<Arc<CaveConnectivityRegion>> {
        self.caves
            .get_or_insert_with(coord, || factory().map(Arc::new))
    }

    pub(super) fn structure_origin_y(
        &self,
        structure_id: &str,
        anchor: IVec2,
        factory: impl FnOnce() -> Option<i32>,
    ) -> Option<i32> {
        self.structure_origins
            .get_or_insert_with((structure_id.to_owned(), anchor), factory)
    }

    pub(super) fn hydrology_region(
        &self,
        coord: IVec2,
        factory: impl FnOnce() -> HydrologyRegion,
    ) -> Arc<HydrologyRegion> {
        self.hydrology
            .get_or_insert_with(coord, || Arc::new(factory()))
    }

    pub(super) fn generation_region(
        &self,
        coord: IVec3,
        factory: impl FnOnce() -> GenerationRegion,
    ) -> Arc<GenerationRegion> {
        self.regions
            .get_or_insert_with(coord, || Arc::new(factory()))
    }

    pub(super) fn retain_for_chunks(&self, desired: &HashSet<IVec3>) {
        let horizontal_chunks = desired.iter().map(|coord| coord.xz()).collect::<HashSet<_>>();
        let mut retained_regions = HashSet::new();

        for &chunk in desired {
            let region = generation_region_coord(chunk);
            for y in (region.y - CACHE_REGION_MARGIN).max(0)..=(region.y + CACHE_REGION_MARGIN) {
                for z in (region.z - CACHE_REGION_MARGIN)..=(region.z + CACHE_REGION_MARGIN) {
                    for x in (region.x - CACHE_REGION_MARGIN)..=(region.x + CACHE_REGION_MARGIN) {
                        retained_regions.insert(IVec3::new(x, y, z));
                    }
                }
            }
        }

        let retained_hydrology = retained_regions
            .iter()
            .map(|coord| coord.xz())
            .collect::<HashSet<_>>();

        self.generation_columns
            .retain(|coord, _| horizontal_chunks.contains(coord));
        self.volume_biomes
            .retain(|coord, _| retained_regions.contains(coord));
        self.caves
            .retain(|coord, _| retained_regions.contains(coord));
        self.regions
            .retain(|coord, _| retained_regions.contains(coord));
        self.hydrology
            .retain(|coord, _| retained_hydrology.contains(coord));
        self.structure_origins.retain(|(_, anchor), _| {
            let chunk_size = CHUNK_SIZE as i32;
            let chunk = IVec2::new(
                anchor.x.div_euclid(chunk_size),
                anchor.y.div_euclid(chunk_size),
            );
            horizontal_chunks.contains(&chunk)
        });
    }

    #[cfg(test)]
    pub(super) fn region_count(&self) -> usize {
        self.regions.len()
    }

    #[cfg(test)]
    pub(super) fn hydrology_region_count(&self) -> usize {
        self.hydrology.len()
    }

    #[cfg(test)]
    pub(super) fn generation_column_count(&self) -> usize {
        self.generation_columns.len()
    }

    #[cfg(test)]
    pub(super) fn volume_biome_region_count(&self) -> usize {
        self.volume_biomes.len()
    }

    #[cfg(test)]
    pub(super) fn cave_region_count(&self) -> usize {
        self.caves.len()
    }

    #[cfg(test)]
    pub(super) fn structure_origin_count(&self) -> usize {
        self.structure_origins.len()
    }
}
