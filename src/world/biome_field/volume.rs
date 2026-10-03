use bevy::prelude::*;

use crate::content::{
    biome::{BiomeVerticalRange, VolumeSurfaceConstraints},
    biome_density::BiomeDensityModifier,
};

use super::{
    BiomeField, BiomeFieldEntry, VolumeBiomeAnchor,
    constants::{VOLUME_BORDER_MARGIN, VOLUME_SITE_JITTER_FRACTION, VOLUME_WARP_AMPLITUDE},
    selection::select_volume_biome_index,
    spatial::{
        hash_unit, lerp, smoothstep, volume_cell_hash, volume_site_position, warp_volume_position,
    },
};

const FLOATING_ISLAND_SURFACE_FIT_DIRECTIONS: usize = 32;
const FLOATING_ISLAND_SURFACE_FIT_PROBE_STEP: f32 = 8.0;
const FLOATING_ISLAND_SURFACE_FIT_BINARY_STEPS: usize = 6;
const FLOATING_ISLAND_SURFACE_FIT_PADDING: f32 =
    VOLUME_WARP_AMPLITUDE * std::f32::consts::SQRT_2 + 1.0;

#[derive(Clone, Debug, Default)]
pub(crate) struct VolumeBiomeRegion {
    sites: Vec<ResolvedVolumeBiomeSite>,
}

impl VolumeBiomeRegion {
    pub(crate) fn restricted_to_bounds(&self, minimum: Vec3, maximum: Vec3) -> Self {
        if self.sites.is_empty() {
            return Self::default();
        }

        let warp = Vec3::splat(VOLUME_WARP_AMPLITUDE);
        let sample_minimum = minimum - warp;
        let sample_maximum = maximum + warp;
        let sites = self
            .sites
            .iter()
            .copied()
            .filter(|site| {
                let expanded = site.radii * (1.0 + VOLUME_BORDER_MARGIN);
                bounds_intersect(
                    site.position - expanded,
                    site.position + expanded,
                    sample_minimum,
                    sample_maximum,
                )
            })
            .collect();

        Self { sites }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct VolumeBiomeSelection {
    pub(crate) biome_index: usize,
    pub(crate) strength: f32,
    pub(crate) local_position: Vec3,
    pub(crate) vertical_radius: f32,
}

#[derive(Clone, Copy, Debug)]
struct ResolvedVolumeBiomeSite {
    biome_index: usize,
    position: Vec3,
    radii: Vec3,
    priority: i32,
    source_hash: u64,
}

impl BiomeField {
    pub(crate) fn volume_region_in_bounds(
        &self,
        minimum: Vec3,
        maximum: Vec3,
    ) -> VolumeBiomeRegion {
        VolumeBiomeRegion {
            sites: self.resolved_volume_sites_in_bounds(minimum, maximum),
        }
    }

    pub(crate) fn volume_selection_in_region(
        &self,
        position: Vec3,
        region: &VolumeBiomeRegion,
    ) -> Option<VolumeBiomeSelection> {
        let surface = self.sample_surface(Vec2::new(position.x, position.z));
        self.volume_selection_in_region_for_surface(
            position,
            region,
            surface.identity_surface_index,
        )
    }

    pub(crate) fn volume_selection_in_region_for_surface(
        &self,
        position: Vec3,
        region: &VolumeBiomeRegion,
        surface_index: usize,
    ) -> Option<VolumeBiomeSelection> {
        if position.y < 0.0 || region.sites.is_empty() {
            return None;
        }

        let surface = self
            .surface_biomes
            .get(surface_index)
            .unwrap_or_else(|| panic!("surface biome index out of bounds: {surface_index}"));
        let warped = warp_volume_position(position, self.seed);
        let mut selected: Option<(ResolvedVolumeBiomeSite, f32, f32, Vec3)> = None;

        for site in &region.sites {
            let biome = &self.volume_biomes[site.biome_index];
            if !vertical_range_contains(biome.vertical_range, position.y) {
                continue;
            }

            let local_position = normalized_ellipsoid_position(warped - site.position, site.radii);
            let surface_allowed = volume_surface_allows(
                biome.surface_constraints.as_ref(),
                &surface.id,
                &surface.tags,
            );
            if !surface_allowed
                && (!floating_island_uses_site_surface_fit(biome)
                    || local_position.xz().length_squared() > 1.0)
            {
                continue;
            }

            let site_strength = volume_site_strength(local_position.length());
            let selection_strength = site_strength * biome.weight.clamp(0.0, 1.0);
            if selection_strength <= 0.0 {
                continue;
            }

            if selected.is_none_or(|(current, _, current_selection_strength, _)| {
                site_is_better(
                    *site,
                    selection_strength,
                    current,
                    current_selection_strength,
                )
            }) {
                selected = Some((*site, site_strength, selection_strength, local_position));
            }
        }

        selected.map(|(site, strength, _, local_position)| VolumeBiomeSelection {
            biome_index: site.biome_index,
            strength,
            local_position,
            vertical_radius: site.radii.y,
        })
    }

    pub(crate) fn volume_density_modifier(
        &self,
        selection: VolumeBiomeSelection,
    ) -> Option<(BiomeDensityModifier, u64)> {
        let biome = &self.volume_biomes[selection.biome_index];

        biome
            .density_modifier
            .map(|modifier| (modifier, biome.density_seed))
    }

    pub(crate) fn volume_solid_block(&self, selection: VolumeBiomeSelection) -> Option<&str> {
        self.volume_biomes[selection.biome_index]
            .solid_block
            .as_deref()
    }

    pub(crate) fn volume_anchors_in_region<'a>(
        &'a self,
        region: &VolumeBiomeRegion,
    ) -> Vec<VolumeBiomeAnchor<'a>> {
        region
            .sites
            .iter()
            .map(|site| VolumeBiomeAnchor {
                id: self.volume_biomes[site.biome_index].id.as_str(),
                position: site.position,
            })
            .collect()
    }

