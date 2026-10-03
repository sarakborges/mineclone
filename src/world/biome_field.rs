mod constants;
mod selection;
mod spatial;
mod surface;
mod surface_field;
mod visuals;
mod volume;

use std::sync::Arc;

use arrayvec::ArrayVec;
use bevy::prelude::*;

use crate::content::{
    biome::{
        BiomeClimate, BiomeKind, BiomeRegistry, BiomeVerticalRange, VolumeSurfaceConstraints,
    },
    biome_density::BiomeDensityModifier,
    biome_terrain::BiomeTerrain,
    biome_terrain_modifier::BiomeTerrainModifier,
    dimension::{DimensionBiomeSize, DimensionBiomeSizeAxis, DimensionDefinition},
};

pub(crate) use self::volume::{VolumeBiomeRegion, VolumeBiomeSelection};
use self::{
    constants::{SITE_SEARCH_RADIUS, VOLUME_SITE_GAP},
    spatial::{surface_map_spacing, surface_site_position},
    surface_field::SurfaceFieldConfig,
};
use super::{macro_climate::MacroClimateField, new_world::biome_size_multiplier_tenths};

const SURFACE_SITE_SEARCH_DIAMETER: usize = (SITE_SEARCH_RADIUS * 2 + 1) as usize;
pub(crate) const MAX_SURFACE_INFLUENCES: usize =
    SURFACE_SITE_SEARCH_DIAMETER * SURFACE_SITE_SEARCH_DIAMETER + 2;

#[derive(Clone)]
pub(super) struct BiomeFieldEntry {
    pub id: String,
    pub tags: Vec<String>,
    pub surface_constraints: Option<VolumeSurfaceConstraints>,
    pub size: DimensionBiomeSize,
    pub weight: f32,
    pub climate: BiomeClimate,
    pub vertical_range: Option<BiomeVerticalRange>,
    pub priority: i32,
    pub terrain: Option<BiomeTerrain>,
    pub terrain_modifiers: Vec<BiomeTerrainModifier>,
    pub density_modifier: Option<BiomeDensityModifier>,
    pub solid_block: Option<String>,
    pub density_seed: u64,
    pub surface_margin: Option<SurfaceMarginField>,
}

#[derive(Clone, Copy)]
pub(super) struct SurfaceMarginField {
    pub width: f32,
    pub width_variation: f32,
    pub variation_scale: f32,
    pub noise_seed: u64,
}

#[derive(Resource, Clone)]
pub struct BiomeField {
    pub(super) surface_biomes: Arc<Vec<BiomeFieldEntry>>,
    pub(super) volume_biomes: Arc<Vec<BiomeFieldEntry>>,
    pub(super) surface_site_spacing: Vec2,
    pub(super) surface_field_config: SurfaceFieldConfig,
    pub(super) volume_site_spacing: Option<Vec3>,
    pub(super) climate: MacroClimateField,
    pub(super) seed: u64,
    pub(super) single_surface_biome: Option<usize>,
    pub(super) ocean_surface_index: Option<usize>,
    pub(super) spawn_oceans: bool,
    spawn_target_surface_biome: Option<usize>,
}

#[derive(Clone, Copy)]
pub struct BiomeInfluence<'a> {
    pub id: &'a str,
    pub weight: f32,
    pub(crate) surface_index: usize,
    pub(crate) terrain_strength: f32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SurfaceBoundarySample {
    pub(crate) neighbor_surface_index: usize,
    pub(crate) distance: f32,
}

pub struct BiomeFieldSample<'a> {
    pub primary_id: &'a str,
    pub(crate) primary_surface_index: usize,
    pub(crate) surface_margin_index: Option<usize>,
    pub(crate) identity_surface_index: usize,
    pub influences: ArrayVec<BiomeInfluence<'a>, MAX_SURFACE_INFLUENCES>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct VolumeBiomeAnchor<'a> {
    pub id: &'a str,
    pub position: Vec3,
}

