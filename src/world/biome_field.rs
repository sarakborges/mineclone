mod constants;
pub(crate) mod distribution;
mod mountain_belt;
mod mountain_peak;
mod noise_band;
mod selection;
mod spatial;
mod surface;
mod visuals;
mod volume;

use std::sync::{Arc, RwLock};

use arrayvec::ArrayVec;
use bevy::{platform::collections::HashMap, prelude::*};

use crate::content::{
    biome::{BiomeClimate, BiomeKind, BiomeRegistry, BiomeVerticalRange},
    biome_density::BiomeDensityModifier, biome_distribution::BiomeDistribution,
    biome_hydrology::BiomeHydrologyRules, biome_terrain::BiomeTerrain,
    biome_terrain_modifier::BiomeTerrainModifier,
    dimension::{DimensionBiomeSize, DimensionBiomeSizeAxis, DimensionDefinition},
};

pub(crate) use self::volume::{VolumeBiomeRegion, VolumeBiomeSelection};
use self::{
    constants::{BORDER_TRANSITION_WIDTH, SITE_SEARCH_RADIUS, VOLUME_SITE_GAP},
    spatial::{hash_unit, lerp, smoothstep, surface_minimum_spacing, warp_surface_position},
};
use super::{
    hydrology::suppress_ocean_continentalness,
    macro_climate::{MacroClimateField, MacroClimateSample},
    new_world::biome_size_multiplier_tenths,
};

const SURFACE_SITE_SEARCH_DIAMETER: usize = (SITE_SEARCH_RADIUS * 2 + 1) as usize;
pub(crate) const MAX_SURFACE_INFLUENCES: usize =
    SURFACE_SITE_SEARCH_DIAMETER * SURFACE_SITE_SEARCH_DIAMETER + 2;

#[derive(Clone)]
pub(super) struct BiomeFieldEntry {
    pub id: String,
    pub distributions: Vec<BiomeDistribution>,
    pub size: DimensionBiomeSize,
    pub weight: f32,
    pub climate: BiomeClimate,
    pub vertical_range: Option<BiomeVerticalRange>,
    pub priority: i32,
    pub terrain: Option<BiomeTerrain>,
    pub terrain_modifiers: Vec<BiomeTerrainModifier>,
    pub hydrology: BiomeHydrologyRules,
    pub density_modifier: Option<BiomeDensityModifier>,
    pub solid_block: Option<String>,
    pub density_seed: u64,
    pub avoid_near: Vec<String>,
    pub require_near: Vec<String>,
}

#[derive(Resource, Clone)]
pub struct BiomeField {
    pub(super) surface_biomes: Vec<BiomeFieldEntry>,
    pub(super) volume_biomes: Vec<BiomeFieldEntry>,
    pub(super) surface_site_spacing: Vec2,
    pub(super) volume_site_spacing: Option<Vec3>,
    pub(super) climate: MacroClimateField,
    pub(super) seed: u64,
    pub(super) surface_site_biomes: Arc<RwLock<HashMap<IVec2, usize>>>,
    forced_surface_biome: Option<ForcedSurfaceBiome>,
    ocean_surface_index: Option<usize>,
    pub(super) ocean_weight: f32,
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
    pub(crate) nearest_boundary: Option<SurfaceBoundarySample>,
    pub influences: ArrayVec<BiomeInfluence<'a>, MAX_SURFACE_INFLUENCES>,
}

#[derive(Clone, Copy, Debug)]
struct ForcedSurfaceBiome {
    biome_index: usize,
    center: Vec2,
    radii: Vec2,
    warp_seed: u64,
}

impl ForcedSurfaceBiome {
    fn warped_delta(self, position: Vec2) -> Vec2 {
        let warped = warp_surface_position(position, self.warp_seed);
        let warped_center = warp_surface_position(self.center, self.warp_seed);
        warped - warped_center
    }

    fn normalized_distance(self, position: Vec2) -> f32 {
        let delta = self.warped_delta(position);
        Vec2::new(delta.x / self.radii.x, delta.y / self.radii.y).length()
    }

    fn core_contains(self, position: Vec2) -> bool {
        self.normalized_distance(position) <= 1.0
    }

    fn weight(self, position: Vec2) -> f32 {
        let delta = self.warped_delta(position);
        let normalized = Vec2::new(delta.x / self.radii.x, delta.y / self.radii.y).length();
        if normalized <= 1.0 {
            return 1.0;
        }

        let radial_distance = delta.length();
        let boundary_distance = radial_distance / normalized;
        let outside_distance = (radial_distance - boundary_distance).max(0.0);
        if outside_distance >= BORDER_TRANSITION_WIDTH {
            return 0.0;
        }

        smoothstep(1.0 - outside_distance / BORDER_TRANSITION_WIDTH)
    }
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
        let mut surface_minimum_radius = Vec2::ZERO;
        let mut volume_minimum_radius = Vec3::ZERO;
        let mut has_active_volume_biome = false;