    fn resolved_volume_sites_in_bounds(
        &self,
        minimum: Vec3,
        maximum: Vec3,
    ) -> Vec<ResolvedVolumeBiomeSite> {
        let Some(spacing) = self.volume_site_spacing else {
            return Vec::new();
        };
        let maximum_radii = maximum_volume_radii(&self.volume_biomes);
        let jitter = spacing * VOLUME_SITE_JITTER_FRACTION;
        let warp = Vec3::splat(VOLUME_WARP_AMPLITUDE);
        let padding = maximum_radii * (1.0 + VOLUME_BORDER_MARGIN) + jitter + warp;
        let minimum_cell = IVec3::new(
            ((minimum.x - padding.x) / spacing.x).floor() as i32 - 1,
            (((minimum.y - padding.y) / spacing.y).floor() as i32 - 1).max(0),
            ((minimum.z - padding.z) / spacing.z).floor() as i32 - 1,
        );
        let maximum_cell = IVec3::new(
            ((maximum.x + padding.x) / spacing.x).ceil() as i32 + 1,
            (((maximum.y + padding.y) / spacing.y).ceil() as i32 + 1).max(0),
            ((maximum.z + padding.z) / spacing.z).ceil() as i32 + 1,
        );
        let sample_minimum = minimum - warp;
        let sample_maximum = maximum + warp;
        let mut sites = Vec::new();

        for y in minimum_cell.y..=maximum_cell.y {
            for z in minimum_cell.z..=maximum_cell.z {
                for x in minimum_cell.x..=maximum_cell.x {
                    let cell = IVec3::new(x, y, z);
                    let site = volume_site_position(cell, spacing, self.seed);
                    let hash = volume_cell_hash(cell, self.seed);
                    let climate = self.climate.sample(Vec2::new(site.x, site.z));
                    let Some(biome_index) =
                        select_volume_biome_index(site.y, climate, hash, &self.volume_biomes)
                    else {
                        continue;
                    };
                    let biome = &self.volume_biomes[biome_index];
                    let radii = volume_site_radii(biome, hash);
                    let Some(radii) = self.fit_floating_island_radii_to_surface(biome, site, radii)
                    else {
                        continue;
                    };
                    let expanded = radii * (1.0 + VOLUME_BORDER_MARGIN);
                    let site_minimum = site - expanded;
                    let site_maximum = site + expanded;

                    if !bounds_intersect(site_minimum, site_maximum, sample_minimum, sample_maximum)
                    {
                        continue;
                    }

                    sites.push(ResolvedVolumeBiomeSite {
                        biome_index,
                        position: site,
                        radii,
                        priority: biome.priority,
                        source_hash: hash,
                    });
                }
            }
        }

        sites
    }

