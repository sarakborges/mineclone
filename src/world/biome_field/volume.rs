use bevy::prelude::*;

use super::{
    BiomeField, BiomeFieldEntry, BiomeInfluence, VolumeBiomeAnchor, VolumeBiomeFieldSample,
    constants::{VOLUME_BORDER_MARGIN, VOLUME_SITE_SEARCH_RADIUS},
    selection::{select_tied_volume_index, select_volume_biome_index},
    spatial::{
        hash_unit, lerp, smoothstep, volume_cell_hash, volume_site_position, warp_volume_position,
    },
};

impl BiomeField {
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
                    let Some(candidate_index) =
                        select_volume_biome_index(position.y, climate, hash, &self.volume_biomes)
                    else {
                        continue;
                    };
                    let radii = volume_site_radii(&self.volume_biomes[candidate_index], hash);
                    let normalized_distance = normalized_ellipsoid_distance(warped - site, radii);
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

        Some(VolumeBiomeFieldSample {
            primary_id: self.volume_biomes[primary_index].id.as_str(),
            influences: vec![BiomeInfluence {
                id: self.volume_biomes[primary_index].id.as_str(),
                weight: 1.0,
            }],
            strength: weights[primary_index],
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
                    let Some(index) =
                        select_volume_biome_index(site.y, climate, hash, &self.volume_biomes)
                    else {
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
}
