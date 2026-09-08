use bevy::prelude::*;

use crate::content::{
    biome::BiomeRegistry,
    color::Rgb,
    dimension::DimensionDefinition,
};

const BORDER_TRANSITION_WIDTH: f32 = 32.0;
const BORDER_WARP_AMPLITUDE: f32 = 24.0;
const SITE_SEARCH_RADIUS: i32 = 2;

#[derive(Resource)]
pub struct BiomeField {
    biome_ids: Vec<String>,
    site_spacing: Vec2,
}

pub struct BiomeFieldSample<'a> {
    pub primary_id: &'a str,
    pub secondary_id: &'a str,
    pub secondary_weight: f32,
}

impl BiomeField {
    pub fn from_dimension(dimension: &DimensionDefinition, biomes: &BiomeRegistry) -> Self {
        assert!(
            !dimension.biomes.is_empty(),
            "dimension {} must define at least one biome",
            dimension.id
        );

        let mut minimum_radius = Vec2::ZERO;

        for biome_id in &dimension.biomes {
            let biome = biomes
                .get(biome_id)
                .unwrap_or_else(|| panic!("missing biome definition: {biome_id}"));

            validate_size_axis(biome_id, "x", biome.size.x.min, biome.size.x.max);
            validate_size_axis(biome_id, "z", biome.size.z.min, biome.size.z.max);

            minimum_radius.x = minimum_radius.x.max(biome.size.x.min);
            minimum_radius.y = minimum_radius.y.max(biome.size.z.min);
        }

        let border_allowance = BORDER_TRANSITION_WIDTH + BORDER_WARP_AMPLITUDE * 2.0;
        let site_spacing = minimum_radius * 2.0 + Vec2::splat(border_allowance);

        Self {
            biome_ids: dimension.biomes.clone(),
            site_spacing,
        }
    }

    pub fn sample(&self, position: Vec2) -> BiomeFieldSample<'_> {
        if self.biome_ids.len() == 1 {
            let id = self.biome_ids[0].as_str();
            return BiomeFieldSample {
                primary_id: id,
                secondary_id: id,
                secondary_weight: 0.0,
            };
        }

        let warped = warp_position(position);
        let center = IVec2::new(
            (warped.x / self.site_spacing.x).round() as i32,
            (warped.y / self.site_spacing.y).round() as i32,
        );

        let mut nearest_distance_squared = f32::MAX;
        let mut nearest_cell = center;
        let mut primary_index = 0;

        for z in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
            for x in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
                let cell = center + IVec2::new(x, z);
                let site = site_position(cell, self.site_spacing);
                let distance_squared = warped.distance_squared(site);

                if distance_squared < nearest_distance_squared {
                    nearest_distance_squared = distance_squared;
                    nearest_cell = cell;
                    primary_index = biome_index(cell, self.biome_ids.len());
                }
            }
        }

        let mut secondary_distance_squared = f32::MAX;
        let mut secondary_index = primary_index;

        for z in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
            for x in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
                let cell = nearest_cell + IVec2::new(x, z);
                let candidate_index = biome_index(cell, self.biome_ids.len());

                if candidate_index == primary_index {
                    continue;
                }

                let site = site_position(cell, self.site_spacing);
                let distance_squared = warped.distance_squared(site);

                if distance_squared < secondary_distance_squared {
                    secondary_distance_squared = distance_squared;
                    secondary_index = candidate_index;
                }
            }
        }

        if secondary_index == primary_index || secondary_distance_squared == f32::MAX {
            let id = self.biome_ids[primary_index].as_str();
            return BiomeFieldSample {
                primary_id: id,
                secondary_id: id,
                secondary_weight: 0.0,
            };
        }

        let primary_distance = nearest_distance_squared.sqrt();
        let secondary_distance = secondary_distance_squared.sqrt();
        let distance_gap = (secondary_distance - primary_distance).max(0.0);
        let border_progress = 1.0 - (distance_gap / BORDER_TRANSITION_WIDTH).clamp(0.0, 1.0);
        let smooth_progress = border_progress * border_progress * (3.0 - 2.0 * border_progress);

        BiomeFieldSample {
            primary_id: self.biome_ids[primary_index].as_str(),
            secondary_id: self.biome_ids[secondary_index].as_str(),
            secondary_weight: smooth_progress * 0.5,
        }
    }

    pub fn grass_color(&self, position: Vec2, biomes: &BiomeRegistry) -> Rgb {
        let sample = self.sample(position);
        let primary = biomes
            .get(sample.primary_id)
            .unwrap_or_else(|| panic!("missing biome definition: {}", sample.primary_id));
        let secondary = biomes
            .get(sample.secondary_id)
            .unwrap_or_else(|| panic!("missing biome definition: {}", sample.secondary_id));

        primary
            .visuals
            .grass_color
            .lerp(secondary.visuals.grass_color, sample.secondary_weight)
    }
}

fn validate_size_axis(biome_id: &str, axis: &str, min: f32, max: f32) {
    assert!(min > 0.0, "biome {biome_id} size.{axis}.min must be positive");
    assert!(
        max >= min,
        "biome {biome_id} size.{axis}.max must be greater than or equal to min"
    );
}

fn warp_position(position: Vec2) -> Vec2 {
    position
        + Vec2::new(
            (position.y * 0.011).sin() * BORDER_WARP_AMPLITUDE,
            (position.x * 0.009).sin() * BORDER_WARP_AMPLITUDE,
        )
}

fn site_position(cell: IVec2, spacing: Vec2) -> Vec2 {
    Vec2::new(cell.x as f32 * spacing.x, cell.y as f32 * spacing.y)
}

fn biome_index(cell: IVec2, biome_count: usize) -> usize {
    if cell == IVec2::ZERO {
        return 0;
    }

    let mut hash = (cell.x as i64 as u64).wrapping_mul(0x9E37_79B1_85EB_CA87);
    hash ^= (cell.y as i64 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    hash ^= hash >> 33;

    hash as usize % biome_count
}