    fn fit_floating_island_radii_to_surface(
        &self,
        biome: &BiomeFieldEntry,
        site: Vec3,
        radii: Vec3,
    ) -> Option<Vec3> {
        if !floating_island_uses_site_surface_fit(biome) {
            return Some(radii);
        }
        let constraints = biome
            .surface_constraints
            .as_ref()
            .expect("site-fitted floating island must define surface constraints");
        let center = site.xz();
        if !self.surface_constraints_allow_at(constraints, center) {
            return None;
        }

        let fit_scale = floating_island_surface_fit_scale(radii.xz(), |direction, max_distance| {
            self.allowed_surface_clearance(center, constraints, direction, max_distance)
        });
        fit_horizontal_radii(
            radii,
            Vec2::new(biome.size.x.min, biome.size.z.min),
            fit_scale,
        )
    }

    fn surface_constraints_allow_at(
        &self,
        constraints: &VolumeSurfaceConstraints,
        position: Vec2,
    ) -> bool {
        let sample = self.sample_surface(position);
        let surface = &self.surface_biomes[sample.identity_surface_index];
        constraints.allows_surface(&surface.id, &surface.tags)
    }

    fn allowed_surface_clearance(
        &self,
        center: Vec2,
        constraints: &VolumeSurfaceConstraints,
        direction: Vec2,
        max_distance: f32,
    ) -> f32 {
        let mut allowed_distance = 0.0;
        let mut probe_distance = FLOATING_ISLAND_SURFACE_FIT_PROBE_STEP.min(max_distance);

        loop {
            if !self.surface_constraints_allow_at(constraints, center + direction * probe_distance)
            {
                let mut low = allowed_distance;
                let mut high = probe_distance;
                for _ in 0..FLOATING_ISLAND_SURFACE_FIT_BINARY_STEPS {
                    let midpoint = (low + high) * 0.5;
                    if self.surface_constraints_allow_at(constraints, center + direction * midpoint)
                    {
                        low = midpoint;
                    } else {
                        high = midpoint;
                    }
                }
                return low;
            }

            allowed_distance = probe_distance;
            if probe_distance >= max_distance {
                return max_distance;
            }
            probe_distance =
                (probe_distance + FLOATING_ISLAND_SURFACE_FIT_PROBE_STEP).min(max_distance);
        }
    }
}

fn floating_island_uses_site_surface_fit(biome: &BiomeFieldEntry) -> bool {
    biome.surface_constraints.is_some()
        && matches!(
            biome.density_modifier,
            Some(BiomeDensityModifier::FloatingIsland { .. })
        )
}

fn floating_island_surface_fit_scale(
    radii: Vec2,
    mut clearance_in_direction: impl FnMut(Vec2, f32) -> f32,
) -> f32 {
    let mut fit_scale = 1.0_f32;

    for index in 0..FLOATING_ISLAND_SURFACE_FIT_DIRECTIONS {
        let angle =
            std::f32::consts::TAU * index as f32 / FLOATING_ISLAND_SURFACE_FIT_DIRECTIONS as f32;
        let direction = Vec2::new(angle.cos(), angle.sin());
        let radius = ellipse_radius_along(radii, direction);
        let max_distance = radius + FLOATING_ISLAND_SURFACE_FIT_PADDING;
        let clearance = clearance_in_direction(direction, max_distance);
        let direction_scale =
            ((clearance - FLOATING_ISLAND_SURFACE_FIT_PADDING) / radius).clamp(0.0, 1.0);
        fit_scale = fit_scale.min(direction_scale);
    }

    fit_scale
}

