use bevy::prelude::*;

use crate::content::{
    biome::{
        BiomeClimate, BiomeClimateRange, BiomeKind, BiomeRegistry, BiomeSize,
        BiomeUnderwaterTint, BiomeVerticalRange,
    },
    color::Rgb,
    dimension::DimensionDefinition,
};

use super::macro_climate::{MacroClimateField, MacroClimateSample};

const BORDER_TRANSITION_WIDTH: f32 = 32.0;
const BORDER_WARP_AMPLITUDE: f32 = 24.0;
const SITE_JITTER_FRACTION: f32 = 0.32;
const SITE_SEARCH_RADIUS: i32 = 2;

const CLIMATE_BLEND_MARGIN: f32 = 0.12;

const VOLUME_SITE_GAP: f32 = 24.0;
const VOLUME_SITE_JITTER_FRACTION: f32 = 0.28;
const VOLUME_SITE_SEARCH_RADIUS: i32 = 1;
const VOLUME_BORDER_MARGIN: f32 = 0.25;
const VOLUME_WARP_AMPLITUDE: f32 = 12.0;

#[derive(Clone)]
struct BiomeFieldEntry {
    id: String,
    size: BiomeSize,
    climate: BiomeClimate,
    vertical_range: Option<BiomeVerticalRange>,
    priority: i32,
}

