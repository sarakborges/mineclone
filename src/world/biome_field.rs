mod constants;
mod mountain_belt;
mod mountain_peak;
mod selection;
mod spatial;
mod surface;
mod visuals;
mod volume;

use bevy::prelude::*;

use crate::content::{
    biome::{BiomeClimate, BiomeKind, BiomeRegistry, BiomeSize, BiomeVerticalRange},
    biome_density::BiomeDensityModifier, biome_distribution::BiomeDistribution,
    dimension::DimensionDefinition,
};

pub(crate) use self::volume::{VolumeBiomeRegion, VolumeBiomeSelection};
use self::{constants::VOLUME_SITE_GAP, spatial::surface_minimum_spacing};
use super::macro_climate::{MacroClimateField, MacroClimateSample};

#[derive(Clone)]
pub(super) struct BiomeFieldEntry {
    pub id: String,
    pub distributions: Vec<BiomeDistribution>,
    pub size: BiomeSize,
    pub climate: BiomeClimate,
    pub vertical_range: Option<BiomeVerticalRange>,
    pub priority: i32,
    pub density_modifier: Option<BiomeDensityModifier>,
    pub solid_block: Option<String>,
    pub density_seed: u64,
}

impl BiomeFieldEntry {
    pub(super) fn is_regional(&self) -> bool {
        self.distributions.len() == 1 && self.distributions[0].is_regional()
    }
}

#[derive(Resource)]
pub struct BiomeField {
    pub(super) surface_biomes: Vec<BiomeFieldEntry>,
    pub(super) volume_biomes: Vec<BiomeFieldEntry>,
    pub(super) surface_site_spacing: Vec2,
    pub(super) volume_site_spacing: Option<Vec3>,
    pub(super) climate: MacroClimateField,
    pub(super) seed: u64,
}

#[derive(Clone, Copy)]
pub struct BiomeInfluence<'a> {
    pub id: &'a str,
    pub weight: f32,
}

pub struct BiomeFieldSample<'a> {
    pub primary_id: &'a str,
    pub influences: Vec<BiomeInfluence<'a>>,
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
    ) -> Self {
        assert!(
            !dimension.biomes.is_empty(),
            "dimension {} must define at least one biome",
            dimension.id
        );

        let mut surface_biomes = Vec::new();
        let mut volume_biomes = Vec::new();
        let mut surface_minimum_radius = Vec2::ZERO;
        let mut volume_minimum_radius = Vec3::ZERO;

        for biome_id in &dimension.biomes {
            let biome = biomes
                .get(biome_id)
                .unwrap_or_else(|| panic!("missing biome definition: {biome_id}"));
            let entry = BiomeFieldEntry {
                id: biome.id.clone(),
                distributions: biome.distributions.clone(),
                size: biome.size,
                climate: biome.climate,
                vertical_range: biome.vertical_range,
                priority: biome.priority,
                density_modifier: biome.density_modifier,
                solid_block: biome.solid_block.clone(),
                density_seed: biome_density_seed(seed, &biome.id),
            };

            match biome.kind {
                BiomeKind::Surface => {
                    if entry.is_regional() {
                        surface_minimum_radius.x = surface_minimum_radius.x.max(biome.size.x.min);
                        surface_minimum_radius.y = surface_minimum_radius.y.max(biome.size.z.min);
                    }
                    surface_biomes.push(entry);
                }
                BiomeKind::Volume => {
                    let vertical_size = biome
                        .size
                        .y
                        .unwrap_or_else(|| panic!("volume biome {} must define size.y", biome.id));
                    volume_minimum_radius.x = volume_minimum_radius.x.max(biome.size.x.min);
                    volume_minimum_radius.y = volume_minimum_radius.y.max(vertical_size.min);
                    volume_minimum_radius.z = volume_minimum_radius.z.max(biome.size.z.min);
                    volume_biomes.push(entry);
                }
                BiomeKind::Hydrology => continue,
            }
        }

        assert!(
            surface_biomes.iter().any(BiomeFieldEntry::is_regional),
            "dimension {} must define at least one regional surface biome",
            dimension.id
        );

        let surface_site_spacing = surface_minimum_spacing(surface_minimum_radius);
        let volume_site_spacing = (!volume_biomes.is_empty())
            .then_some(volume_minimum_radius * 2.0 + Vec3::splat(VOLUME_SITE_GAP));

        Self {
            surface_biomes,
            volume_biomes,
            surface_site_spacing,
            volume_site_spacing,
            climate: MacroClimateField::new(seed),
            seed,
        }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub(crate) fn climate_at(&self, position: Vec2) -> MacroClimateSample {
        self.climate.sample(position)
    }

    pub(crate) fn surface_biome_index(&self, biome_id: &str) -> usize {
        self.surface_biomes
            .iter()
            .position(|biome| biome.id == biome_id)
            .unwrap_or_else(|| panic!("missing surface biome in field: {biome_id}"))
    }

    pub(crate) fn surface_biome_id(&self, index: usize) -> &str {
        self.surface_biomes
            .get(index)
            .unwrap_or_else(|| panic!("surface biome index out of bounds: {index}"))
            .id
            .as_str()
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
