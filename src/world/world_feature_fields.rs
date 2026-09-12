mod cache;

use std::{collections::HashSet, sync::Arc};

use bevy::prelude::*;

use crate::content::dimension_hydrology::DimensionHydrology;

use self::cache::FeatureCaches;
use super::{
    biome_field::VolumeBiomeRegion,
    cave_connectivity::{CaveConnectivityField, CaveConnectivityRegion},
    generation::GenerationColumnSample,
    generation_region::GenerationRegion,
    hydrology::{HydrologyBiomeOverlay, HydrologyField, HydrologyRegion},
};

#[derive(Resource)]
pub(crate) struct WorldFeatureFields {
    hydrology: HydrologyField,
    cave_connectivity: CaveConnectivityField,
    caches: FeatureCaches,
}

impl WorldFeatureFields {
    pub(crate) fn new(seed: u64, sea_level: i32, hydrology: DimensionHydrology) -> Self {
        Self {
            hydrology: HydrologyField::new(seed.rotate_left(7), sea_level, hydrology),
            cave_connectivity: CaveConnectivityField::new(seed.rotate_left(23)),
            caches: FeatureCaches::new(),
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
        self.caches.generation_columns(coord, factory)
    }

    pub(crate) fn volume_biome_region(
        &self,
        coord: IVec3,
        factory: impl FnOnce() -> VolumeBiomeRegion,
    ) -> Arc<VolumeBiomeRegion> {
        self.caches.volume_biome_region(coord, factory)
    }

    pub(crate) fn cave_region(
        &self,
        coord: IVec3,
        factory: impl FnOnce(&CaveConnectivityField) -> Option<CaveConnectivityRegion>,
    ) -> Option<Arc<CaveConnectivityRegion>> {
        self.caches
            .cave_region(coord, || factory(&self.cave_connectivity))
    }

    pub(crate) fn structure_origin_y(
        &self,
        structure_id: &str,
        anchor: IVec2,
        factory: impl FnOnce() -> Option<i32>,
    ) -> Option<i32> {
        self.caches
            .structure_origin_y(structure_id, anchor, factory)
    }

    pub(crate) fn region_with_hydrology(
        &self,
        coord: IVec3,
        hydrology_factory: impl FnOnce(&HydrologyField) -> HydrologyRegion,
    ) -> Arc<GenerationRegion> {
        self.caches.generation_region(coord, || {
            let hydrology_coord = coord.xz();
            let hydrology = self.caches.hydrology_region(hydrology_coord, || {
                hydrology_factory(&self.hydrology)
            });

            GenerationRegion { coord, hydrology }
        })
    }

    pub(crate) fn retain_for_chunks(&self, desired: &HashSet<IVec3>) {
        self.caches.retain_for_chunks(desired);
    }

    #[cfg(test)]
    fn cached_region_count(&self) -> usize {
        self.caches.region_count()
    }

    #[cfg(test)]
    fn cached_hydrology_region_count(&self) -> usize {
        self.caches.hydrology_region_count()
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
    fn cached_cave_region_count(&self) -> usize {
        self.caches.cave_region_count()
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

    #[test]
    fn structure_origin_cache_reuses_accepted_and_rejected_placements() {
        let fields = WorldFeatureFields::new(42, 64, DimensionHydrology::default());
        let accepted_anchor = IVec2::new(8, 12);
        let rejected_anchor = IVec2::new(24, -4);

        assert_eq!(
            fields.structure_origin_y("asteria:test/tree", accepted_anchor, || Some(65)),
            Some(65),
        );
        assert_eq!(
            fields.structure_origin_y("asteria:test/tree", accepted_anchor, || {
                panic!("accepted structure origin should be cached")
            }),
            Some(65),
        );
        assert_eq!(
            fields.structure_origin_y("asteria:test/tree", rejected_anchor, || None),
            None,
        );
        assert_eq!(
            fields.structure_origin_y("asteria:test/tree", rejected_anchor, || {
                panic!("rejected structure origin should be cached")
            }),
            None,
        );
        assert_eq!(fields.cached_structure_origin_count(), 2);
    }

    #[test]
    fn cache_retention_drops_entries_outside_the_streaming_window() {
        let fields = WorldFeatureFields::new(42, 64, DimensionHydrology::default());
        let near_chunk = IVec3::ZERO;
        let far_chunk = IVec3::new(32, 0, 0);
        let near_region = generation_region_coord(near_chunk);
        let far_region = generation_region_coord(far_chunk);

        fields.generation_columns(near_chunk.xz(), Vec::new);
        fields.generation_columns(far_chunk.xz(), Vec::new);
        fields.volume_biome_region(near_region, VolumeBiomeRegion::default);
        fields.volume_biome_region(far_region, VolumeBiomeRegion::default);
        fields.cave_region(near_region, |_| Some(CaveConnectivityRegion::default()));
        fields.cave_region(far_region, |_| Some(CaveConnectivityRegion::default()));
        fields.structure_origin_y("asteria:test/tree", IVec2::ZERO, || Some(64));
        fields.structure_origin_y(
            "asteria:test/tree",
            IVec2::new(far_chunk.x * CHUNK_SIZE as i32, 0),
            || Some(64),
        );
        for coord in [near_region, far_region] {
            fields.region_with_hydrology(coord, |hydrology| {
                hydrology.region_from_macro_terrain(coord.xz(), |_| {
                    super::super::hydrology::HydrologySurfaceSample {
                        elevation: 64.0,
                        continentalness: 0.5,
                        biome_hydrology: crate::content::biome_hydrology::BiomeHydrology::default(),
                    }
                })
            });
        }

        fields.retain_for_chunks(&HashSet::from([near_chunk]));

        assert_eq!(fields.cached_generation_column_count(), 1);
        assert_eq!(fields.cached_volume_biome_region_count(), 1);
        assert_eq!(fields.cached_cave_region_count(), 1);
        assert_eq!(fields.cached_structure_origin_count(), 1);
        assert_eq!(fields.cached_region_count(), 1);
        assert_eq!(fields.cached_hydrology_region_count(), 1);
    }
}