#[derive(Resource)]
pub struct BiomeField {
    surface_biomes: Vec<BiomeFieldEntry>,
    volume_biomes: Vec<BiomeFieldEntry>,
    surface_site_spacing: Vec2,
    volume_site_spacing: Option<Vec3>,
    climate: MacroClimateField,
    seed: u64,
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
                    surface_minimum_radius.x =
                        surface_minimum_radius.x.max(biome.size.x.min);
                    surface_minimum_radius.y =
                        surface_minimum_radius.y.max(biome.size.z.min);
                    surface_biomes.push(entry);
                }
                BiomeKind::Volume => {
                    let vertical_size = biome.size.y.unwrap_or_else(|| {
                        panic!("volume biome {} must define size.y", biome.id)
                    });
                    volume_minimum_radius.x =
                        volume_minimum_radius.x.max(biome.size.x.min);
                    volume_minimum_radius.y =
                        volume_minimum_radius.y.max(vertical_size.min);
                    volume_minimum_radius.z =
                        volume_minimum_radius.z.max(biome.size.z.min);
                    volume_biomes.push(entry);
                }
            }
        }

        assert!(
            !surface_biomes.is_empty(),
            "dimension {} must define at least one surface biome",
            dimension.id
        );

        let border_allowance = BORDER_TRANSITION_WIDTH + BORDER_WARP_AMPLITUDE * 2.0;
        let surface_site_spacing =
            surface_minimum_radius * 2.0 + Vec2::splat(border_allowance);
        let volume_site_spacing = (!volume_biomes.is_empty()).then_some(
            volume_minimum_radius * 2.0 + Vec3::splat(VOLUME_SITE_GAP),
        );

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

    pub fn sample_surface(&self, position: Vec2) -> BiomeFieldSample<'_> {
        if self.surface_biomes.len() == 1 {
            let id = self.surface_biomes[0].id.as_str();
            return BiomeFieldSample {
                primary_id: id,
                influences: vec![BiomeInfluence { id, weight: 1.0 }],
            };
        }

        let warped = warp_surface_position(position, self.seed);
        let center = IVec2::new(
            (warped.x / self.surface_site_spacing.x).round() as i32,
            (warped.y / self.surface_site_spacing.y).round() as i32,
        );
        let mut sites = Vec::new();
        let mut nearest_distance = f32::MAX;
        let mut primary_index = 0;

        for z in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
            for x in -SITE_SEARCH_RADIUS..=SITE_SEARCH_RADIUS {
                let cell = center + IVec2::new(x, z);
                let site = surface_site_position(cell, self.surface_site_spacing, self.seed);
                let distance = warped.distance(site);
                let candidate_index = select_surface_biome_index(
                    cell,
                    site,
                    &self.surface_biomes,
                    &self.climate,
                    self.seed,
                );

                if distance < nearest_distance {
                    nearest_distance = distance;
                    primary_index = candidate_index;
                }

                sites.push((candidate_index, distance));
            }
        }

        let mut weights = vec![0.0_f32; self.surface_biomes.len()];

        for (candidate_index, distance) in sites {
            let distance_gap = (distance - nearest_distance).max(0.0);
            let border_progress =
                1.0 - (distance_gap / BORDER_TRANSITION_WIDTH).clamp(0.0, 1.0);
            let smooth_progress = smoothstep(border_progress);
            let previous_weight = weights[candidate_index];

            weights[candidate_index] = previous_weight.max(smooth_progress);
        }

        let total_weight: f32 = weights.iter().sum();
        let influences = weights
            .into_iter()
            .enumerate()
            .filter_map(|(index, weight)| {
                (weight > 0.0).then_some(BiomeInfluence {
                    id: self.surface_biomes[index].id.as_str(),
                    weight: weight / total_weight,
                })
            })
            .collect();

        BiomeFieldSample {
            primary_id: self.surface_biomes[primary_index].id.as_str(),
            influences,
        }
    }

    pub fn sample_volume(&self, position: Vec3) -> Option<VolumeBiomeFieldSample<'_>> {
        if position.y < 0.0 || self.volume_biomes.is_empty() {
            return None;
        }

        let spacing = self
            .volume_site_spacing
            .expect("volume biome spacing should exist when volume biomes exist");
        let warped = warp_volume_position(position, self.seed);
        let center = IVec3::new(
            (warped.x / spacing.x).round() as i32,
            ((warped.y / spacing.y).round() as i32).max(0),
            (warped.z / spacing.z).round() as i32,
        );
        let climate = self.climate.sample(Vec2::new(position.x, position.z));
        let mut weights = vec![0.0_f32; self.volume_biomes.len()];
        let mut source_hashes = vec![None; self.volume_biomes.len()];

        for y in -VOLUME_SITE_SEARCH_RADIUS..=VOLUME_SITE_SEARCH_RADIUS {
            for z in -VOLUME_SITE_SEARCH_RADIUS..=VOLUME_SITE_SEARCH_RADIUS {
                for x in -VOLUME_SITE_SEARCH_RADIUS..=VOLUME_SITE_SEARCH_RADIUS {
                    let cell = center + IVec3::new(x, y, z);

                    if cell.y < 0 {
                        continue;
                    }

                    let site = volume_site_position(cell, spacing, self.seed);
                    let hash = volume_cell_hash(cell, self.seed);
                    let Some(candidate_index) = select_volume_biome_index(
                        position.y,
                        climate,
                        hash,
                        &self.volume_biomes,
                    ) else {
                        continue;
                    };
                    let radii = volume_site_radii(&self.volume_biomes[candidate_index], hash);
                    let normalized_distance =
                        normalized_ellipsoid_distance(warped - site, radii);
                    let strength = volume_site_strength(normalized_distance);

                    if strength > weights[candidate_index] {
                        weights[candidate_index] = strength;
                        source_hashes[candidate_index] = Some(hash);
                    }
                }
            }
        }

        let winning_priority = weights
            .iter()
            .enumerate()
            .filter_map(|(index, weight)| {
                (*weight > 0.0).then_some(self.volume_biomes[index].priority)
            })
            .max()?;
        let tied_indices = weights
            .iter()
            .enumerate()
            .filter_map(|(index, weight)| {
                (*weight > 0.0 && self.volume_biomes[index].priority == winning_priority)
                    .then_some(index)
            })
            .collect::<Vec<_>>();
        let primary_index = select_tied_volume_index(&tied_indices, &source_hashes, self.seed)?;
        let overlay_strength = weights[primary_index];

        Some(VolumeBiomeFieldSample {
            primary_id: self.volume_biomes[primary_index].id.as_str(),
            influences: vec![BiomeInfluence {
                id: self.volume_biomes[primary_index].id.as_str(),
                weight: 1.0,
            }],
            strength: overlay_strength,
        })
    }

    pub(crate) fn volume_anchors_in_bounds(
        &self,
        minimum: Vec3,
        maximum: Vec3,
    ) -> Vec<VolumeBiomeAnchor<'_>> {
        let Some(spacing) = self.volume_site_spacing else {
            return Vec::new();
        };

        let minimum_cell = IVec3::new(
            (minimum.x / spacing.x).floor() as i32 - 1,
            ((minimum.y / spacing.y).floor() as i32 - 1).max(0),
            (minimum.z / spacing.z).floor() as i32 - 1,
        );
        let maximum_cell = IVec3::new(
            (maximum.x / spacing.x).ceil() as i32 + 1,
            ((maximum.y / spacing.y).ceil() as i32 + 1).max(0),
            (maximum.z / spacing.z).ceil() as i32 + 1,
        );
        let mut anchors = Vec::new();

        for y in minimum_cell.y..=maximum_cell.y {
            for z in minimum_cell.z..=maximum_cell.z {
                for x in minimum_cell.x..=maximum_cell.x {
                    let cell = IVec3::new(x, y, z);
                    let site = volume_site_position(cell, spacing, self.seed);
                    let hash = volume_cell_hash(cell, self.seed);
                    let climate = self.climate.sample(Vec2::new(site.x, site.z));
                    let Some(index) = select_volume_biome_index(
                        site.y,
                        climate,
                        hash,
                        &self.volume_biomes,
                    ) else {
                        continue;
                    };
                    let radii = volume_site_radii(&self.volume_biomes[index], hash);
                    let expanded = radii * (1.0 + VOLUME_BORDER_MARGIN);
                    let site_minimum = site - expanded;
                    let site_maximum = site + expanded;
                    let intersects = site_maximum.x >= minimum.x
                        && site_minimum.x <= maximum.x
                        && site_maximum.y >= minimum.y
                        && site_minimum.y <= maximum.y
                        && site_maximum.z >= minimum.z
                        && site_minimum.z <= maximum.z;

                    if intersects {
                        anchors.push(VolumeBiomeAnchor {
                            id: self.volume_biomes[index].id.as_str(),
                            position: site,
                        });
                    }
                }
            }
        }

        anchors
    }

    pub fn sample_resolved(&self, position: Vec3) -> ResolvedBiomeFieldSample<'_> {
        let surface = self.sample_surface(Vec2::new(position.x, position.z));
        let volume = self.sample_volume(position);

        let (primary_id, influences) = if let Some(volume_sample) = &volume {
            let volume_strength = volume_sample.strength.clamp(0.0, 1.0);
            let surface_strength = 1.0 - volume_strength;
            let mut influences = Vec::with_capacity(
                surface.influences.len() + volume_sample.influences.len(),
            );

            influences.extend(surface.influences.iter().filter_map(|influence| {
                let weight = influence.weight * surface_strength;
                (weight > 0.0).then_some(BiomeInfluence {
                    id: influence.id,
                    weight,
                })
            }));
            influences.extend(volume_sample.influences.iter().filter_map(|influence| {
                let weight = influence.weight * volume_strength;
                (weight > 0.0).then_some(BiomeInfluence {
                    id: influence.id,
                    weight,
                })
            }));

            let primary_id = if volume_strength >= 0.5 {
                volume_sample.primary_id
            } else {
                surface.primary_id
            };

            (primary_id, influences)
        } else {
            (surface.primary_id, surface.influences.clone())
        };

        ResolvedBiomeFieldSample {
            primary_id,
            influences,
            surface,
            volume,
        }
    }

    pub fn grass_color(&self, position: Vec2, biomes: &BiomeRegistry) -> Rgb {
        let sample = self.sample_surface(position);
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

    pub fn underwater_tint(
        &self,
        position: Vec3,
        biomes: &BiomeRegistry,
    ) -> BiomeUnderwaterTint {
        let sample = self.sample_resolved(position);
        let mut color = Rgb {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        };
        let mut opacity = 0.0;

        for influence in sample.influences {
            let biome = biomes
                .get(influence.id)
                .unwrap_or_else(|| panic!("missing biome definition: {}", influence.id));
            let tint = biome.visuals.underwater_tint;

            color.r += tint.color.r * influence.weight;
            color.g += tint.color.g * influence.weight;
            color.b += tint.color.b * influence.weight;
            opacity += tint.opacity * influence.weight;
        }

        BiomeUnderwaterTint { color, opacity }
    }
}

