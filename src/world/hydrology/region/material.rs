use bevy::prelude::*;

use super::HydrologyRegion;
use crate::world::hydrology::{
    constants::{BED_MATERIAL_DEPTH, COAST_MAXIMUM_SURFACE_HEIGHT, SHORE_STRENGTH},
    math::{hydrology_dominates_surface, lerp, ocean_strength, smoothstep},
};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct HydrologyMaterialSet<'a> {
    pub(crate) river_bed_block: Option<&'a str>,
    pub(crate) lake_bed_block: Option<&'a str>,
    pub(crate) inland_shore_block: Option<&'a str>,
    pub(crate) ocean_bed_block: Option<&'a str>,
    pub(crate) coast_shore_block: Option<&'a str>,
}

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
    pub(crate) fn solid_blocks_for_column<'a, const N: usize>(
        &self,
        horizontal: Vec2,
        first_y: f32,
        materials: HydrologyMaterialSet<'a>,
    ) -> [Option<&'a str>; N] {
        let profile = self.material_column_profile(horizontal, materials);

        std::array::from_fn(|index| profile.material_at(first_y + index as f32))
    }

    fn material_column_profile<'a>(
        &self,
        horizontal: Vec2,
        materials: HydrologyMaterialSet<'a>,
    ) -> MaterialColumnProfile<'a> {
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
                material: lake_bed_material(strength, materials),
            });

        let river = self.river_graph.sample_horizontal(horizontal).map(|river| {
            let profile = smoothstep(river.strength);
            let material = if river.strength <= SHORE_STRENGTH {
                materials
                    .inland_shore_block
                    .or(materials.river_bed_block)
            } else {
                materials
                    .river_bed_block
                    .or(materials.inland_shore_block)
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

            let target_floor = self.ocean_floor_target(horizontal, strength);
            let floor = lerp(sample.elevation, target_floor, strength);
            if floor > self.sea_level + COAST_MAXIMUM_SURFACE_HEIGHT {
                return None;
            }

            let material = if strength <= SHORE_STRENGTH {
                materials
                    .coast_shore_block
                    .or(materials.ocean_bed_block)
            } else {
                materials
                    .ocean_bed_block
                    .or(materials.coast_shore_block)
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
}

fn lake_bed_material(strength: f32, materials: HydrologyMaterialSet<'_>) -> Option<&str> {
    if strength <= SHORE_STRENGTH {
        materials
            .inland_shore_block
            .or(materials.lake_bed_block)
    } else {
        materials
            .lake_bed_block
            .or(materials.inland_shore_block)
    }
}

fn is_near_bed(y: f32, bed: f32) -> bool {
    y >= bed - BED_MATERIAL_DEPTH && y <= bed + BED_MATERIAL_DEPTH
}
