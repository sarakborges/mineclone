use bevy::prelude::*;

use crate::content::{
    biome::BiomeRegistry,
    color::Rgb,
    dimension::DimensionDefinition,
};

const BORDER_TRANSITION_WIDTH: f32 = 32.0;
const BORDER_WARP_AMPLITUDE: f32 = 24.0;
const SITE_JITTER_FRACTION: f32 = 0.32;
const SITE_SEARCH_RADIUS: i32 = 2;

#[derive(Resource)]
pub struct BiomeField {
    biome_ids: Vec<String>,
    site_spacing: Vec2,
    seed: u64,
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
            seed,
        }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn sample(&self, position: Vec2) -> BiomeFieldSample<'_> {
        if self.biome_ids.len() == 1 {
            let id = self.biome_ids[0].as_str();
            return BiomeFieldSample {
                primary_id: id,
                influences: vec![BiomeInfluence { id, weight: 1.0 }],
            };
        }

        let warped = warp_position(position, self.seed);
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
                let site = site_position(cell, self.site_spacing, self.seed);
                let distance = warped.distance(site);
                let candidate_index = biome_index(cell, self.biome_ids.len(), self.seed);

                if distance < nearest_distance {
                    nearest_distance = distance;
                    primary_index = candidate_index;
                }

                sites.push((candidate_index, distance));
            }
        }

        let mut weights = vec![0.0_f32; self.biome_ids.len()];

        for (candidate_index, distance) in sites {
            let distance_gap = (distance - nearest_distance).max(0.0);
            let border_progress =
                1.0 - (distance_gap / BORDER_TRANSITION_WIDTH).clamp(0.0, 1.0);
            let smooth_progress =
                border_progress * border_progress * (3.0 - 2.0 * border_progress);
            let previous_weight = weights[candidate_index];

            weights[candidate_index] = previous_weight.max(smooth_progress);
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

fn warp_position(position: Vec2, seed: u64) -> Vec2 {
    let phase_x = hash_component(seed) * std::f32::consts::TAU;
    let phase_z = hash_component(seed.rotate_left(31)) * std::f32::consts::TAU;

    position
        + Vec2::new(
            (position.y * 0.011 + phase_x).sin() * BORDER_WARP_AMPLITUDE,
            (position.x * 0.009 + phase_z).sin() * BORDER_WARP_AMPLITUDE,
        )
}

fn site_position(cell: IVec2, spacing: Vec2, seed: u64) -> Vec2 {
    let base = Vec2::new(cell.x as f32 * spacing.x, cell.y as f32 * spacing.y);

    if cell == IVec2::ZERO {
        return base;
    }

    let hash = cell_hash(cell, seed);
    let jitter_x = hash_component(hash) * spacing.x * SITE_JITTER_FRACTION;
    let jitter_z = hash_component(hash.rotate_left(29)) * spacing.y * SITE_JITTER_FRACTION;

    base + Vec2::new(jitter_x, jitter_z)
}

fn biome_index(cell: IVec2, biome_count: usize, seed: u64) -> usize {
    if cell == IVec2::ZERO {
        return 0;
    }

    cell_hash(cell, seed) as usize % biome_count
}

fn cell_hash(cell: IVec2, seed: u64) -> u64 {
    let mut hash = seed ^ 0xa076_1d64_78bd_642f;
    hash ^= (cell.x as i64 as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
    hash ^= (cell.y as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^= hash >> 33;
    hash
}

fn hash_component(hash: u64) -> f32 {
    let normalized = (hash & 0xffff) as f32 / u16::MAX as f32;
    normalized * 2.0 - 1.0
}
