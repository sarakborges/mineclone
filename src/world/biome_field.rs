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

pub struct BiomeInfluence<'a> {
    pub id: &'a str,
    pub weight: f32,
}

pub struct BiomeFieldSample<'a> {
    pub primary_id: &'a str,
    pub influences: Vec<BiomeInfluence<'a>>,
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

            if let Some(vertical_size) = biome.size.y {
                validate_size_axis(biome_id, "y", vertical_size.min, vertical_size.max);
            }

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
                influences: vec![BiomeInfluence { id, weight: 1.0 }],
            };
        }

        let warped = warp_position(position);
        let center = IVec2::new(
            (warped.x / self.site_spacing.x).round() as i32,
            (warped.y / self.site_spacing.y).round() as i32,
        );
        let mut sites = Vec::new();
        let mut nearest_distance = f32::MAX;
        let mut primary_index = 0;

        for z in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
            for x in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
                let cell = center + IVec2::new(x, z);
                let site = site_position(cell, self.site_spacing);
                let distance = warped.distance(site);
                let biome_index = biome_index(cell, self.biome_ids.len());

                if distance < nearest_distance {
                    nearest_distance = distance;
                    primary_index = biome_index;
                }

                sites.push((biome_index, distance));
            }
        }

        let mut weights = vec![0.0_f32; self.biome_ids.len()];

        for (biome_index, distance) in sites {
            let distance_gap = (distance - nearest_distance).max(0.0);
            let border_progress =
                1.0 - (distance_gap / BORDER_TRANSITION_WIDTH).clamp(0.0, 1.0);
            let smooth_progress =
                border_progress * border_progress * (3.0 - 2.0 * border_progress);

            weights[biome_index] = weights[biome_index].max(smooth_progress);
        }

        let total_weight: f32 = weights.iter().sum();
        let influences = weights
            .into_iter()
            .enumerate()
            .filter_map(|(index, weight)| {
                if weight <= 0.0 {
                    return None;
                }

                Some(BiomeInfluence {
                    id: self.biome_ids[index].as_str(),
                    weight: weight / total_weight,
                })
            })
            .collect();

        BiomeFieldSample {
            primary_id: self.biome_ids[primary_index].as_str(),
            influences,
        }
    }

    pub fn grass_color(&self, position: Vec2, biomes: &BiomeRegistry) -> Rgb {
        let sample = self.sample(position);
        let mut color = Rgb {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        };

        for influence in sample.influences {
            let biome = biomes
                .get(influence.id)
                .unwrap_or_else(|| panic!("missing biome definition: {}", influence.id));
            let grass = biome.visuals.grass_color;

            color.r += grass.r * influence.weight;
            color.g += grass.g * influence.weight;
            color.b += grass.b * influence.weight;
        }

        color
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
