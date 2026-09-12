use bevy::prelude::*;

use super::HydrologyRegion;
use crate::world::hydrology::{
    constants::{
        BED_MATERIAL_DEPTH, COAST_MAXIMUM_SURFACE_HEIGHT, OCEAN_EXTRA_DEPTH,
        OCEAN_MINIMUM_DEPTH, SHORE_STRENGTH,
    },
    math::{hydrology_dominates_surface, lerp, ocean_strength, smoothstep},
};

#[derive(Clone, Copy, Debug)]
struct BedMaterial<'a> {
    bed: f32,
    material: Option<&'a str>,
}

impl<'a> BedMaterial<'a> {
    fn at(self, y: f32) -> Option<&'a str> {
        if is_near_bed(y, self.bed) {
            self.material
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct MaterialColumnProfile<'a> {
    water_body: Option<BedMaterial<'a>>,
    river: Option<BedMaterial<'a>>,
    ocean: Option<BedMaterial<'a>>,
}

impl<'a> MaterialColumnProfile<'a> {
    fn material_at(self, y: f32) -> Option<&'a str> {
        self.water_body
            .and_then(|material| material.at(y))
            .or_else(|| self.river.and_then(|material| material.at(y)))
            .or_else(|| self.ocean.and_then(|material| material.at(y)))
    }
}

impl HydrologyRegion {
    pub(crate) fn solid_blocks_for_column<const N: usize>(
        &self,
        horizontal: Vec2,
        first_y: f32,
    ) -> [Option<&str>; N] {
        let profile = self.material_column_profile(horizontal);

        std::array::from_fn(|index| profile.material_at(first_y + index as f32))
    }

    fn material_column_profile(&self, horizontal: Vec2) -> MaterialColumnProfile<'_> {
        let water_body = self
            .water_bodies
            .iter()
            .filter_map(|body| {
                let strength = body.horizontal_strength(horizontal);
                (strength > 0.0).then_some((body, strength))
            })
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .map(|(body, strength)| BedMaterial {
                bed: body.water_level - body.carve_depth * strength,
                material: self.lake_bed_material(strength),
            });

        let river = self.river_graph.sample_horizontal(horizontal).map(|river| {
            let profile = smoothstep(river.strength);
            let material = if river.strength <= SHORE_STRENGTH {
                self.settings
                    .shore_block
                    .as_deref()
                    .or(self.settings.river_bed_block.as_deref())
            } else {
                self.settings
                    .river_bed_block
                    .as_deref()
                    .or(self.settings.shore_block.as_deref())
            };

            BedMaterial {
                bed: river.height - self.river_carve_depth * profile,
                material,
            }
        });

        let ocean = self.macro_sample_at(horizontal).and_then(|sample| {
            let strength = ocean_strength(sample.continentalness, self.ocean_weight);
            if strength <= 0.0 || !hydrology_dominates_surface(strength) {
                return None;
            }

            let target_floor =
                self.sea_level - OCEAN_MINIMUM_DEPTH - OCEAN_EXTRA_DEPTH * strength;
            let floor = lerp(sample.elevation, target_floor, strength);
            if floor > self.sea_level + COAST_MAXIMUM_SURFACE_HEIGHT {
                return None;
            }

            let material = if strength <= SHORE_STRENGTH {
                self.settings
                    .shore_block
                    .as_deref()
                    .or(self.settings.ocean_bed_block.as_deref())
            } else {
                self.settings
                    .ocean_bed_block
                    .as_deref()
                    .or(self.settings.shore_block.as_deref())
            };

            Some(BedMaterial {
                bed: floor,
                material,
            })
        });

        MaterialColumnProfile {
            water_body,
            river,
            ocean,
        }
    }

    fn lake_bed_material(&self, strength: f32) -> Option<&str> {
        let bed = self.settings.lake_bed_block.as_deref();

        if strength <= SHORE_STRENGTH {
            self.settings.shore_block.as_deref().or(bed)
        } else {
            bed.or(self.settings.shore_block.as_deref())
        }
    }
}

fn is_near_bed(y: f32, bed: f32) -> bool {
    y >= bed - BED_MATERIAL_DEPTH && y <= bed + BED_MATERIAL_DEPTH
}
