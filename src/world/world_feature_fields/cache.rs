use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    sync::{Arc, OnceLock, RwLock},
};

use bevy::prelude::*;

use crate::voxel::chunk::CHUNK_SIZE;

use super::super::{
    biome_field::VolumeBiomeRegion,
    cave_connectivity::CaveConnectivityRegion,
    generation::GenerationColumnSample,
    generation_region::{GenerationRegion, generation_region_coord},
    hydrology::HydrologyRegion,
};

const CACHE_REGION_MARGIN: i32 = 1;

type StructureOriginEntry = Arc<OnceLock<Option<i32>>>;
type StructureOriginAnchors = HashMap<IVec2, StructureOriginEntry>;
type StructureOriginEntries = HashMap<String, StructureOriginAnchors>;

struct ConcurrentCache<K, V> {
    name: &'static str,
    entries: RwLock<HashMap<K, Arc<OnceLock<V>>>>,
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
        let cached = self
            .entries
            .read()
            .unwrap_or_else(|_| panic!("{} read lock was poisoned", self.name))
            .get(&key)
            .cloned();
        let entry = cached.unwrap_or_else(|| {
            let mut entries = self
                .entries
                .write()
                .unwrap_or_else(|_| panic!("{} write lock was poisoned", self.name));

            entries
                .entry(key)
                .or_insert_with(|| Arc::new(OnceLock::new()))
                .clone()
        });

        entry.get_or_init(factory).clone()
    }

    fn retain(&self, mut predicate: impl FnMut(&K) -> bool) {
        self.entries
            .write()
            .unwrap_or_else(|_| panic!("{} write lock was poisoned", self.name))
            .retain(|key, _| predicate(key));
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries
            .read()
            .unwrap_or_else(|_| panic!("{} read lock was poisoned", self.name))
            .len()
    }
}

struct StructureOriginCache {
    entries: RwLock<StructureOriginEntries>,
}

impl StructureOriginCache {
    fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    fn get_or_insert_with(
        &self,
        structure_id: &str,
        anchor: IVec2,
        factory: impl FnOnce() -> Option<i32>,
    ) -> Option<i32> {
        let cached = self
            .entries
            .read()
            .expect("structure origin cache read lock was poisoned")
            .get(structure_id)
            .and_then(|anchors| anchors.get(&anchor))
            .cloned();
        let entry = cached.unwrap_or_else(|| {
            let mut entries = self
                .entries
                .write()
                .expect("structure origin cache write lock was poisoned");

            if let Some(cached) = entries
                .get(structure_id)
                .and_then(|anchors| anchors.get(&anchor))
                .cloned()
            {
                return cached;
            }

            let entry = Arc::new(OnceLock::new());
            if let Some(anchors) = entries.get_mut(structure_id) {
                anchors.insert(anchor, entry.clone());
            } else {
                entries.insert(structure_id.to_owned(), HashMap::from([(anchor, entry.clone())]));
            }
            entry
        });

        *entry.get_or_init(factory)
    }

    fn retain(&self, mut predicate: impl FnMut(IVec2) -> bool) {
        let mut entries = self
            .entries
            .write()
            .expect("structure origin cache write lock was poisoned");

        for anchors in entries.values_mut() {
            anchors.retain(|anchor, _| predicate(*anchor));
        }
        entries.retain(|_, anchors| !anchors.is_empty());
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries
            .read()
            .expect("structure origin cache read lock was poisoned")
            .values()
            .map(HashMap::len)
            .sum()
    }
}

pub(super) struct FeatureCaches {
    hydrology: ConcurrentCache<IVec2, Arc<HydrologyRegion>>,
    generation_columns: ConcurrentCache<IVec2, Arc<Vec<GenerationColumnSample>>>,
    volume_biomes: ConcurrentCache<IVec3, Arc<VolumeBiomeRegion>>,
    caves: ConcurrentCache<IVec3, Option<Arc<CaveConnectivityRegion>>>,
    regions: ConcurrentCache<IVec3, Arc<GenerationRegion>>,
    structure_origins: StructureOriginCache,
}

impl FeatureCaches {
    pub(super) fn new() -> Self {
        Self {
            hydrology: ConcurrentCache::new("hydrology region cache"),
            generation_columns: ConcurrentCache::new("generation column cache"),
            volume_biomes: ConcurrentCache::new("volume biome cache"),
            caves: ConcurrentCache::new("cave region cache"),
            regions: ConcurrentCache::new("generation region cache"),
            structure_origins: StructureOriginCache::new(),
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
            .get_or_insert_with(structure_id, anchor, factory)
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
        let generation_regions = desired
            .iter()
            .copied()
            .map(generation_region_coord)
            .collect::<HashSet<_>>();
        let mut retained_regions = HashSet::new();

        // Many desired chunks share one 8x8x8 generation region. Expand the
        // cache margin once per unique region rather than once per chunk.
        for region in generation_regions {
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
            .retain(|coord| horizontal_chunks.contains(coord));
        self.volume_biomes
            .retain(|coord| retained_regions.contains(coord));
        self.caves
            .retain(|coord| retained_regions.contains(coord));
        self.regions
            .retain(|coord| retained_regions.contains(coord));
        self.hydrology
            .retain(|coord| retained_hydrology.contains(coord));
        self.structure_origins.retain(|anchor| {
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

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc, Barrier,
            atomic::{AtomicUsize, Ordering},
        },
        thread,
        time::Duration,
    };

    use super::ConcurrentCache;

    #[test]
    fn concurrent_cache_runs_factory_once_per_key() {
        const WORKERS: usize = 8;

        let cache = Arc::new(ConcurrentCache::<u32, u32>::new("test cache"));
        let calls = Arc::new(AtomicUsize::new(0));
        let start = Arc::new(Barrier::new(WORKERS));
        let mut handles = Vec::with_capacity(WORKERS);

        for _ in 0..WORKERS {
            let cache = cache.clone();
            let calls = calls.clone();
            let start = start.clone();
            handles.push(thread::spawn(move || {
                start.wait();
                let value = cache.get_or_insert_with(7, || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(10));
                    42
                });
                assert_eq!(value, 42);
            }));
        }

        for handle in handles {
            handle.join().expect("cache worker should finish");
        }

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(cache.len(), 1);
    }
}
