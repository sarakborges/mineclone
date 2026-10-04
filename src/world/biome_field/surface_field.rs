use bevy::prelude::*;

use crate::world::deterministic::hash_signed;

use super::{
    BiomeField, BiomeFieldEntry, SurfaceBoundarySample,
    constants::BORDER_TRANSITION_WIDTH,
    spatial::{
        cell_hash, hash_unit, lerp, smoothstep, surface_value_noise, warp_surface_position,
    },
};

const LAND_SITE_JITTER_FRACTION: f32 = 0.12;
const LAND_SITE_SEARCH_RADIUS: i32 = 3;
const LAND_SCALE_MIN_SPACING_FRACTION: f32 = 0.65;
const LAND_SCALE_MAX_SPACING_FRACTION: f32 = 1.35;
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
        self.surface_field_sample_at(position).primary_index
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
            return self.resolve_surface_adjacency(
                ocean_index,
                1.0,
                Some(SurfaceBoundarySample {
                    neighbor_surface_index: best_land.biome_index,
                    neighbor_terrain_strength: best_land_strength,
                    distance: self.ocean_boundary_distance(warped, ocean_value),
                }),
            );
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

        self.resolve_surface_adjacency(best_land.biome_index, best_land_strength, boundary)
    }

    fn resolve_surface_adjacency(
        &self,
        primary_index: usize,
        primary_terrain_strength: f32,
        boundary: Option<SurfaceBoundarySample>,
    ) -> SurfaceFieldSample {
        let Some(boundary) = boundary else {
            return SurfaceFieldSample {
                primary_index,
                primary_terrain_strength,
                boundary: None,
            };
        };

        if self.surface_biomes_can_neighbor(primary_index, boundary.neighbor_surface_index) {
            return SurfaceFieldSample {
                primary_index,
                primary_terrain_strength,
                boundary: Some(boundary),
            };
        }

        let separator_index = self
            .surface_boundary_separator(primary_index, boundary.neighbor_surface_index)
            .unwrap_or_else(|| {
                panic!(
                    "surface biomes {} and {} deny adjacency but no compatible separator biome exists",
                    self.surface_biomes[primary_index].id,
                    self.surface_biomes[boundary.neighbor_surface_index].id,
                )
            });
        let separator_width = BORDER_TRANSITION_WIDTH.max(f32::EPSILON);

        if boundary.distance <= separator_width {
            SurfaceFieldSample {
                primary_index: separator_index,
                primary_terrain_strength: 1.0,
                boundary: Some(SurfaceBoundarySample {
                    neighbor_surface_index: primary_index,
                    neighbor_terrain_strength: primary_terrain_strength,
                    distance: separator_width - boundary.distance,
                }),
            }
        } else {
            SurfaceFieldSample {
                primary_index,
                primary_terrain_strength,
                boundary: Some(SurfaceBoundarySample {
                    neighbor_surface_index: separator_index,
                    neighbor_terrain_strength: 1.0,
                    distance: boundary.distance - separator_width,
                }),
            }
        }
    }

    pub(crate) fn surface_biomes_can_neighbor(&self, left: usize, right: usize) -> bool {
        if left == right {
            return true;
        }

        let left_biome = self
            .surface_biomes
            .get(left)
            .unwrap_or_else(|| panic!("surface biome index out of bounds: {left}"));
        let right_biome = self
            .surface_biomes
            .get(right)
            .unwrap_or_else(|| panic!("surface biome index out of bounds: {right}"));

        if left_biome.exclusive_neighbor_group.is_some()
            && left_biome.exclusive_neighbor_group == right_biome.exclusive_neighbor_group
        {
            return false;
        }

        !left_biome
            .neighbor_deny
            .as_ref()
            .is_some_and(|selector| selector.matches(&right_biome.id, &right_biome.tags))
            && !right_biome
                .neighbor_deny
                .as_ref()
                .is_some_and(|selector| selector.matches(&left_biome.id, &left_biome.tags))
    }

    fn surface_boundary_separator(&self, left: usize, right: usize) -> Option<usize> {
        self.surface_biomes
            .iter()
            .enumerate()
            .find(|(index, biome)| {
                *index != left
                    && *index != right
                    && biome.weight > 0.0
                    && self.surface_biome_is_enabled(*index)
                    && self.surface_biomes_can_neighbor(left, *index)
                    && self.surface_biomes_can_neighbor(*index, right)
            })
            .map(|(index, _)| index)
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
            let scale = land_site_scale(cell, biome, spacing, self.seed);
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
        let spacing = self.surface_field_config.land_spacing;
        let site = land_site_position(cell, spacing, self.seed);
        let scale = land_site_scale(cell, biome, spacing, self.seed);
        LandSiteCandidate {
            score: ((warped - site) / scale).length_squared(),
            tie_break: cell_hash(cell, self.seed ^ LAND_HASH_SALT.rotate_left(17)),
            biome_index,
            site,
            scale,
        }
    }

    fn land_biome_for_cell(&self, cell: IVec2) -> usize {
        self.weighted_land_biome_for_cell(cell)
            .unwrap_or(self.surface_field_config.first_land_index)
    }

    fn weighted_land_biome_for_cell(&self, cell: IVec2) -> Option<usize> {
        if self.surface_field_config.land_total_weight <= f32::EPSILON {
            return None;
        }

        let target = hash_unit(cell_hash(cell, self.seed ^ LAND_HASH_SALT))
            * self.surface_field_config.land_total_weight;
        let mut cumulative = 0.0_f32;
        let mut fallback = None;

        for (index, biome) in self.surface_biomes.iter().enumerate() {
            if Some(index) == self.ocean_surface_index || biome.weight <= 0.0 {
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

fn land_site_scale(cell: IVec2, biome: &BiomeFieldEntry, spacing: Vec2, seed: u64) -> Vec2 {
    let x = hash_unit(cell_hash(cell, seed ^ LAND_SCALE_X_SALT));
    let z = hash_unit(cell_hash(cell, seed ^ LAND_SCALE_Z_SALT));
    Vec2::new(
        stabilized_axis_scale(x, biome.size.x.min, biome.size.x.max, spacing.x),
        stabilized_axis_scale(z, biome.size.z.min, biome.size.z.max, spacing.y),
    )
}

fn stabilized_axis_scale(sample: f32, authored_min: f32, authored_max: f32, spacing: f32) -> f32 {
    let authored_min = authored_min.max(1.0);
    let authored_max = authored_max.max(authored_min);
    let lower = (spacing * LAND_SCALE_MIN_SPACING_FRACTION).clamp(authored_min, authored_max);
    let upper = (spacing * LAND_SCALE_MAX_SPACING_FRACTION).clamp(lower, authored_max);
    lerp(authored_min, authored_max, sample).clamp(lower, upper)
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
    fn different_seeds_change_the_field() {
        let a = field(1);
        let b = field(2);
        assert!((0..64).any(|i| {
            let point = Vec2::new(i as f32 * 173.0, i as f32 * -91.0);
            a.surface_biome_index_at(point) != b.surface_biome_index_at(point)
        }));
    }

    #[test]
    fn land_selection_is_only_weight_and_seed() {
        let mut constrained = entry("test:constrained", 1.0, size(120.0, 320.0));
        constrained.neighbor_deny = Some(SurfaceBiomeSelector {
            ids: vec!["test:other".to_owned()],
            tags: Vec::new(),
        });
        let surface_biomes = Arc::new(vec![
            entry("test:plains", 1.0, size(120.0, 420.0)),
            constrained,
            entry("test:other", 1.0, size(120.0, 320.0)),
        ]);
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

        for z in -8..=8 {
            for x in -8..=8 {
                let cell = IVec2::new(x, z);
                assert_eq!(
                    field.land_biome_for_cell(cell),
                    field.weighted_land_biome_for_cell(cell).unwrap()
                );
            }
        }
    }

    #[test]
    fn adjacency_rules_are_generic_and_symmetric() {
        let plains = entry("test:plains", 1.0, size(120.0, 420.0));
        let mut wasteland = entry("test:wasteland", 1.0, size(120.0, 320.0));
        wasteland.neighbor_deny = Some(SurfaceBiomeSelector {
            ids: vec!["test:ocean".to_owned()],
            tags: Vec::new(),
        });
        let mut mountain = entry("test:mountain", 1.0, size(120.0, 320.0));
        mountain.tags.push("mountain".to_owned());
        let mut swamp = entry("test:swamp", 1.0, size(120.0, 320.0));
        swamp.neighbor_deny = Some(SurfaceBiomeSelector {
            ids: Vec::new(),
            tags: vec!["mountain".to_owned()],
        });
        let ocean = entry("test:ocean", 1.0, size(180.0, 520.0));
        let surface_biomes = Arc::new(vec![plains, wasteland, mountain, swamp, ocean]);
        let config = SurfaceFieldConfig::from_biomes(&surface_biomes, Some(4));
        let field = BiomeField {
            surface_biomes,
            volume_biomes: Arc::new(Vec::new()),
            surface_field_config: config,
            volume_site_spacing: None,
            climate: MacroClimateField::new(7),
            seed: 7,
            single_surface_biome: None,
            ocean_surface_index: Some(4),
            spawn_oceans: true,
            spawn_target_surface_biome: None,
        };

        assert!(!field.surface_biomes_can_neighbor(1, 4));
        assert!(!field.surface_biomes_can_neighbor(4, 1));
        assert!(!field.surface_biomes_can_neighbor(2, 3));
        assert!(!field.surface_biomes_can_neighbor(3, 2));
        assert!(field.surface_biomes_can_neighbor(0, 4));
    }

    #[test]
    fn denied_boundary_uses_compatible_separator_band() {
        let plains = entry("test:plains", 1.0, size(120.0, 420.0));
        let mut wasteland = entry("test:wasteland", 1.0, size(120.0, 320.0));
        wasteland.neighbor_deny = Some(SurfaceBiomeSelector {
            ids: vec!["test:ocean".to_owned()],
            tags: Vec::new(),
        });
        let ocean = entry("test:ocean", 1.0, size(180.0, 520.0));
        let surface_biomes = Arc::new(vec![plains, wasteland, ocean]);
        let config = SurfaceFieldConfig::from_biomes(&surface_biomes, Some(2));
        let field = BiomeField {
            surface_biomes,
            volume_biomes: Arc::new(Vec::new()),
            surface_field_config: config,
            volume_site_spacing: None,
            climate: MacroClimateField::new(404),
            seed: 404,
            single_surface_biome: None,
            ocean_surface_index: Some(2),
            spawn_oceans: true,
            spawn_target_surface_biome: None,
        };

        let resolved = field.resolve_surface_adjacency(
            1,
            0.8,
            Some(SurfaceBoundarySample {
                neighbor_surface_index: 2,
                neighbor_terrain_strength: 1.0,
                distance: 8.0,
            }),
        );

        assert_eq!(resolved.primary_index, 0);
        assert_eq!(resolved.primary_terrain_strength, 1.0);
        let boundary = resolved.boundary.unwrap();
        assert_eq!(boundary.neighbor_surface_index, 1);
        assert_eq!(boundary.distance, BORDER_TRANSITION_WIDTH - 8.0);
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
                let scale = land_site_scale(IVec2::new(x, z), &biome, Vec2::splat(250.0), 42);
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
}