fn select_surface_biome_index(
    cell: IVec2,
    site: Vec2,
    biomes: &[BiomeFieldEntry],
    climate_field: &MacroClimateField,
    seed: u64,
) -> usize {
    if biomes.iter().all(|biome| climate_is_unrestricted(biome.climate)) {
        return biome_index(cell, biomes.len(), seed);
    }

    let climate = climate_field.sample(site);
    let hash = cell_hash(cell, seed);
    select_weighted_biome_index(biomes, climate, hash, |_| true)
        .unwrap_or_else(|| biome_index(cell, biomes.len(), seed))
}

fn select_volume_biome_index(
    world_y: f32,
    climate: MacroClimateSample,
    hash: u64,
    biomes: &[BiomeFieldEntry],
) -> Option<usize> {
    select_weighted_biome_index(biomes, climate, hash, |biome| {
        vertical_range_contains(biome.vertical_range, world_y)
    })
}

fn select_tied_volume_index(
    indices: &[usize],
    source_hashes: &[Option<u64>],
    seed: u64,
) -> Option<usize> {
    if indices.is_empty() {
        return None;
    }

    if indices.len() == 1 {
        return Some(indices[0]);
    }

    let mut candidates = indices
        .iter()
        .map(|index| (*index, source_hashes[*index].unwrap_or_default()))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(_, source_hash)| *source_hash);

    let mut hash = seed ^ 0x6a09_e667_f3bc_c909;

    for (_, source_hash) in &candidates {
        hash ^= source_hash.wrapping_mul(0x9e37_79b1_85eb_ca87);
        hash ^= hash >> 33;
        hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
        hash ^= hash >> 33;
    }

    Some(candidates[hash as usize % candidates.len()].0)
}