impl BiomeField {
    pub fn from_dimension(
        dimension: &DimensionDefinition,
        biomes: &BiomeRegistry,
        seed: u64,
        biome_size_multiplier: f32,
    ) -> Self {
        assert!(
            !dimension.biomes.is_empty(),
            "dimension {} must define at least one biome",
            dimension.id
        );
        let multiplier_tenths = biome_size_multiplier_tenths(biome_size_multiplier)
            .unwrap_or_else(|| {
                panic!(
                    "biome size multiplier must be between 0.5 and 5.0 in 0.1 increments: {biome_size_multiplier}"
                )
            });

        let mut surface_biomes = Vec::new();
        let mut volume_biomes = Vec::new();
        let mut volume_minimum_radius = Vec3::ZERO;
        let mut has_active_volume_biome = false;

        for dimension_biome in &dimension.biomes {
            let biome_id = &dimension_biome.id;
            let biome = biomes
                .get(biome_id)
                .unwrap_or_else(|| panic!("missing biome definition: {biome_id}"));

            let size = dimension_biome.size.unwrap_or_else(|| {
                panic!(
                    "dimension {} biome {} must define size",
                    dimension.id, biome.id
                )
            });
            let size = scaled_biome_size(size, multiplier_tenths);
            let entry = BiomeFieldEntry {
                id: biome.id.clone(),
                tags: biome.tags.clone(),
                surface_constraints: biome.surface_constraints.clone(),
                size,
                weight: dimension_biome.weight,
                climate: biome.climate,
                vertical_range: biome.vertical_range,
                priority: biome.priority,
                terrain: biome.terrain,
                terrain_modifiers: biome.terrain_modifiers.clone(),
                density_modifier: biome.density_modifier,
                solid_block: biome.solid_block.clone(),
                density_seed: biome_density_seed(seed, &biome.id),
                surface_margin: biome.surface_margin.as_ref().map(|margin| SurfaceMarginField {
                    width: margin.width,
                    width_variation: margin.width_variation,
                    variation_scale: margin.variation_scale,
                    noise_seed: biome_density_seed(
                        seed ^ 0x9e37_79b9_7f4a_7c15,
                        &biome.id,
                    ),
                }),
            };

            match biome.kind {
                BiomeKind::Surface => surface_biomes.push(entry),
                BiomeKind::Volume => {
                    if entry.weight > 0.0 {
                        let vertical_size = entry.size.y.unwrap_or_else(|| {
                            panic!("volume biome {} must define dimension size.y", biome.id)
                        });
                        volume_minimum_radius.x = volume_minimum_radius.x.max(entry.size.x.min);
                        volume_minimum_radius.y = volume_minimum_radius.y.max(vertical_size.min);
                        volume_minimum_radius.z = volume_minimum_radius.z.max(entry.size.z.min);
                        has_active_volume_biome = true;
                    }
                    volume_biomes.push(entry);
                }
            }
        }

        assert!(
            surface_biomes.iter().any(|biome| biome.weight > 0.0),
            "dimension {} must define at least one active surface biome",
            dimension.id
        );

        let surface_site_spacing = surface_map_spacing();
        let volume_site_spacing = has_active_volume_biome
            .then_some(volume_minimum_radius * 2.0 + Vec3::splat(VOLUME_SITE_GAP));
        let ocean_surface_index = dimension
            .ocean_biome
            .as_deref()
            .and_then(|id| surface_biomes.iter().position(|biome| biome.id == id));
        let surface_field_config =
            SurfaceFieldConfig::from_biomes(&surface_biomes, ocean_surface_index);

        Self {
            surface_biomes: Arc::new(surface_biomes),
            volume_biomes: Arc::new(volume_biomes),
            surface_site_spacing,
            surface_field_config,
            volume_site_spacing,
            climate: MacroClimateField::new(seed),
            seed,
            single_surface_biome: None,
            ocean_surface_index,
            spawn_oceans: true,
            spawn_target_surface_biome: None,
        }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub(crate) fn set_spawn_oceans(&mut self, spawn_oceans: bool) {
        self.spawn_oceans = spawn_oceans;
    }

    pub(super) fn surface_biome_is_enabled(&self, index: usize) -> bool {
        self.spawn_oceans || Some(index) != self.ocean_surface_index
    }

    pub(crate) fn set_single_surface_biome(&mut self, biome_id: &str) {
        let biome_index = self
            .surface_biomes
            .iter()
            .position(|biome| biome.id == biome_id)
            .unwrap_or_else(|| panic!("single biome is not a surface biome: {biome_id}"));
        self.single_surface_biome = Some(biome_index);
        self.spawn_target_surface_biome = None;
    }

    pub(crate) fn set_spawn_target_surface_biome(&mut self, biome_id: &str) {
        let biome_index = self
            .surface_biomes
            .iter()
            .position(|biome| biome.id == biome_id)
            .unwrap_or_else(|| panic!("spawn target is not a surface biome: {biome_id}"));
        self.spawn_target_surface_biome = Some(biome_index);
    }

    pub(crate) fn spawn_target_contains(&self, position: Vec2) -> bool {
        self.spawn_target_surface_biome
            .is_some_and(|target| self.surface_biome_index_at(position) == target)
    }

    pub(crate) fn surface_biome_at(&self, position: Vec2) -> &str {
        self.surface_biome_id(self.surface_biome_index_at(position))
    }

    pub(crate) fn ocean_surface_index(&self) -> Option<usize> {
        self.ocean_surface_index
    }

    pub(crate) fn surface_site_spacing(&self) -> Vec2 {
        self.surface_site_spacing
    }

    pub(crate) fn surface_site_position(&self, cell: IVec2) -> Vec2 {
        surface_site_position(cell, self.surface_site_spacing, self.seed)
    }

    pub(crate) fn surface_biome_id(&self, index: usize) -> &str {
        self.surface_biomes
            .get(index)
            .unwrap_or_else(|| panic!("surface biome index out of bounds: {index}"))
            .id
            .as_str()
    }

    pub(crate) fn surface_biome_has_tag(&self, index: usize, tag: &str) -> bool {
        self.surface_biomes
            .get(index)
            .unwrap_or_else(|| panic!("surface biome index out of bounds: {index}"))
            .tags
            .iter()
            .any(|candidate| candidate == tag)
    }

    pub(crate) fn surface_terrain(
        &self,
        index: usize,
    ) -> (BiomeTerrain, &[BiomeTerrainModifier], u64) {
        let biome = self
            .surface_biomes
            .get(index)
            .unwrap_or_else(|| panic!("surface biome index out of bounds: {index}"));
        let terrain = biome
            .terrain
            .unwrap_or_else(|| panic!("surface biome {} must define terrain", biome.id));

        (terrain, &biome.terrain_modifiers, biome.density_seed)
    }

    pub(crate) fn volume_biome_id(&self, selection: VolumeBiomeSelection) -> &str {
        self.volume_biomes
            .get(selection.biome_index)
            .unwrap_or_else(|| {
                panic!(
                    "volume biome index out of bounds: {}",
                    selection.biome_index
                )
            })
            .id
            .as_str()
    }
}

fn scaled_biome_size(size: DimensionBiomeSize, multiplier_tenths: u8) -> DimensionBiomeSize {
    DimensionBiomeSize {
        x: scaled_biome_size_axis(size.x, multiplier_tenths),
        z: scaled_biome_size_axis(size.z, multiplier_tenths),
        y: size
            .y
            .map(|axis| scaled_biome_size_axis(axis, multiplier_tenths)),
    }
}

fn scaled_biome_size_axis(
    size: DimensionBiomeSizeAxis,
    multiplier_tenths: u8,
) -> DimensionBiomeSizeAxis {
    let scale = |value: f32| {
        (value * f32::from(multiplier_tenths) / 10.0)
            .round()
            .max(1.0)
    };
    let min = scale(size.min);
    let max = scale(size.max).max(min);
    DimensionBiomeSizeAxis { min, max }
}

fn biome_density_seed(seed: u64, biome_id: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;

    for byte in biome_id.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    let mut mixed = seed ^ hash;
    mixed ^= mixed >> 33;
    mixed = mixed.wrapping_mul(0xff51_afd7_ed55_8ccd);
    mixed ^= mixed >> 33;
    mixed = mixed.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    mixed ^= mixed >> 33;
    mixed
}

#[cfg(test)]
mod biome_size_multiplier_tests {
    use super::*;

    #[test]
    fn biome_size_multiplier_scales_and_rounds_every_axis() {
        let scaled = scaled_biome_size(
            DimensionBiomeSize {
                x: DimensionBiomeSizeAxis {
                    min: 75.0,
                    max: 125.0,
                },
                z: DimensionBiomeSizeAxis {
                    min: 41.0,
                    max: 99.0,
                },
                y: Some(DimensionBiomeSizeAxis {
                    min: 15.0,
                    max: 35.0,
                }),
            },
            5,
        );

        assert_eq!(scaled.x.min, 38.0);
        assert_eq!(scaled.x.max, 63.0);
        assert_eq!(scaled.z.min, 21.0);
        assert_eq!(scaled.z.max, 50.0);
        let y = scaled
            .y
            .expect("scaled vertical size should remain defined");
        assert_eq!(y.min, 8.0);
        assert_eq!(y.max, 18.0);
    }
}
