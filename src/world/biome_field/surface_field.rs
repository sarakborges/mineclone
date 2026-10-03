use bevy::prelude::*;

use crate::world::deterministic::hash_signed;

use super::{
    BiomeField, BiomeFieldEntry,
    spatial::{cell_hash, hash_unit, surface_value_noise, warp_surface_position},
};

const LAND_SITE_JITTER_FRACTION: f32 = 0.28;
const LAND_SITE_SEARCH_RADIUS: i32 = 2;
const OCEAN_DOMAIN_SCALE_MULTIPLIER: f32 = 2.0;
const OCEAN_DOMAIN_THRESHOLD: f32 = 0.18;
const OCEAN_DOMAIN_DETAIL_WEIGHT: f32 = 0.28;
const OCEAN_DOMAIN_DETAIL_SCALE_MULTIPLIER: f32 = 0.38;
const MIN_SURFACE_REGION_SCALE: f32 = 64.0;
const LAND_HASH_SALT: u64 = 0x9e37_79b1_85eb_ca87;
const OCEAN_HASH_SALT: u64 = 0xd6e8_feb8_6659_fd93;

#[derive(Clone, Copy, Debug)]
pub(super) struct SurfaceFieldConfig {
    pub(super) land_spacing: Vec2,
    pub(super) ocean_scale: f32,
}

impl SurfaceFieldConfig {
    pub(super) fn from_biomes(
        surface_biomes: &[BiomeFieldEntry],
        ocean_surface_index: Option<usize>,
    ) -> Self {
        let mut weighted_spacing = Vec2::ZERO;
        let mut total_weight = 0.0_f32;

        for (index, biome) in surface_biomes.iter().enumerate() {
            if biome.weight <= 0.0 || Some(index) == ocean_surface_index {
                continue;
            }

            let midpoint = Vec2::new(
                (biome.size.x.min + biome.size.x.max) * 0.5,
                (biome.size.z.min + biome.size.z.max) * 0.5,
            );
            weighted_spacing += midpoint * biome.weight;
            total_weight += biome.weight;
        }

        let land_spacing = if total_weight > f32::EPSILON {
            weighted_spacing / total_weight
        } else {
            Vec2::splat(MIN_SURFACE_REGION_SCALE)
        }
        .max(Vec2::splat(MIN_SURFACE_REGION_SCALE));

        let authored_ocean_scale = ocean_surface_index
            .and_then(|index| surface_biomes.get(index))
            .map(|biome| {
                ((biome.size.x.min + biome.size.x.max + biome.size.z.min + biome.size.z.max)
                    * 0.25)
                    .max(MIN_SURFACE_REGION_SCALE)
            })
            .unwrap_or(land_spacing.max_element());

        Self {
            land_spacing,
            ocean_scale: authored_ocean_scale
                .max(land_spacing.max_element() * OCEAN_DOMAIN_SCALE_MULTIPLIER),
        }
    }
}

impl BiomeField {
    pub(super) fn surface_biome_index_at(&self, position: Vec2) -> usize {
        if let Some(index) = self.single_surface_biome {
            return index;
        }

        let warped = warp_surface_position(position, self.seed);
        if self.spawn_oceans
            && let Some(ocean_index) = self.ocean_surface_index
            && self.ocean_domain_contains(warped)
        {
            return ocean_index;
        }

        self.nearest_land_site_biome(warped)
    }

    fn ocean_domain_contains(&self, warped: Vec2) -> bool {
        let scale = self.surface_field_config.ocean_scale.max(1.0);
        let broad = surface_value_noise(warped / scale, self.seed ^ OCEAN_HASH_SALT);
        let detail = surface_value_noise(
            warped / (scale * OCEAN_DOMAIN_DETAIL_SCALE_MULTIPLIER).max(1.0)
                + Vec2::new(29.0, -17.0),
            self.seed ^ OCEAN_HASH_SALT.rotate_left(23),
        );
        broad + detail * OCEAN_DOMAIN_DETAIL_WEIGHT > OCEAN_DOMAIN_THRESHOLD
    }

    fn nearest_land_site_biome(&self, warped: Vec2) -> usize {
        let spacing = self.surface_field_config.land_spacing;
        let center = IVec2::new(
            (warped.x / spacing.x).floor() as i32,
            (warped.y / spacing.y).floor() as i32,
        );

        let mut best: Option<(f32, u64, usize)> = None;
        for z in -LAND_SITE_SEARCH_RADIUS..=LAND_SITE_SEARCH_RADIUS {
            for x in -LAND_SITE_SEARCH_RADIUS..=LAND_SITE_SEARCH_RADIUS {
                let cell = center + IVec2::new(x, z);
                let biome_index = self.land_biome_for_cell(cell);
                let site = land_site_position(cell, spacing, self.seed);
                let distance = warped.distance_squared(site);
                let tie_break = cell_hash(cell, self.seed ^ LAND_HASH_SALT.rotate_left(17));
                let candidate = (distance, tie_break, biome_index);

                if best.is_none_or(|current| {
                    candidate.0 < current.0
                        || (candidate.0 == current.0 && candidate.1 < current.1)
                }) {
                    best = Some(candidate);
                }
            }
        }

        best.map(|(_, _, index)| index)
            .unwrap_or_else(|| self.first_enabled_land_biome())
    }