fn select_weighted_biome_index(
    biomes: &[BiomeFieldEntry],
    climate: MacroClimateSample,
    hash: u64,
    predicate: impl Fn(&BiomeFieldEntry) -> bool,
) -> Option<usize> {
    let eligible = biomes
        .iter()
        .enumerate()
        .filter_map(|(index, biome)| predicate(biome).then_some(index))
        .collect::<Vec<_>>();

    if eligible.is_empty() {
        return None;
    }

    if eligible
        .iter()
        .all(|index| climate_is_unrestricted(biomes[*index].climate))
    {
        return Some(eligible[hash as usize % eligible.len()]);
    }

    let weighted = eligible
        .iter()
        .map(|index| {
            (
                *index,
                climate_suitability(biomes[*index].climate, climate),
            )
        })
        .collect::<Vec<_>>();
    let total_weight: f32 = weighted.iter().map(|(_, weight)| *weight).sum();

    if total_weight <= f32::EPSILON {
        return Some(eligible[hash as usize % eligible.len()]);
    }

    let mut selector = hash_unit(hash.rotate_left(17)) * total_weight;

    for (index, weight) in weighted {
        selector -= weight;

        if selector <= 0.0 {
            return Some(index);
        }
    }

    eligible.last().copied()
}

fn climate_suitability(profile: BiomeClimate, climate: MacroClimateSample) -> f32 {
    climate_axis_suitability(profile.temperature, climate.temperature)
        * climate_axis_suitability(profile.humidity, climate.humidity)
        * climate_axis_suitability(profile.continentalness, climate.continentalness)
        * climate_axis_suitability(profile.erosion, climate.erosion)
}

