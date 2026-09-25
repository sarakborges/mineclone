mod cache;

use std::sync::Arc;

#[cfg(test)]
use std::collections::HashSet;

use bevy::prelude::*;

use crate::content::structure::StructureRotation;

#[derive(Clone, Debug)]
pub(crate) struct CachedStructureCandidate {
    pub(crate) placement_id: String,
    pub(crate) structure_id: String,
    pub(crate) rotation: StructureRotation,
    pub(crate) placement_anchor: IVec2,
    pub(crate) placement_y: i32,
    pub(crate) anchor: IVec2,
    pub(crate) origin_y: i32,
    pub(crate) primary_placement_piece: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct CachedSurfaceStructurePiece {
    pub(crate) structure_id: String,
    pub(crate) rotation: StructureRotation,
    pub(crate) anchor: IVec2,
    pub(crate) origin_y: i32,
    pub(crate) primary_placement_piece: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct CachedSurfaceStructurePlacement {
    pub(crate) pieces: Vec<CachedSurfaceStructurePiece>,
    pub(crate) minimum: IVec2,
    pub(crate) maximum: IVec2,
    pub(crate) minimum_y: i32,
    pub(crate) maximum_y: i32,
}

use self::cache::FeatureCaches;
use super::{
    biome_field::VolumeBiomeRegion,
    generation::GenerationColumnSample,
    structure_field::StructureField,
};

#[derive(Resource, Clone)]
pub(crate) struct WorldFeatureFields {
    caches: Arc<FeatureCaches>,
    structure_field: Arc<StructureField>,
}

impl WorldFeatureFields {
    pub(crate) fn new(seed: u64) -> Self {
        Self {
            caches: Arc::new(FeatureCaches::new()),
            structure_field: Arc::new(StructureField::empty(seed)),
        }
    }

    pub(crate) fn with_structure_field(mut self, structure_field: StructureField) -> Self {
        self.structure_field = Arc::new(structure_field);
        self
    }

    pub(crate) fn clone_with_fresh_caches(&self) -> Self {
        Self {
            caches: Arc::new(FeatureCaches::new()),
            structure_field: self.structure_field.clone(),
        }
    }

    pub(crate) fn structure_field(&self) -> &StructureField {
        &self.structure_field
    }

    pub(crate) fn generation_columns(
        &self,
        coord: IVec2,
        factory: impl FnOnce() -> Vec<GenerationColumnSample>,
    ) -> Arc<Vec<GenerationColumnSample>> {
        self.caches.generation_columns(coord, factory)
    }

    #[cfg(test)]
    pub(crate) fn generation_columns_initialized(&self, coord: IVec2) -> bool {
        self.caches.generation_columns_initialized(coord)
    }

    pub(crate) fn volume_biome_region(
        &self,
        coord: IVec3,
        factory: impl FnOnce() -> VolumeBiomeRegion,
    ) -> Arc<VolumeBiomeRegion> {
        self.caches.volume_biome_region(coord, factory)
    }

    pub(crate) fn structure_top_y(
        &self,
        coord: IVec2,
        factory: impl FnOnce() -> i32,
    ) -> i32 {
        self.caches.structure_top_y(coord, factory)
    }

    pub(crate) fn structure_top_y_if_ready(&self, coord: IVec2) -> Option<i32> {
        self.caches.structure_top_y_if_ready(coord)
    }

    pub(crate) fn structure_placement_bounds(
        &self,
        reference: &str,
        factory: impl FnOnce() -> Option<(IVec2, IVec2)>,
    ) -> Option<(IVec2, IVec2)> {
        self.caches.structure_placement_bounds(reference, factory)
    }

    pub(crate) fn structure_origin_y(
        &self,
        structure_id: &str,
        rotation: StructureRotation,
        anchor: IVec2,
        factory: impl FnOnce() -> Option<i32>,
    ) -> Option<i32> {
        self.caches
            .structure_origin_y(structure_id, rotation, anchor, factory)
    }

    pub(crate) fn structure_candidates(
        &self,
        coord: IVec2,
        factory: impl FnOnce() -> Vec<CachedStructureCandidate>,
    ) -> Arc<Vec<CachedStructureCandidate>> {
        self.caches.structure_candidates(coord, factory)
    }

    pub(crate) fn surface_structure_placement(
        &self,
        biome_id: &str,
        placement_id: &str,
        anchor: IVec2,
        factory: impl FnOnce() -> Option<CachedSurfaceStructurePlacement>,
    ) -> Arc<Option<CachedSurfaceStructurePlacement>> {
        self.caches
            .surface_structure_placement(biome_id, placement_id, anchor, factory)
    }

    pub(crate) fn retain_for_chunks<'a>(
        &self,
        desired: impl IntoIterator<Item = &'a IVec3>,
    ) {
        self.caches.retain_for_chunks(desired);
    }

    #[cfg(test)]
    fn cached_generation_column_count(&self) -> usize {
        self.caches.generation_column_count()
    }

    #[cfg(test)]
    fn cached_volume_biome_region_count(&self) -> usize {
        self.caches.volume_biome_region_count()
    }

    #[cfg(test)]
    fn cached_structure_origin_count(&self) -> usize {
        self.caches.structure_origin_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        voxel::chunk::CHUNK_SIZE,
        world::generation_region::generation_region_coord,
    };

    fn test_fields() -> WorldFeatureFields {
        WorldFeatureFields::new(42)
    }

    #[test]
    fn generation_column_cache_reuses_horizontal_chunk_samples() {
        let fields = test_fields();
        let coord = IVec2::new(3, -2);
        let first = fields.generation_columns(coord, Vec::new);
        let second = fields.generation_columns(coord, || {
            panic!("cached generation columns should not rebuild")
        });

        assert!(Arc::ptr_eq(&first, &second));
        assert!(fields.generation_columns_initialized(coord));
        assert_eq!(fields.cached_generation_column_count(), 1);
    }

    #[test]
    fn volume_biome_cache_reuses_the_same_generation_region_result() {
        let fields = test_fields();
        let coord = IVec3::new(1, 2, 3);
        let first = fields.volume_biome_region(coord, VolumeBiomeRegion::default);
        let second = fields.volume_biome_region(coord, || {
            panic!("cached volume biome region should not rebuild")
        });

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(fields.cached_volume_biome_region_count(), 1);
    }

    #[test]
    fn structure_origin_cache_reuses_accepted_and_rejected_placements() {
        let fields = test_fields();
        let accepted_anchor = IVec2::new(8, 12);
        let rejected_anchor = IVec2::new(24, -4);

        assert_eq!(
            fields.structure_origin_y("asteria:test/tree", StructureRotation::Degrees0, accepted_anchor, || Some(65)),
            Some(65),
        );
        assert_eq!(
            fields.structure_origin_y("asteria:test/tree", StructureRotation::Degrees0, accepted_anchor, || {
                panic!("accepted structure origin should be cached")
            }),
            Some(65),
        );
        assert_eq!(
            fields.structure_origin_y("asteria:test/tree", StructureRotation::Degrees0, rejected_anchor, || None),
            None,
        );
        assert_eq!(
            fields.structure_origin_y("asteria:test/tree", StructureRotation::Degrees0, rejected_anchor, || {
                panic!("rejected structure origin should be cached")
            }),
            None,
        );
        assert_eq!(fields.cached_structure_origin_count(), 2);
    }

    #[test]
    fn cache_retention_drops_entries_outside_the_streaming_window() {
        let fields = test_fields();
        let near_chunk = IVec3::ZERO;
        let far_chunk = IVec3::new(32, 0, 0);
        let near_region = generation_region_coord(near_chunk);
        let far_region = generation_region_coord(far_chunk);

        fields.generation_columns(near_chunk.xz(), Vec::new);
        fields.generation_columns(far_chunk.xz(), Vec::new);
        fields.volume_biome_region(near_region, VolumeBiomeRegion::default);
        fields.volume_biome_region(far_region, VolumeBiomeRegion::default);
        fields.structure_origin_y("test", StructureRotation::Degrees0, IVec2::ZERO, || Some(64));
        fields.structure_origin_y("test", StructureRotation::Degrees0, IVec2::new(32 * CHUNK_SIZE as i32, 0), || Some(64));

        let desired = HashSet::from([near_chunk]);
        fields.retain_for_chunks(&desired);

        assert_eq!(fields.cached_generation_column_count(), 1);
        assert_eq!(fields.cached_volume_biome_region_count(), 1);
        assert_eq!(fields.cached_structure_origin_count(), 1);
    }
}