    fn land_biome_for_cell(&self, cell: IVec2) -> usize {
        let total_weight = self
            .surface_biomes
            .iter()
            .enumerate()
            .filter(|(index, biome)| {
                Some(*index) != self.ocean_surface_index
                    && biome.weight > 0.0
                    && self.surface_biome_is_enabled(*index)
            })
            .map(|(_, biome)| biome.weight)
            .sum::<f32>();

        if total_weight <= f32::EPSILON {
            return self.first_enabled_land_biome();
        }

        let target = hash_unit(cell_hash(cell, self.seed ^ LAND_HASH_SALT)) * total_weight;
        let mut cumulative = 0.0_f32;
        let mut fallback = None;
        for (index, biome) in self.surface_biomes.iter().enumerate() {
            if Some(index) == self.ocean_surface_index
                || biome.weight <= 0.0
                || !self.surface_biome_is_enabled(index)
            {
                continue;
            }
            fallback = Some(index);
            cumulative += biome.weight;
            if target < cumulative {
                return index;
            }
        }

        fallback.unwrap_or_else(|| self.first_enabled_land_biome())
    }

    fn first_enabled_land_biome(&self) -> usize {
        self.surface_biomes
            .iter()
            .enumerate()
            .find(|(index, biome)| {
                Some(*index) != self.ocean_surface_index
                    && biome.weight > 0.0
                    && self.surface_biome_is_enabled(*index)
            })
            .map(|(index, _)| index)
            .or(self
                .ocean_surface_index
                .filter(|index| self.surface_biome_is_enabled(*index)))
            .unwrap_or(0)
    }
}

fn land_site_position(cell: IVec2, spacing: Vec2, seed: u64) -> Vec2 {
    let base = (cell.as_vec2() + Vec2::splat(0.5)) * spacing;
    let jitter_x = hash_signed(cell_hash(
        cell,
        seed ^ LAND_HASH_SALT ^ 0x243f_6a88_85a3_08d3,
    ));
    let jitter_z = hash_signed(cell_hash(
        cell,
        seed ^ LAND_HASH_SALT ^ 0x1319_8a2e_0370_7344,
    ));
    base
        + Vec2::new(
            jitter_x * spacing.x * LAND_SITE_JITTER_FRACTION,
            jitter_z * spacing.y * LAND_SITE_JITTER_FRACTION,
        )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        content::{
            biome::BiomeClimate,
            dimension::{DimensionBiomeSize, DimensionBiomeSizeAxis},
        },
        world::macro_climate::MacroClimateField,
    };

    fn size(min: f32, max: f32) -> DimensionBiomeSize {
        let axis = DimensionBiomeSizeAxis { min, max };
        DimensionBiomeSize {
            x: axis,
            z: axis,
            y: None,
        }
    }

    fn entry(id: &str, weight: f32, size: DimensionBiomeSize) -> BiomeFieldEntry {
        BiomeFieldEntry {
            id: id.to_owned(),
            tags: Vec::new(),
            surface_constraints: None,
            size,
            weight,
            climate: BiomeClimate::default(),
            vertical_range: None,
            priority: 0,
            terrain: None,
            terrain_modifiers: Vec::new(),
            density_modifier: None,
            solid_block: None,
            density_seed: 0,
            surface_margin: None,
        }
    }

    fn field(seed: u64) -> BiomeField {
        let surface_biomes = Arc::new(vec![
            entry("test:plains", 1.0, size(120.0, 420.0)),
            entry("test:wasteland", 1.3, size(120.0, 420.0)),
            entry("test:ocean", 1.0, size(180.0, 520.0)),
        ]);
        let config = SurfaceFieldConfig::from_biomes(&surface_biomes, Some(2));
        BiomeField {
            surface_biomes,
            volume_biomes: Arc::new(Vec::new()),
            surface_site_spacing: Vec2::splat(8.0),
            surface_field_config: config,
            volume_site_spacing: None,
            climate: MacroClimateField::new(seed),
            seed,
            single_surface_biome: None,
            ocean_surface_index: Some(2),
            spawn_oceans: true,
            spawn_target_surface_biome: None,
        }
    }

    #[test]
    fn same_seed_and_position_are_stable() {
        let field = field(42);
        let position = Vec2::new(13_337.25, -9_001.5);
        assert_eq!(
            field.surface_biome_index_at(position),
            field.surface_biome_index_at(position)
        );
    }

    #[test]
    fn direct_far_query_does_not_require_incremental_planning() {
        let field = field(42);
        let far = field.surface_biome_index_at(Vec2::new(1_000_000.0, -1_000_000.0));
        let near = field.surface_biome_index_at(Vec2::new(8.0, 8.0));
        let far_again = field.surface_biome_index_at(Vec2::new(1_000_000.0, -1_000_000.0));
        assert_eq!(far, far_again);
        assert!(near < field.surface_biomes.len());
    }

    #[test]
    fn query_order_does_not_change_results() {
        let a = field(77);
        let b = field(77);
        let points = [
            Vec2::new(-900.0, 120.0),
            Vec2::new(3_000.0, 4_000.0),
            Vec2::new(64.0, -128.0),
        ];
        let forward = points.map(|point| a.surface_biome_index_at(point));
        let mut reverse = points;
        reverse.reverse();
        for point in reverse {
            let _ = b.surface_biome_index_at(point);
        }
        assert_eq!(
            forward,
            points.map(|point| b.surface_biome_index_at(point))
        );
    }

    #[test]
    fn different_seeds_change_the_field() {
        let a = field(1);
        let b = field(2);
        assert!((0..64).any(|i| {
            let point = Vec2::new(i as f32 * 173.0, i as f32 * -91.0);
            a.surface_biome_index_at(point) != b.surface_biome_index_at(point)
        }));
    }
}
