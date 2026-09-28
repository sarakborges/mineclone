use std::sync::{
    Arc, LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

use bevy::{platform::collections::HashMap, prelude::*};

use crate::voxel::chunk::CHUNK_SIZE;

use super::{SurfaceSiteCacheEntry, constants::SITE_SEARCH_RADIUS};

/// Disposable acceleration for deterministic surface-biome site selection.
///
/// Clones intentionally share the same cache so async generation snapshots can
/// reuse resolved sites. Clearing or retaining entries must never change the
/// deterministic biome field; a cache miss simply recomputes the same result.
#[derive(Clone, Default)]
pub(crate) struct SurfaceSiteCache {
    entries: Arc<RwLock<HashMap<IVec2, SurfaceSiteCacheEntry>>>,
}

impl SurfaceSiteCache {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn read(
        &self,
    ) -> LockResult<RwLockReadGuard<'_, HashMap<IVec2, SurfaceSiteCacheEntry>>> {
        self.entries.read()
    }

    pub(super) fn write(
        &self,
    ) -> LockResult<RwLockWriteGuard<'_, HashMap<IVec2, SurfaceSiteCacheEntry>>> {
        self.entries.write()
    }

    pub(super) fn clear(&self) {
        self.entries
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }

    pub(super) fn retain_around(
        &self,
        center_chunk: IVec2,
        radius_chunks: i32,
        surface_site_spacing: Vec2,
    ) {
        let chunk_size = CHUNK_SIZE as i32;
        let center_world = (center_chunk * chunk_size).as_vec2()
            + Vec2::splat(CHUNK_SIZE as f32 * 0.5);
        let center_cell = IVec2::new(
            (center_world.x / surface_site_spacing.x).round() as i32,
            (center_world.y / surface_site_spacing.y).round() as i32,
        );
        let world_radius = radius_chunks
            .max(0)
            .saturating_add(2)
            .saturating_mul(chunk_size) as f32;
        let padding = SITE_SEARCH_RADIUS + 2;
        let radius_x = (world_radius / surface_site_spacing.x).ceil() as i32 + padding;
        let radius_z = (world_radius / surface_site_spacing.y).ceil() as i32 + padding;

        self.entries
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|cell, _| {
                (cell.x - center_cell.x).abs() <= radius_x
                    && (cell.y - center_cell.y).abs() <= radius_z
            });
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(index: usize) -> SurfaceSiteCacheEntry {
        SurfaceSiteCacheEntry {
            position: Vec2::ZERO,
            biome_index: index,
        }
    }

    #[test]
    fn clones_share_entries_and_invalidation() {
        let cache = SurfaceSiteCache::new();
        cache
            .write()
            .expect("surface-site cache write lock should not be poisoned")
            .insert(IVec2::new(2, -3), entry(4));

        let clone = cache.clone();
        assert_eq!(clone.len(), 1);

        clone.clear();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn retention_drops_sites_outside_streaming_bound() {
        let cache = SurfaceSiteCache::new();
        {
            let mut entries = cache
                .write()
                .expect("surface-site cache write lock should not be poisoned");
            entries.insert(IVec2::ZERO, entry(0));
            entries.insert(IVec2::new(512, 0), entry(1));
        }

        cache.retain_around(IVec2::ZERO, 4, Vec2::splat(128.0));

        let entries = cache
            .read()
            .expect("surface-site cache read lock should not be poisoned");
        assert!(entries.contains_key(&IVec2::ZERO));
        assert!(!entries.contains_key(&IVec2::new(512, 0)));
    }
}
