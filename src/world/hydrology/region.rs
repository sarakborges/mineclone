use bevy::prelude::*;

use crate::{content::dimension_hydrology::DimensionHydrology, world::feature_graph::FeatureGraph};

use super::{
    constants::{
        BED_MATERIAL_DEPTH, HYDROLOGY_REGION_SIZE, MACRO_SAMPLE_GRID, OCEAN_EXTRA_DEPTH,
        OCEAN_MINIMUM_DEPTH, RIVER_CARVE_DEPTH, RIVER_CARVE_STRENGTH, SHORE_STRENGTH,
    },
    math::{lerp, ocean_strength, smoothstep},
    types::{
        HydrologyMacroSample, HydrologyTerrainSummary, HydrologyWaterSample, WaterBody,
        WaterBodyKind,
    },
};

#[derive(Clone, Debug)]
pub struct HydrologyRegion {
    pub coord: IVec2,
    pub terrain: HydrologyTerrainSummary,
    pub river_graph: FeatureGraph,
    pub river_carve_depth: f32,
    pub water_bodies: Vec<WaterBody>,
    pub(super) sea_level: f32,
    pub(super) settings: DimensionHydrology,
    pub(super) macro_samples: Vec<HydrologyMacroSample>,
}

impl HydrologyRegion {
    pub(super) fn empty(coord: IVec2, sea_level: f32, settings: DimensionHydrology) -> Self {
        Self {
            coord,
            terrain: HydrologyTerrainSummary::default(),
            river_graph: FeatureGraph::default(),
            river_carve_depth: RIVER_CARVE_DEPTH,
            water_bodies: Vec::new(),
            sea_level,
            settings,
            macro_samples: Vec::new(),
        }
    }

    pub fn density_delta(&self, position: Vec3) -> f32 {
        let horizontal = Vec2::new(position.x, position.z);
        let ocean_delta = self.ocean_density_delta(horizontal);
        let river_delta = self
            .river_graph
            .sample_horizontal(horizontal)
            .map_or(0.0, |sample| {
                let profile = smoothstep(sample.strength);
                let bed = sample.height - self.river_carve_depth * profile;

                if position.y < bed - 0.5 || position.y > sample.height + 1.5 {
                    return 0.0;
                }

                -RIVER_CARVE_STRENGTH * profile
            });
        let water_body_delta = self
            .water_bodies
            .iter()
            .map(|body| {
                if body.carve_depth <= 0.0 {
                    return 0.0;
                }

                let strength = body.horizontal_strength(horizontal);
                let bottom = body.water_level - body.carve_depth * strength;

                if strength <= 0.0
                    || position.y < bottom - 0.5
                    || position.y > body.water_level + 0.5
                {
                    return 0.0;
                }

                -(body.carve_depth + 2.0) * strength
            })
            .sum::<f32>();

        ocean_delta + river_delta + water_body_delta
    }

    pub fn water_at(&self, position: Vec2) -> Option<HydrologyWaterSample<'_>> {
        let mut selected = self
            .water_bodies
            .iter()
            .filter(|body| body.contains_horizontal(position))
            .max_by(|left, right| left.water_level.total_cmp(&right.water_level))
            .map(|body| HydrologyWaterSample {
                fluid_id: body.fluid_id.as_str(),
                water_level: body.water_level,
            });

        if let Some(river) = self.river_graph.sample_horizontal(position) {
            if river.strength > 0.0 {
                let candidate = HydrologyWaterSample {
                    fluid_id: self.settings.water_fluid.as_str(),
                    water_level: river.height,
                };

                if selected
                    .as_ref()
                    .is_none_or(|current| candidate.water_level > current.water_level)
                {
                    selected = Some(candidate);
                }
            }
        }

        if self.ocean_strength_at(position) > 0.0 {
            let candidate = HydrologyWaterSample {
                fluid_id: self.settings.water_fluid.as_str(),
                water_level: self.sea_level,
            };

            if selected
                .as_ref()
                .is_none_or(|current| candidate.water_level > current.water_level)
            {
                selected = Some(candidate);
            }
        }