fn climate_axis_suitability(range: Option<BiomeClimateRange>, value: f32) -> f32 {
    let Some(range) = range else {
        return 1.0;
    };

    if (range.min..=range.max).contains(&value) {
        return 1.0;
    }

    let distance = if value < range.min {
        range.min - value
    } else {
        value - range.max
    };

    1.0 - (distance / CLIMATE_BLEND_MARGIN).clamp(0.0, 1.0)
}

fn climate_is_unrestricted(climate: BiomeClimate) -> bool {
    climate.temperature.is_none()
        && climate.humidity.is_none()
        && climate.continentalness.is_none()
        && climate.erosion.is_none()
}

fn vertical_range_contains(range: Option<BiomeVerticalRange>, y: f32) -> bool {
    let Some(range) = range else {
        return y >= 0.0;
    };

    y >= range.min && y <= range.max
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
    let normalized = Vec3::new(
        delta.x / radii.x,
        delta.y / radii.y,
        delta.z / radii.z,
    );

    (normalized.x * normalized.x
        + normalized.y * normalized.y
        + normalized.z * normalized.z)
        .sqrt()
}

fn volume_site_strength(normalized_distance: f32) -> f32 {
    if normalized_distance <= 1.0 {
        return 1.0;
    }

    if normalized_distance >= 1.0 + VOLUME_BORDER_MARGIN {
        return 0.0;
    }

    let progress =
        1.0 - ((normalized_distance - 1.0) / VOLUME_BORDER_MARGIN).clamp(0.0, 1.0);

    smoothstep(progress)
}

fn warp_surface_position(position: Vec2, seed: u64) -> Vec2 {
    let phase_x = hash_component(seed) * std::f32::consts::TAU;
    let phase_z = hash_component(seed.rotate_left(31)) * std::f32::consts::TAU;

    position
        + Vec2::new(
            (position.y * 0.011 + phase_x).sin() * BORDER_WARP_AMPLITUDE,
            (position.x * 0.009 + phase_z).sin() * BORDER_WARP_AMPLITUDE,
        )
}

fn warp_volume_position(position: Vec3, seed: u64) -> Vec3 {
    let phase_x = hash_component(seed.rotate_left(5)) * std::f32::consts::TAU;
    let phase_y = hash_component(seed.rotate_left(19)) * std::f32::consts::TAU;
    let phase_z = hash_component(seed.rotate_left(37)) * std::f32::consts::TAU;

    position
        + Vec3::new(
            ((position.y + position.z) * 0.008 + phase_x).sin() * VOLUME_WARP_AMPLITUDE,
            ((position.x + position.z) * 0.006 + phase_y).sin() * VOLUME_WARP_AMPLITUDE,
            ((position.x + position.y) * 0.008 + phase_z).sin() * VOLUME_WARP_AMPLITUDE,
        )
}

fn surface_site_position(cell: IVec2, spacing: Vec2, seed: u64) -> Vec2 {
    let base = Vec2::new(cell.x as f32 * spacing.x, cell.y as f32 * spacing.y);

    if cell == IVec2::ZERO {
        return base;
    }

    let hash = cell_hash(cell, seed);
    let jitter_x = hash_component(hash) * spacing.x * SITE_JITTER_FRACTION;
    let jitter_z = hash_component(hash.rotate_left(29)) * spacing.y * SITE_JITTER_FRACTION;

    base + Vec2::new(jitter_x, jitter_z)
}