fn ellipse_radius_along(radii: Vec2, direction: Vec2) -> f32 {
    let normalized = Vec2::new(direction.x / radii.x, direction.y / radii.y);
    normalized.length().recip()
}

fn fit_horizontal_radii(radii: Vec3, minimum: Vec2, fit_scale: f32) -> Option<Vec3> {
    let minimum_scale = (minimum.x / radii.x)
        .max(minimum.y / radii.z)
        .clamp(0.0, 1.0);
    if fit_scale + f32::EPSILON < minimum_scale {
        return None;
    }

    let scale = fit_scale.max(minimum_scale).min(1.0);
    Some(Vec3::new(radii.x * scale, radii.y, radii.z * scale))
}

fn volume_surface_allows(
    constraints: Option<&VolumeSurfaceConstraints>,
    surface_id: &str,
    surface_tags: &[String],
) -> bool {
    constraints.is_none_or(|constraints| constraints.allows_surface(surface_id, surface_tags))
}

fn vertical_range_contains(range: Option<BiomeVerticalRange>, y: f32) -> bool {
    let Some(range) = range else {
        return y >= 0.0;
    };

    y >= range.min && y <= range.max
}

fn site_is_better(
    candidate: ResolvedVolumeBiomeSite,
    candidate_strength: f32,
    current: ResolvedVolumeBiomeSite,
    current_strength: f32,
) -> bool {
    candidate.priority > current.priority
        || (candidate.priority == current.priority
            && (candidate_strength.total_cmp(&current_strength).is_gt()
                || (candidate_strength.total_cmp(&current_strength).is_eq()
                    && candidate.source_hash < current.source_hash)))
}

fn maximum_volume_radii(biomes: &[BiomeFieldEntry]) -> Vec3 {
    biomes
        .iter()
        .filter(|biome| biome.weight > f32::EPSILON)
        .fold(Vec3::ZERO, |maximum, biome| {
            let vertical = biome
                .size
                .y
                .expect("volume biome field entry must define size.y");

            maximum.max(Vec3::new(biome.size.x.max, vertical.max, biome.size.z.max))
        })
}

fn bounds_intersect(
    left_minimum: Vec3,
    left_maximum: Vec3,
    right_minimum: Vec3,
    right_maximum: Vec3,
) -> bool {
    left_maximum.x >= right_minimum.x
        && left_minimum.x <= right_maximum.x
        && left_maximum.y >= right_minimum.y
        && left_minimum.y <= right_maximum.y
        && left_maximum.z >= right_minimum.z
        && left_minimum.z <= right_maximum.z
}

fn volume_site_radii(biome: &BiomeFieldEntry, hash: u64) -> Vec3 {
    let vertical_size = biome
        .size
        .y
        .expect("volume biome field entry must define size.y");

    Vec3::new(
        lerp(
            biome.size.x.min,
            biome.size.x.max,
            hash_unit(hash.rotate_left(7)),
        ),
        lerp(
            vertical_size.min,
            vertical_size.max,
            hash_unit(hash.rotate_left(23)),
        ),
        lerp(
            biome.size.z.min,
            biome.size.z.max,
            hash_unit(hash.rotate_left(41)),
        ),
    )
}

fn normalized_ellipsoid_position(delta: Vec3, radii: Vec3) -> Vec3 {
    Vec3::new(delta.x / radii.x, delta.y / radii.y, delta.z / radii.z)
}