        selected
    }

    pub fn solid_block_at(&self, position: Vec3) -> Option<&str> {
        let horizontal = Vec2::new(position.x, position.z);

        if let Some((body, strength)) = self
            .water_bodies
            .iter()
            .filter_map(|body| {
                let strength = body.horizontal_strength(horizontal);
                (strength > 0.0).then_some((body, strength))
            })
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
        {
            let bottom = body.water_level - body.carve_depth * strength;

            if position.y >= bottom - BED_MATERIAL_DEPTH
                && position.y <= bottom + BED_MATERIAL_DEPTH
            {
                return self.material_for_water_body(body.kind, strength);
            }
        }

        if let Some(river) = self.river_graph.sample_horizontal(horizontal) {
            let profile = smoothstep(river.strength);
            let bed = river.height - self.river_carve_depth * profile;

            if position.y >= bed - BED_MATERIAL_DEPTH
                && position.y <= bed + BED_MATERIAL_DEPTH
            {
                if river.strength <= SHORE_STRENGTH {
                    return self
                        .settings
                        .shore_block
                        .as_deref()
                        .or(self.settings.river_bed_block.as_deref());
                }

                return self
                    .settings
                    .river_bed_block
                    .as_deref()
                    .or(self.settings.shore_block.as_deref());
            }
        }

        let strength = self.ocean_strength_at(horizontal);
        if strength > 0.0 {
            let sample = self.macro_sample_at(horizontal)?;
            let target_floor =
                self.sea_level - OCEAN_MINIMUM_DEPTH - OCEAN_EXTRA_DEPTH * strength;
            let floor = lerp(sample.elevation, target_floor, strength);

            if position.y >= floor - BED_MATERIAL_DEPTH
                && position.y <= floor + BED_MATERIAL_DEPTH
            {
                if strength <= SHORE_STRENGTH {
                    return self
                        .settings
                        .shore_block
                        .as_deref()
                        .or(self.settings.ocean_bed_block.as_deref());
                }

                return self
                    .settings
                    .ocean_bed_block
                    .as_deref()
                    .or(self.settings.shore_block.as_deref());
            }
        }

        None
    }

    pub fn ocean_strength_at(&self, position: Vec2) -> f32 {
        self.macro_sample_at(position)
            .map_or(0.0, |sample| ocean_strength(sample.continentalness))
    }

    fn material_for_water_body(&self, kind: WaterBodyKind, strength: f32) -> Option<&str> {
        let bed = match kind {
            WaterBodyKind::Lake => self.settings.lake_bed_block.as_deref(),
            WaterBodyKind::Ocean => self.settings.ocean_bed_block.as_deref(),
        };

        if strength <= SHORE_STRENGTH {
            self.settings.shore_block.as_deref().or(bed)
        } else {
            bed.or(self.settings.shore_block.as_deref())
        }
    }

    fn ocean_density_delta(&self, position: Vec2) -> f32 {
        let Some(sample) = self.macro_sample_at(position) else {
            return 0.0;
        };
        let strength = self.ocean_strength_at(position);

        if strength <= 0.0 {
            return 0.0;
        }

        let target_floor = self.sea_level - OCEAN_MINIMUM_DEPTH - OCEAN_EXTRA_DEPTH * strength;
        (target_floor - sample.elevation).min(0.0) * strength
    }

    fn macro_sample_at(&self, position: Vec2) -> Option<HydrologyMacroSample> {
        if self.macro_samples.len() != MACRO_SAMPLE_GRID * MACRO_SAMPLE_GRID {
            return None;
        }

        let origin = self.coord.as_vec2() * HYDROLOGY_REGION_SIZE;
        let step = HYDROLOGY_REGION_SIZE / (MACRO_SAMPLE_GRID - 1) as f32;
        let local = (position - origin) / step;
        let x = local.x.clamp(0.0, (MACRO_SAMPLE_GRID - 1) as f32);
        let z = local.y.clamp(0.0, (MACRO_SAMPLE_GRID - 1) as f32);
        let x0 = x.floor() as usize;
        let z0 = z.floor() as usize;
        let x1 = (x0 + 1).min(MACRO_SAMPLE_GRID - 1);
        let z1 = (z0 + 1).min(MACRO_SAMPLE_GRID - 1);
        let tx = smoothstep(x - x0 as f32);
        let tz = smoothstep(z - z0 as f32);
        let top = interpolate_macro(
            self.macro_samples[macro_index(x0, z0)],
            self.macro_samples[macro_index(x1, z0)],
            tx,
        );
        let bottom = interpolate_macro(
            self.macro_samples[macro_index(x0, z1)],
            self.macro_samples[macro_index(x1, z1)],
            tx,
        );

        Some(interpolate_macro(top, bottom, tz))
    }
}

fn macro_index(x: usize, z: usize) -> usize {
    x + z * MACRO_SAMPLE_GRID
}

fn interpolate_macro(
    from: HydrologyMacroSample,
    to: HydrologyMacroSample,
    amount: f32,
) -> HydrologyMacroSample {
    HydrologyMacroSample {
        elevation: lerp(from.elevation, to.elevation, amount),
        continentalness: lerp(from.continentalness, to.continentalness, amount),
    }
}