fn volume_site_position(cell: IVec3, spacing: Vec3, seed: u64) -> Vec3 {
    let base = Vec3::new(
        cell.x as f32 * spacing.x,
        cell.y as f32 * spacing.y,
        cell.z as f32 * spacing.z,
    );

    if cell == IVec3::ZERO {
        return base;
    }

    let hash = volume_cell_hash(cell, seed);
    let jitter = Vec3::new(
        hash_component(hash) * spacing.x * VOLUME_SITE_JITTER_FRACTION,
        hash_component(hash.rotate_left(21)) * spacing.y * VOLUME_SITE_JITTER_FRACTION,
        hash_component(hash.rotate_left(43)) * spacing.z * VOLUME_SITE_JITTER_FRACTION,
    );
    let mut position = base + jitter;

    position.y = position.y.max(0.0);
    position
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

fn volume_cell_hash(cell: IVec3, seed: u64) -> u64 {
    let mut hash = seed ^ 0xe703_7ed1_a0b4_28db;
    hash ^= (cell.x as i64 as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
    hash ^= (cell.y as i64 as u64).wrapping_mul(0xd6e8_feb8_6659_fd93);
    hash ^= (cell.z as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    hash ^= hash >> 32;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 29;
    hash
}

fn hash_component(hash: u64) -> f32 {
    hash_unit(hash) * 2.0 - 1.0
}

fn hash_unit(hash: u64) -> f32 {
    (hash & 0xffff) as f32 / u16::MAX as f32
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

fn lerp(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
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
    fn vertical_ranges_never_include_negative_world_y() {
        assert!(!vertical_range_contains(None, -1.0));
        assert!(!vertical_range_contains(
            Some(BiomeVerticalRange {
                min: 10.0,
                max: 20.0,
            }),
            -1.0,
        ));
    }

    #[test]
    fn higher_priority_volume_weights_suppress_lower_priority_weights() {
        let biomes = [
            BiomeFieldEntry {
                id: "low".into(),
                size: BiomeSize {
                    x: crate::content::biome::BiomeSizeAxis { min: 1.0, max: 1.0 },
                    y: Some(crate::content::biome::BiomeSizeAxis { min: 1.0, max: 1.0 }),
                    z: crate::content::biome::BiomeSizeAxis { min: 1.0, max: 1.0 },
                },
                climate: BiomeClimate::default(),
                vertical_range: None,
                priority: 0,
            },
            BiomeFieldEntry {
                id: "high".into(),
                size: BiomeSize {
                    x: crate::content::biome::BiomeSizeAxis { min: 1.0, max: 1.0 },
                    y: Some(crate::content::biome::BiomeSizeAxis { min: 1.0, max: 1.0 }),
                    z: crate::content::biome::BiomeSizeAxis { min: 1.0, max: 1.0 },
                },
                climate: BiomeClimate::default(),
                vertical_range: None,
                priority: 5,
            },
        ];
        let weights = [0.9_f32, 0.4_f32];
        let winning_priority = weights
            .iter()
            .enumerate()
            .filter_map(|(index, weight)| (*weight > 0.0).then_some(biomes[index].priority))
            .max()
            .unwrap();
        let tied = weights
            .iter()
            .enumerate()
            .filter_map(|(index, weight)| {
                (*weight > 0.0 && biomes[index].priority == winning_priority).then_some(index)
            })
            .collect::<Vec<_>>();

        assert_eq!(tied, vec![1]);
    }

    #[test]
    fn equal_priority_volume_tiebreak_is_seeded_and_deterministic() {
        let tied = [0_usize, 1, 2];
        let source_hashes = [Some(11_u64), Some(29_u64), Some(47_u64)];
        let first = select_tied_volume_index(&tied, &source_hashes, 12345).unwrap();
        let second = select_tied_volume_index(&tied, &source_hashes, 12345).unwrap();

        assert_eq!(first, second);
        assert!(tied.contains(&first));
    }
}
