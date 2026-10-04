use std::sync::{Arc, OnceLock, RwLock};

use arrayvec::ArrayVec;
use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use smallvec::SmallVec;

use crate::voxel::chunk::CHUNK_SIZE;

use super::super::biome_field::{
    BiomeField, BiomeFieldSample, BiomeInfluence, MAX_SURFACE_INFLUENCES,
};

pub(super) const BIOME_MAP_HALO: i32 = 1;
const BIOME_MAP_EDGE: usize = CHUNK_SIZE + BIOME_MAP_HALO as usize * 2;
const BIOME_MAP_SAMPLE_COUNT: usize = BIOME_MAP_EDGE * BIOME_MAP_EDGE;

#[derive(Clone, Copy, Debug)]
pub(crate) struct BiomeMapInfluence {
    pub(crate) surface_index: usize,
    pub(crate) weight: f32,
    pub(crate) terrain_strength: f32,
}

#[derive(Clone, Debug)]
pub(crate) struct BiomeMapSample {
    pub(crate) primary_surface_index: usize,
    pub(crate) surface_margin_index: Option<usize>,
    pub(crate) influences: SmallVec<[BiomeMapInfluence; 4]>,
}

impl BiomeMapSample {
    fn from_field_sample(surface: &BiomeFieldSample<'_>, biome_field: &BiomeField) -> Self {
        let primary = surface.primary_surface_index;
        let incompatible_neighbor = surface
            .influences
            .iter()
            .map(|influence| influence.surface_index)
            .chain(surface.surface_margin_index)
            .find(|&neighbor| {
                neighbor != primary && !biome_field.surface_biomes_can_neighbor(primary, neighbor)
            });

        if let Some(neighbor) = incompatible_neighbor {
            let separator = biome_field
                .surface_boundary_separator(primary, neighbor)
                .unwrap_or_else(|| {
                    panic!(
                        "surface biomes {} and {} deny adjacency but no compatible separator biome exists",
                        biome_field.surface_biome_id(primary),
                        biome_field.surface_biome_id(neighbor),
                    )
                });
            let mut influences = SmallVec::new();
            influences.push(BiomeMapInfluence {
                surface_index: separator,
                weight: 1.0,
                terrain_strength: 1.0,
            });
            return Self {
                primary_surface_index: separator,
                surface_margin_index: None,
                influences,
            };
        }

        Self {
            primary_surface_index: primary,
            surface_margin_index: surface.surface_margin_index,
            influences: surface
                .influences
                .iter()
                .map(|influence| BiomeMapInfluence {
                    surface_index: influence.surface_index,
                    weight: influence.weight,
                    terrain_strength: influence.terrain_strength,
                })
                .collect(),
        }
    }

    pub(crate) fn primary_terrain_strength(&self) -> f32 {
        self.influences
            .iter()
            .find(|influence| influence.surface_index == self.primary_surface_index)
            .map_or(1.0, |influence| influence.terrain_strength)
    }

    pub(crate) fn as_field_sample<'a>(&self, biome_field: &'a BiomeField) -> BiomeFieldSample<'a> {
        let identity_surface_index = self
            .surface_margin_index
            .unwrap_or(self.primary_surface_index);
        let mut influences = ArrayVec::<BiomeInfluence<'a>, MAX_SURFACE_INFLUENCES>::new();
        for influence in &self.influences {
            influences.push(BiomeInfluence {
                id: biome_field.surface_biome_id(influence.surface_index),
                weight: influence.weight,
                surface_index: influence.surface_index,
                terrain_strength: influence.terrain_strength,
            });
        }

        BiomeFieldSample {
            primary_id: biome_field.surface_biome_id(identity_surface_index),
            primary_surface_index: self.primary_surface_index,
            surface_margin_index: self.surface_margin_index,
            identity_surface_index,
            influences,
        }
    }
}

/// Immutable authoritative surface-biome map for one horizontal chunk plus a
/// one-block halo.
///
/// `BiomeField` only provides the raw deterministic surface selection. Authored
/// adjacency constraints are normalized here, once, before terrain and feature
/// generation consume the map. Ocean therefore participates as an ordinary
/// surface biome instead of having a separate adjacency path.
pub(crate) struct BiomeMapTile {
    samples: Vec<BiomeMapSample>,
}

impl BiomeMapTile {
    pub(crate) fn sample(horizontal_chunk: IVec2, biome_field: &BiomeField) -> Self {
        let chunk_origin = horizontal_chunk * CHUNK_SIZE as i32;
        let mut samples = Vec::with_capacity(BIOME_MAP_SAMPLE_COUNT);

        for local_z in -BIOME_MAP_HALO..CHUNK_SIZE as i32 + BIOME_MAP_HALO {
            for local_x in -BIOME_MAP_HALO..CHUNK_SIZE as i32 + BIOME_MAP_HALO {
                let world_position = chunk_origin + IVec2::new(local_x, local_z);
                let surface =
                    biome_field.sample_surface(world_position.as_vec2() + Vec2::splat(0.5));
                samples.push(BiomeMapSample::from_field_sample(&surface, biome_field));
            }
        }

        debug_assert_eq!(samples.len(), BIOME_MAP_SAMPLE_COUNT);
        Self { samples }
    }

    pub(crate) fn sample_at(&self, local: IVec2) -> &BiomeMapSample {
        &self.samples[Self::sample_index(local)]
    }

    pub(crate) fn sample_index(local: IVec2) -> usize {
        debug_assert!(
            local.x >= -BIOME_MAP_HALO
                && local.y >= -BIOME_MAP_HALO
                && local.x < CHUNK_SIZE as i32 + BIOME_MAP_HALO
                && local.y < CHUNK_SIZE as i32 + BIOME_MAP_HALO,
            "biome map local sample outside halo: {local:?}"
        );
        let x = (local.x + BIOME_MAP_HALO) as usize;
        let z = (local.y + BIOME_MAP_HALO) as usize;
        z * BIOME_MAP_EDGE + x
    }
}

/// Disposable cache for the first stage of world generation. It is kept
/// separate from feature caches so biome-map ownership stays explicit.
pub(crate) struct BiomeMapCache {
    entries: RwLock<HashMap<IVec2, Arc<OnceLock<Arc<BiomeMapTile>>>>>,
}

impl BiomeMapCache {
    pub(crate) fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    pub(crate) fn get_or_insert_with(
        &self,
        coord: IVec2,
        factory: impl FnOnce() -> BiomeMapTile,
    ) -> Arc<BiomeMapTile> {
        let cached = self
            .entries
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&coord)
            .cloned();
        let entry = cached.unwrap_or_else(|| {
            let mut entries = self
                .entries
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            entries
                .entry(coord)
                .or_insert_with(|| Arc::new(OnceLock::new()))
                .clone()
        });

        entry.get_or_init(|| Arc::new(factory())).clone()
    }

    pub(crate) fn retain_for_chunks<'a>(&self, desired: impl IntoIterator<Item = &'a IVec3>) {
        let horizontal = desired
            .into_iter()
            .map(|coord| coord.xz())
            .collect::<HashSet<_>>();
        self.entries
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|coord, _| horizontal.contains(coord));
    }
}
