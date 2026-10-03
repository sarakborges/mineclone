use bevy::prelude::*;

use crate::world::deterministic::hash_signed;

use super::{
    BiomeField, BiomeFieldEntry, SurfaceBoundarySample,
    spatial::{
        cell_hash, hash_unit, lerp, smoothstep, surface_value_noise, warp_surface_position,
    },
};

const LAND_SITE_JITTER_FRACTION: f32 = 0.28;
const LAND_SITE_SEARCH_RADIUS: i32 = 3;
const OCEAN_DOMAIN_SCALE_MULTIPLIER: f32 = 2.0;
const OCEAN_DOMAIN_THRESHOLD: f32 = 0.18;
const OCEAN_DOMAIN_DETAIL_WEIGHT: f32 = 0.28;
const OCEAN_DOMAIN_DETAIL_SCALE_MULTIPLIER: f32 = 0.38;
const OCEAN_DISTANCE_GRADIENT_STEP: f32 = 2.0;
const MIN_SURFACE_REGION_SCALE: f32 = 64.0;
const LAND_HASH_SALT: u64 = 0x9e37_79b1_85eb_ca87;
const LAND_SCALE_X_SALT: u64 = 0xa409_3822_299f_31d0;
const LAND_SCALE_Z_SALT: u64 = 0x082e_fa98_ec4e_6c89;
const OCEAN_HASH_SALT: u64 = 0xd6e8_feb8_6659_fd93;

#[derive(Clone, Copy, Debug)]
pub(super) struct SurfaceFieldConfig {
    pub(super) land_spacing: Vec2,
    pub(super) ocean_scale: f32,
    land_total_weight: f32,
    first_land_index: usize,
}

impl SurfaceFieldConfig {
    pub(super) fn from_biomes(
        surface_biomes: &[BiomeFieldEntry],
        ocean_surface_index: Option<usize>,
    ) -> Self {
        let mut weighted_spacing = Vec2::ZERO;
        let mut total_weight = 0.0_f32;
        let mut first_land_index = None;

        for (index, biome) in surface_biomes.iter().enumerate() {
            if biome.weight <= 0.0 || Some(index) == ocean_surface_index {
                continue;
            }

            first_land_index.get_or_insert(index);
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
                ((biome.size.x.min + biome.size.x.max + biome.size.z.min + biome.size.z.max) * 0.25)
                    .max(MIN_SURFACE_REGION_SCALE)
            })
            .unwrap_or(land_spacing.max_element());