        for dimension_biome in &dimension.biomes {
            let biome_id = &dimension_biome.id;
            let biome = biomes
                .get(biome_id)
                .unwrap_or_else(|| panic!("missing biome definition: {biome_id}"));

            if biome.kind == BiomeKind::Hydrology {
                continue;
            }

            let size = dimension_biome.size.unwrap_or_else(|| {
                panic!(
                    "dimension {} biome {} must define size",
                    dimension.id, biome.id
                )
            });
            let size = scaled_biome_size(size, multiplier_tenths);
            let entry = BiomeFieldEntry {
                id: biome.id.clone(),
                distributions: biome.distributions.clone(),
                size,
                weight: dimension_biome.weight,
                climate: biome.climate,
                vertical_range: biome.vertical_range,
                priority: biome.priority,
                terrain: biome.terrain,
                terrain_modifiers: biome.terrain_modifiers.clone(),
                hydrology: biome.hydrology.rules(),
                density_modifier: biome.density_modifier,
                solid_block: biome.solid_block.clone(),
                density_seed: biome_density_seed(seed, &biome.id),
                avoid_near: dimension_biome.avoid_near.clone(),
                require_near: dimension_biome.require_near.clone(),
            };

            match biome.kind {
                BiomeKind::Surface => {
                    if entry.weight > 0.0 {
                        surface_minimum_radius.x = surface_minimum_radius.x.max(entry.size.x.min);
                        surface_minimum_radius.y = surface_minimum_radius.y.max(entry.size.z.min);
                    }
                    surface_biomes.push(entry);
                }
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
                BiomeKind::Hydrology => unreachable!(),
            }
        }

        assert!(
            surface_biomes.iter().any(|biome| biome.weight > 0.0),
            "dimension {} must define at least one active surface biome",
            dimension.id
        );

        let surface_site_spacing = surface_minimum_spacing(surface_minimum_radius);
        let volume_site_spacing = has_active_volume_biome
            .then_some(volume_minimum_radius * 2.0 + Vec3::splat(VOLUME_SITE_GAP));
        let ocean_biome_id = dimension.hydrology.ocean_biome.clone();
        let ocean_weight = ocean_biome_id
            .as_deref()
            .map(|id| dimension.biome_weight(id))
            .unwrap_or(0.0);
        let ocean_surface_index = ocean_biome_id
            .as_deref()
            .and_then(|id| surface_biomes.iter().position(|biome| biome.id == id));
        Self {
            surface_biomes,
            volume_biomes,
            surface_site_spacing,
            volume_site_spacing,
            climate: MacroClimateField::new(seed),
            seed,
            surface_site_biomes: Arc::new(RwLock::new(HashMap::new())),
            forced_surface_biome: None,
            ocean_surface_index,
            ocean_weight,
        }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub(crate) fn climate_at(&self, position: Vec2) -> MacroClimateSample {
        let mut climate = self.climate.sample(position);
        if let Some((index, weight)) = self.forced_surface_biome_at(position)
            && Some(index) != self.ocean_surface_index
        {
            climate.continentalness = suppress_ocean_continentalness(
                climate.continentalness,
                self.ocean_weight,
                weight,
            );
        }
        climate
    }

    pub(crate) fn force_surface_biome(&mut self, biome_id: &str, center: Vec2) {
        let biome_index = self
            .surface_biomes
            .iter()
            .position(|biome| biome.id == biome_id)
            .unwrap_or_else(|| panic!("forced surface biome is not a surface biome: {biome_id}"));
        let biome = &self.surface_biomes[biome_index];
        let hash = biome_density_seed(self.seed ^ 0x6a09_e667_f3bc_c909, biome_id);
        let radii = Vec2::new(
            lerp(
                biome.size.x.min,
                biome.size.x.max,
                hash_unit(hash.rotate_left(11)),
            ),
            lerp(
                biome.size.z.min,
                biome.size.z.max,
                hash_unit(hash.rotate_left(37)),
            ),
        );
        self.forced_surface_biome = Some(ForcedSurfaceBiome {
            biome_index,
            center,
            radii,
            warp_seed: self.seed ^ hash.rotate_left(23),
        });
    }

    pub(crate) fn forced_surface_core_contains(&self, position: Vec2) -> bool {
        self.forced_surface_biome
            .is_some_and(|forced| forced.core_contains(position))
    }

    pub(super) fn forced_surface_biome_at(&self, position: Vec2) -> Option<(usize, f32)> {
        let forced = self.forced_surface_biome?;
        let weight = forced.weight(position);
        (weight > 0.0).then_some((forced.biome_index, weight))
    }

    pub(crate) fn surface_biome_id(&self, index: usize) -> &str {
        self.surface_biomes
            .get(index)
            .unwrap_or_else(|| panic!("surface biome index out of bounds: {index}"))
            .id
            .as_str()
    }

    pub(crate) fn surface_biome_hydrology(&self, index: usize) -> BiomeHydrologyRules {
        self.surface_biomes
            .get(index)
            .unwrap_or_else(|| panic!("surface biome index out of bounds: {index}"))
            .hydrology
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
                x: DimensionBiomeSizeAxis { min: 75.0, max: 125.0 },
                z: DimensionBiomeSizeAxis { min: 41.0, max: 99.0 },
                y: Some(DimensionBiomeSizeAxis { min: 15.0, max: 35.0 }),
            },
            5,
        );

        assert_eq!(scaled.x.min, 38.0);
        assert_eq!(scaled.x.max, 63.0);
        assert_eq!(scaled.z.min, 21.0);
        assert_eq!(scaled.z.max, 50.0);
        let y = scaled.y.expect("scaled vertical size should remain defined");
        assert_eq!(y.min, 8.0);
        assert_eq!(y.max, 18.0);
    }
}