fn volume_site_strength(normalized_distance: f32) -> f32 {
    if normalized_distance <= 1.0 {
        return 1.0;
    }

    if normalized_distance >= 1.0 + VOLUME_BORDER_MARGIN {
        return 0.0;
    }

    let progress = 1.0 - ((normalized_distance - 1.0) / VOLUME_BORDER_MARGIN).clamp(0.0, 1.0);
    smoothstep(progress)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::biome::{SurfaceBiomeSelector, VolumeSurfaceConstraints};

    fn tags(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn volume_strength_is_full_inside_and_fades_outside() {
        assert_eq!(volume_site_strength(0.5), 1.0);
        assert_eq!(volume_site_strength(1.0), 1.0);
        assert!(volume_site_strength(1.1) > 0.0);
        assert_eq!(volume_site_strength(1.25), 0.0);
    }

    #[test]
    fn volume_samples_stay_inside_their_vertical_range() {
        let range = Some(BiomeVerticalRange {
            min: 8.0,
            max: 64.0,
        });

        assert!(vertical_range_contains(range, 8.0));
        assert!(vertical_range_contains(range, 64.0));
        assert!(!vertical_range_contains(range, 7.99));
        assert!(!vertical_range_contains(range, 64.01));
    }

    #[test]
    fn constrained_volume_accepts_allowed_surface_and_rejects_denied_surface() {
        let constraints = VolumeSurfaceConstraints {
            allow: Some(SurfaceBiomeSelector {
                ids: Vec::new(),
                tags: vec!["land".to_string()],
            }),
            deny: Some(SurfaceBiomeSelector {
                ids: vec!["asteria:overworld/wasteland".to_string()],
                tags: Vec::new(),
            }),
        };

        assert!(volume_surface_allows(
            Some(&constraints),
            "asteria:overworld/plains",
            &tags(&["land"]),
        ));
        assert!(!volume_surface_allows(
            Some(&constraints),
            "asteria:overworld/wasteland",
            &tags(&["land"]),
        ));
        assert!(!volume_surface_allows(
            Some(&constraints),
            "asteria:overworld/ocean",
            &tags(&["water"]),
        ));
        assert!(volume_surface_allows(
            None,
            "asteria:overworld/ocean",
            &tags(&["water"]),
        ));
    }

    #[test]
    fn floating_island_surface_fit_uses_the_tightest_direction() {
        let radii = Vec2::new(80.0, 60.0);
        let scale = floating_island_surface_fit_scale(radii, |_direction, max_distance| {
            FLOATING_ISLAND_SURFACE_FIT_PADDING
                + (max_distance - FLOATING_ISLAND_SURFACE_FIT_PADDING) * 0.5
        });

        assert!((scale - 0.5).abs() <= 1e-5);
    }

    #[test]
    fn floating_island_surface_fit_rejects_sizes_below_authored_minimum() {
        let radii = Vec3::new(80.0, 32.0, 60.0);
        let minimum = Vec2::new(48.0, 36.0);

        let fitted = fit_horizontal_radii(radii, minimum, 0.75)
            .expect("fit above authored minimum should remain valid");
        assert_eq!(fitted, Vec3::new(60.0, 32.0, 45.0));
        assert!(fit_horizontal_radii(radii, minimum, 0.5).is_none());
    }

    #[test]
    fn stronger_equal_priority_site_wins_without_allocating_candidates() {
        let current = ResolvedVolumeBiomeSite {
            biome_index: 0,
            position: Vec3::ZERO,
            radii: Vec3::ONE,
            priority: 3,
            source_hash: 20,
        };
        let candidate = ResolvedVolumeBiomeSite {
            biome_index: 1,
            position: Vec3::ZERO,
            radii: Vec3::ONE,
            priority: 3,
            source_hash: 10,
        };

        assert!(site_is_better(candidate, 0.8, current, 0.5));
        assert!(!site_is_better(candidate, 0.2, current, 0.5));
    }

    #[test]
    fn restricted_region_keeps_only_sites_that_can_reach_bounds_with_warp() {
        let region = VolumeBiomeRegion {
            sites: vec![
                ResolvedVolumeBiomeSite {
                    biome_index: 0,
                    position: Vec3::new(20.0, 8.0, 8.0),
                    radii: Vec3::ONE,
                    priority: 0,
                    source_hash: 1,
                },
                ResolvedVolumeBiomeSite {
                    biome_index: 0,
                    position: Vec3::new(100.0, 8.0, 8.0),
                    radii: Vec3::ONE,
                    priority: 0,
                    source_hash: 2,
                },
            ],
        };

        let restricted = region.restricted_to_bounds(Vec3::ZERO, Vec3::splat(16.0));

        assert_eq!(restricted.sites.len(), 1);
        assert_eq!(restricted.sites[0].source_hash, 1);
    }
}
