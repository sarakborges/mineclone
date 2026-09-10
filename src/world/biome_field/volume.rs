use bevy::prelude::*;

use crate::content::{
    biome::BiomeVerticalRange,
    biome_density::BiomeDensityModifier,
};

use super::{
    BiomeField, BiomeFieldEntry, VolumeBiomeAnchor, VolumeBiomeFieldSample,
    constants::{VOLUME_BORDER_MARGIN, VOLUME_SITE_JITTER_FRACTION, VOLUME_WARP_AMPLITUDE},
    selection::select_volume_biome_index,
    spatial::{
        hash_unit, lerp, smoothstep, volume_cell_hash, volume_site_position, warp_volume_position,
    },
};

#[derive(Clone, Debug, Default)]
pub(crate) struct VolumeBiomeRegion {
    sites: Vec<ResolvedVolumeBiomeSite>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct VolumeBiomeSelection {
    pub(crate) biome_index: usize,
    pub(crate) strength: f32,
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

    pub(crate) fn sample_volume_in_region<'a>(
        &'a self,
        position: Vec3,
        region: &VolumeBiomeRegion,
    ) -> Option<VolumeBiomeFieldSample<'a>> {
        let selection = self.volume_selection_in_region(position, region)?;

        Some(VolumeBiomeFieldSample {
            primary_id: self.volume_biomes[selection.biome_index].id.as_str(),
            strength: selection.strength,
        })
    }

    pub(crate) fn volume_selection_in_region(
        &self,
        position: Vec3,
        region: &VolumeBiomeRegion,
    ) -> Option<VolumeBiomeSelection> {
        if position.y < 0.0 || region.sites.is_empty() {
            return None;
        }

        let warped = warp_volume_position(position, self.seed);
        let mut selected: Option<(ResolvedVolumeBiomeSite, f32)> = None;

        for site in &region.sites {
            let biome = &self.volume_biomes[site.biome_index];
            if !vertical_range_contains(biome.vertical_range, position.y) {
                continue;
            }

            let normalized_distance =
                normalized_ellipsoid_distance(warped - site.position, site.radii);
            let strength = volume_site_strength(normalized_distance);
            if strength <= 0.0 {
                continue;
            }

            if selected.is_none_or(|(current, current_strength)| {
                site_is_better(*site, strength, current, current_strength)
            }) {
                selected = Some((*site, strength));
            }
        }

        selected.map(|(site, strength)| VolumeBiomeSelection {
            biome_index: site.biome_index,
            strength,
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

    pub(crate) fn volume_anchors_in_bounds(
        &self,
        minimum: Vec3,
        maximum: Vec3,
    ) -> Vec<VolumeBiomeAnchor<'_>> {
        self.resolved_volume_sites_in_bounds(minimum, maximum)
            .into_iter()
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
    biomes.iter().fold(Vec3::ZERO, |maximum, biome| {
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

fn normalized_ellipsoid_distance(delta: Vec3, radii: Vec3) -> f32 {
    let normalized = Vec3::new(delta.x / radii.x, delta.y / radii.y, delta.z / radii.z);

    (normalized.x * normalized.x + normalized.y * normalized.y + normalized.z * normalized.z).sqrt()
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
}
