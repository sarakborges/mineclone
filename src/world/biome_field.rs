mod constants;
mod resolved;
mod selection;
mod spatial;
mod surface;
mod visuals;
mod volume;

use bevy::prelude::*;

use crate::content::{
    biome::{BiomeClimate, BiomeKind, BiomeRegistry, BiomeSize, BiomeVerticalRange},
    dimension::DimensionDefinition,
};

use self::{constants::VOLUME_SITE_GAP, spatial::surface_minimum_spacing};
use super::macro_climate::{MacroClimateField, MacroClimateSample};

#[derive(Clone)]
pub(super) struct BiomeFieldEntry {
    pub id: String,
    pub size: BiomeSize,
    pub climate: BiomeClimate,
    pub vertical_range: Option<BiomeVerticalRange>,
    pub priority: i32,
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

pub struct VolumeBiomeFieldSample<'a> {
    pub primary_id: &'a str,
    pub influences: Vec<BiomeInfluence<'a>>,
    pub strength: f32,
}

pub struct ResolvedBiomeFieldSample<'a> {
    pub primary_id: &'a str,
    pub influences: Vec<BiomeInfluence<'a>>,
    pub surface: BiomeFieldSample<'a>,
    pub volume: Option<VolumeBiomeFieldSample<'a>>,
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
                size: biome.size,
                climate: biome.climate,
                vertical_range: biome.vertical_range,
                priority: biome.priority,
            };

            match biome.kind {
                BiomeKind::Surface => {
                    surface_minimum_radius.x = surface_minimum_radius.x.max(biome.size.x.min);
                    surface_minimum_radius.y = surface_minimum_radius.y.max(biome.size.z.min);
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
            !surface_biomes.is_empty(),
            "dimension {} must define at least one surface biome",
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

    pub fn sample(&self, position: Vec2) -> BiomeFieldSample<'_> {
        self.sample_surface(position)
    }
}