        Self {
            land_spacing,
            ocean_scale: authored_ocean_scale
                .max(land_spacing.max_element() * OCEAN_DOMAIN_SCALE_MULTIPLIER),
            land_total_weight: total_weight,
            first_land_index: first_land_index.or(ocean_surface_index).unwrap_or(0),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct LandSiteCandidate {
    score: f32,
    tie_break: u64,
    biome_index: usize,
    site: Vec2,
    scale: Vec2,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct SurfaceFieldSample {
    pub(super) primary_index: usize,
    pub(super) primary_terrain_strength: f32,
    pub(super) boundary: Option<SurfaceBoundarySample>,
}

impl BiomeField {
    pub(super) fn surface_biome_index_at(&self, position: Vec2) -> usize {
        if let Some(index) = self.single_surface_biome {
            return index;
        }

        let warped = warp_surface_position(position, self.seed);
        if self.spawn_oceans
            && let Some(ocean_index) = self.ocean_surface_index
            && self.ocean_domain_value(warped) > 0.0
        {
            return ocean_index;
        }

        self.nearest_land_site(warped).biome_index
    }

    pub(super) fn surface_field_sample_at(&self, position: Vec2) -> SurfaceFieldSample {
        if let Some(index) = self.single_surface_biome {
            return SurfaceFieldSample {
                primary_index: index,
                primary_terrain_strength: 1.0,
                boundary: None,
            };
        }

        let warped = warp_surface_position(position, self.seed);
        let best_land = self.nearest_land_site(warped);
        let best_land_strength = land_site_terrain_strength(best_land);
        let ocean = self
            .spawn_oceans
            .then_some(self.ocean_surface_index)
            .flatten()
            .map(|index| (index, self.ocean_domain_value(warped)));

        if let Some((ocean_index, ocean_value)) = ocean
            && ocean_value > 0.0
        {
            return SurfaceFieldSample {
                primary_index: ocean_index,
                primary_terrain_strength: 1.0,
                boundary: Some(SurfaceBoundarySample {
                    neighbor_surface_index: best_land.biome_index,
                    neighbor_terrain_strength: best_land_strength,
                    distance: self.ocean_boundary_distance(warped, ocean_value),
                }),
            };
        }

        let mut boundary = self
            .nearest_different_land_site(warped, best_land)
            .and_then(|other| {
                land_boundary_distance(warped, best_land, other).map(|distance| {
                    SurfaceBoundarySample {
                        neighbor_surface_index: other.biome_index,
                        neighbor_terrain_strength: land_site_terrain_strength(other),
                        distance,
                    }
                })
            });

        if let Some((ocean_index, ocean_value)) = ocean {
            let ocean_boundary = SurfaceBoundarySample {
                neighbor_surface_index: ocean_index,
                neighbor_terrain_strength: 1.0,
                distance: self.ocean_boundary_distance(warped, ocean_value),
            };
            if boundary.is_none_or(|current| {
                ocean_boundary.distance < current.distance
                    || (ocean_boundary.distance == current.distance
                        && ocean_boundary.neighbor_surface_index < current.neighbor_surface_index)
            }) {
                boundary = Some(ocean_boundary);
            }
        }

        SurfaceFieldSample {
            primary_index: best_land.biome_index,
            primary_terrain_strength: best_land_strength,
            boundary,
        }
    }

    fn ocean_domain_value(&self, warped: Vec2) -> f32 {
        let scale = self.surface_field_config.ocean_scale.max(1.0);
        let broad = surface_value_noise(warped / scale, self.seed ^ OCEAN_HASH_SALT);
        let detail = surface_value_noise(
            warped / (scale * OCEAN_DOMAIN_DETAIL_SCALE_MULTIPLIER).max(1.0)
                + Vec2::new(29.0, -17.0),
            self.seed ^ OCEAN_HASH_SALT.rotate_left(23),
        );
        broad + detail * OCEAN_DOMAIN_DETAIL_WEIGHT - OCEAN_DOMAIN_THRESHOLD
    }

    fn ocean_boundary_distance(&self, warped: Vec2, value: f32) -> f32 {
        let step = OCEAN_DISTANCE_GRADIENT_STEP;
        let dx = (self.ocean_domain_value(warped + Vec2::X * step)
            - self.ocean_domain_value(warped - Vec2::X * step))
            / (step * 2.0);
        let dz = (self.ocean_domain_value(warped + Vec2::Y * step)
            - self.ocean_domain_value(warped - Vec2::Y * step))
            / (step * 2.0);
        let gradient = Vec2::new(dx, dz).length();
        if gradient <= 1e-6 {
            f32::INFINITY
        } else {
            value.abs() / gradient
        }
    }

    fn nearest_land_site(&self, warped: Vec2) -> LandSiteCandidate {
        let spacing = self.surface_field_config.land_spacing;
        let center = IVec2::new(
            (warped.x / spacing.x).floor() as i32,
            (warped.y / spacing.y).floor() as i32,
        );

        let mut best = None;
        for z in -LAND_SITE_SEARCH_RADIUS..=LAND_SITE_SEARCH_RADIUS {
            for x in -LAND_SITE_SEARCH_RADIUS..=LAND_SITE_SEARCH_RADIUS {
                let candidate = self.land_site_candidate(center + IVec2::new(x, z), warped);
                if best.is_none_or(|current| candidate_precedes(candidate, current)) {
                    best = Some(candidate);
                }
            }
        }

        best.unwrap_or_else(|| {
            let cell = center;
            let biome_index = self.surface_field_config.first_land_index;
            let biome = &self.surface_biomes[biome_index];
            let site = land_site_position(cell, spacing, self.seed);
            let scale = land_site_scale(cell, biome, self.seed);
            LandSiteCandidate {
                score: ((warped - site) / scale).length_squared(),
                tie_break: cell_hash(cell, self.seed ^ LAND_HASH_SALT.rotate_left(17)),
                biome_index,
                site,
                scale,
            }
        })
    }

    fn nearest_different_land_site(
        &self,
        warped: Vec2,
        best_land: LandSiteCandidate,
    ) -> Option<LandSiteCandidate> {
        let spacing = self.surface_field_config.land_spacing;
        let center = IVec2::new(
            (warped.x / spacing.x).floor() as i32,
            (warped.y / spacing.y).floor() as i32,
        );
        let mut best_other = None;

        for z in -LAND_SITE_SEARCH_RADIUS..=LAND_SITE_SEARCH_RADIUS {
            for x in -LAND_SITE_SEARCH_RADIUS..=LAND_SITE_SEARCH_RADIUS {
                let candidate = self.land_site_candidate(center + IVec2::new(x, z), warped);
                if candidate.biome_index == best_land.biome_index {
                    continue;
                }
                if best_other.is_none_or(|current| candidate_precedes(candidate, current)) {
                    best_other = Some(candidate);
                }
            }
        }

        best_other
    }

    fn land_site_candidate(&self, cell: IVec2, warped: Vec2) -> LandSiteCandidate {
        let biome_index = self.land_biome_for_cell(cell);
        let biome = &self.surface_biomes[biome_index];
        let site = land_site_position(cell, self.surface_field_config.land_spacing, self.seed);
        let scale = land_site_scale(cell, biome, self.seed);
        LandSiteCandidate {
            score: ((warped - site) / scale).length_squared(),
            tie_break: cell_hash(cell, self.seed ^ LAND_HASH_SALT.rotate_left(17)),
            biome_index,
            site,
            scale,
        }
    }

    fn land_biome_for_cell(&self, cell: IVec2) -> usize {
        let raw = self
            .weighted_land_biome_for_cell(cell, &[], &[])
            .unwrap_or(self.surface_field_config.first_land_index);
        let mut excluded_indices = Vec::<usize>::new();
        let mut excluded_groups = Vec::<&str>::new();

        loop {
            let Some(candidate) =
                self.weighted_land_biome_for_cell(cell, &excluded_indices, &excluded_groups)
            else {
                return raw;
            };

            if !self.neighbor_conflict(cell, candidate) {
                return candidate;
            }

            if let Some(group) = self.surface_biomes[candidate]
                .exclusive_neighbor_group
                .as_deref()
                && !excluded_groups.contains(&group)
            {
                excluded_groups.push(group);
                continue;
            }

            if excluded_indices.contains(&candidate) {
                return raw;
            }
            excluded_indices.push(candidate);
        }
    }

    fn weighted_land_biome_for_cell(
        &self,
        cell: IVec2,
        excluded_indices: &[usize],
        excluded_groups: &[&str],
    ) -> Option<usize> {
        let allowed = |index: usize, biome: &BiomeFieldEntry| {
            Some(index) != self.ocean_surface_index
                && biome.weight > 0.0
                && !excluded_indices.contains(&index)
                && biome
                    .exclusive_neighbor_group
                    .as_deref()
                    .is_none_or(|group| !excluded_groups.contains(&group))
        };
        let total_weight = if excluded_indices.is_empty() && excluded_groups.is_empty() {
            self.surface_field_config.land_total_weight
        } else {
            self.surface_biomes
                .iter()
                .enumerate()
                .filter(|(index, biome)| allowed(*index, biome))
                .map(|(_, biome)| biome.weight)
                .sum::<f32>()
        };
        if total_weight <= f32::EPSILON {
            return None;
        }

        let target = hash_unit(cell_hash(cell, self.seed ^ LAND_HASH_SALT)) * total_weight;
        let mut cumulative = 0.0_f32;
        let mut fallback = None;
        for (index, biome) in self.surface_biomes.iter().enumerate() {
            if !allowed(index, biome) {
                continue;
            }
            fallback = Some(index);
            cumulative += biome.weight;
            if target < cumulative {
                return Some(index);
            }
        }

        fallback
    }

    fn neighbor_conflict(&self, cell: IVec2, candidate: usize) -> bool {
        for z in -1..=1 {
            for x in -1..=1 {
                if x == 0 && z == 0 {
                    continue;
                }
                let Some(neighbor) = self.weighted_land_biome_for_cell(
                    cell + IVec2::new(x, z),
                    &[],
                    &[],
                ) else {
                    continue;
                };
                if neighbor == candidate {
                    continue;
                }

                let candidate_biome = &self.surface_biomes[candidate];
                let neighbor_biome = &self.surface_biomes[neighbor];
                if candidate_biome.exclusive_neighbor_group.is_some()
                    && candidate_biome.exclusive_neighbor_group
                        == neighbor_biome.exclusive_neighbor_group
                {
                    return true;
                }
                if candidate_biome.neighbor_deny.as_ref().is_some_and(|selector| {
                    selector.matches(&neighbor_biome.id, &neighbor_biome.tags)
                }) || neighbor_biome.neighbor_deny.as_ref().is_some_and(|selector| {
                    selector.matches(&candidate_biome.id, &candidate_biome.tags)
                }) {
                    return true;
                }
            }
        }
        false
    }
}

fn candidate_precedes(candidate: LandSiteCandidate, current: LandSiteCandidate) -> bool {
    candidate.score < current.score
        || (candidate.score == current.score && candidate.tie_break < current.tie_break)
}

fn land_site_terrain_strength(candidate: LandSiteCandidate) -> f32 {
    let normalized_distance = candidate.score.max(0.0).sqrt().clamp(0.0, 1.0);
    1.0 - smoothstep(normalized_distance)
}

fn land_boundary_distance(
    warped: Vec2,
    primary: LandSiteCandidate,
    neighbor: LandSiteCandidate,
) -> Option<f32> {
    let score_gap = (neighbor.score - primary.score).max(0.0);
    let primary_scale_sq = primary.scale * primary.scale;
    let neighbor_scale_sq = neighbor.scale * neighbor.scale;
    let primary_gradient = (warped - primary.site) * 2.0 / primary_scale_sq;
    let neighbor_gradient = (warped - neighbor.site) * 2.0 / neighbor_scale_sq;
    let gradient_delta = (neighbor_gradient - primary_gradient).length();
    (gradient_delta > 1e-6).then_some(score_gap / gradient_delta)
}

pub(super) fn land_site_position(cell: IVec2, spacing: Vec2, seed: u64) -> Vec2 {
    let base = (cell.as_vec2() + Vec2::splat(0.5)) * spacing;
    let jitter_x = hash_signed(cell_hash(
        cell,
        seed ^ LAND_HASH_SALT ^ 0x243f_6a88_85a3_08d3,
    ));
    let jitter_z = hash_signed(cell_hash(
        cell,
        seed ^ LAND_HASH_SALT ^ 0x1319_8a2e_0370_7344,
    ));
    base + Vec2::new(
        jitter_x * spacing.x * LAND_SITE_JITTER_FRACTION,
        jitter_z * spacing.y * LAND_SITE_JITTER_FRACTION,
    )
}

fn land_site_scale(cell: IVec2, biome: &BiomeFieldEntry, seed: u64) -> Vec2 {
    let x = hash_unit(cell_hash(cell, seed ^ LAND_SCALE_X_SALT));
    let z = hash_unit(cell_hash(cell, seed ^ LAND_SCALE_Z_SALT));
    Vec2::new(
        lerp(biome.size.x.min, biome.size.x.max, x).max(1.0),
        lerp(biome.size.z.min, biome.size.z.max, z).max(1.0),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        content::{
            biome::{BiomeClimate, SurfaceBiomeSelector},
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
            exclusive_neighbor_group: None,
            neighbor_deny: None,
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
        assert_eq!(forward, points.map(|point| b.surface_biome_index_at(point)));
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

    #[test]
    fn site_scale_stays_inside_authored_bounds() {
        let biome = BiomeFieldEntry {
            size: DimensionBiomeSize {
                x: DimensionBiomeSizeAxis {
                    min: 80.0,
                    max: 180.0,
                },
                z: DimensionBiomeSizeAxis {
                    min: 140.0,
                    max: 360.0,
                },
                y: None,
            },
            ..entry("test:gorge", 1.0, size(1.0, 1.0))
        };
        for z in -8..=8 {
            for x in -8..=8 {
                let scale = land_site_scale(IVec2::new(x, z), &biome, 42);
                assert!((80.0..=180.0).contains(&scale.x));
                assert!((140.0..=360.0).contains(&scale.y));
            }
        }
    }

    #[test]
    fn terrain_strength_falls_from_site_center_to_authored_edge() {
        let centered = LandSiteCandidate {
            score: 0.0,
            tie_break: 0,
            biome_index: 0,
            site: Vec2::ZERO,
            scale: Vec2::ONE,
        };
        let midpoint = LandSiteCandidate {
            score: 0.25,
            ..centered
        };
        let edge = LandSiteCandidate {
            score: 1.0,
            ..centered
        };

        assert_eq!(land_site_terrain_strength(centered), 1.0);
        assert!(land_site_terrain_strength(midpoint) > 0.0);
        assert!(land_site_terrain_strength(midpoint) < 1.0);
        assert_eq!(land_site_terrain_strength(edge), 0.0);
    }

    #[test]
    fn locate_sites_use_the_same_spacing_as_surface_ownership() {
        let field = field(42);
        let spacing = field.surface_field_config.land_spacing;
        let site = land_site_position(IVec2::new(3, -2), spacing, field.seed);
        let cell_center = (IVec2::new(3, -2).as_vec2() + Vec2::splat(0.5)) * spacing;
        assert!((site.x - cell_center.x).abs() <= spacing.x * LAND_SITE_JITTER_FRACTION);
        assert!((site.y - cell_center.y).abs() <= spacing.y * LAND_SITE_JITTER_FRACTION);
    }

    #[test]
    fn boundary_sampling_reuses_the_cell_field() {
        let field = field(42);
        let sample = field.surface_field_sample_at(Vec2::new(128.0, -96.0));
        assert!(sample.primary_index < field.surface_biomes.len());
        assert!((0.0..=1.0).contains(&sample.primary_terrain_strength));
        if let Some(boundary) = sample.boundary {
            assert!(boundary.neighbor_surface_index < field.surface_biomes.len());
            assert!((0.0..=1.0).contains(&boundary.neighbor_terrain_strength));
            assert!(boundary.distance.is_finite() || boundary.distance == f32::INFINITY);
            assert!(boundary.distance >= 0.0);
        }
    }

    #[test]
    fn exclusive_neighbor_groups_do_not_touch() {
        let plains = entry("test:plains", 1.0, size(120.0, 420.0));
        let mut alps = entry("test:alps", 1.0, size(120.0, 320.0));
        alps.exclusive_neighbor_group = Some("test:mountains".to_owned());
        let mut volcano = entry("test:volcano", 1.0, size(120.0, 320.0));
        volcano.exclusive_neighbor_group = Some("test:mountains".to_owned());
        let surface_biomes = Arc::new(vec![plains, alps, volcano]);
        let config = SurfaceFieldConfig::from_biomes(&surface_biomes, None);
        let field = BiomeField {
            surface_biomes,
            volume_biomes: Arc::new(Vec::new()),
            surface_field_config: config,
            volume_site_spacing: None,
            climate: MacroClimateField::new(91),
            seed: 91,
            single_surface_biome: None,
            ocean_surface_index: None,
            spawn_oceans: true,
            spawn_target_surface_biome: None,
        };
        let mut saw_alps = false;
        let mut saw_volcano = false;

        for z in -32..=32 {
            for x in -32..=32 {
                let cell = IVec2::new(x, z);
                let biome = field.land_biome_for_cell(cell);
                saw_alps |= biome == 1;
                saw_volcano |= biome == 2;
                if biome != 1 && biome != 2 {
                    continue;
                }
                for dz in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dz == 0 {
                            continue;
                        }
                        let neighbor = field.land_biome_for_cell(cell + IVec2::new(dx, dz));
                        assert!(
                            neighbor == biome || (neighbor != 1 && neighbor != 2),
                            "exclusive biomes touched at {cell:?}"
                        );
                    }
                }
            }
        }

        assert!(saw_alps && saw_volcano);
    }

    #[test]
    fn neighbor_deny_is_symmetric_for_ids_and_tags() {
        let plains = entry("test:plains", 1.0, size(120.0, 420.0));
        let mut swamp = entry("test:swamp", 1.0, size(120.0, 320.0));
        swamp.neighbor_deny = Some(SurfaceBiomeSelector {
            ids: Vec::new(),
            tags: vec!["mountain".to_owned()],
        });
        let mut mountain = entry("test:mountain", 1.0, size(120.0, 320.0));
        mountain.tags.push("mountain".to_owned());
        let mut arctic = entry("test:arctic", 1.0, size(120.0, 320.0));
        arctic.neighbor_deny = Some(SurfaceBiomeSelector {
            ids: vec!["test:swamp".to_owned()],
            tags: Vec::new(),
        });
        let surface_biomes = Arc::new(vec![plains, swamp, mountain, arctic]);
        let config = SurfaceFieldConfig::from_biomes(&surface_biomes, None);
        let field = BiomeField {
            surface_biomes,
            volume_biomes: Arc::new(Vec::new()),
            surface_field_config: config,
            volume_site_spacing: None,
            climate: MacroClimateField::new(1337),
            seed: 1337,
            single_surface_biome: None,
            ocean_surface_index: None,
            spawn_oceans: true,
            spawn_target_surface_biome: None,
        };

        for z in -32..=32 {
            for x in -32..=32 {
                let cell = IVec2::new(x, z);
                let biome = field.land_biome_for_cell(cell);
                for dz in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dz == 0 {
                            continue;
                        }
                        let neighbor = field.land_biome_for_cell(cell + IVec2::new(dx, dz));
                        assert!(
                            !matches!((biome, neighbor), (1, 2) | (2, 1) | (1, 3) | (3, 1)),
                            "denied biomes touched at {cell:?}"
                        );
                    }
                }
            }
        }
    }
}
