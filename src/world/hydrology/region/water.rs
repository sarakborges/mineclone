use bevy::prelude::*;

use super::HydrologyRegion;
use crate::world::hydrology::{
    constants::{OCEAN_EXTRA_DEPTH, OCEAN_MINIMUM_DEPTH, SHORE_STRENGTH},
    math::{lerp, smoothstep},
    types::{HydrologyRiverSurfaceSample, HydrologyWaterKind, HydrologyWaterSample},
};

impl HydrologyRegion {
    pub(crate) fn water_at(&self, position: Vec2) -> Option<HydrologyWaterSample<'_>> {
        self.water_with_margin(position, 0.0)
    }

    pub(crate) fn water_near(
        &self,
        position: Vec2,
        radius: f32,
    ) -> Option<HydrologyWaterSample<'_>> {
        self.water_with_margin(position, radius.max(0.0))
    }

    pub(crate) fn river_water_at(&self, position: Vec2) -> Option<HydrologyWaterSample<'_>> {
        self.river_water_with_margin(position, 0.0)
    }

    pub(crate) fn river_surface_at(
        &self,
        position: Vec2,
    ) -> Option<HydrologyRiverSurfaceSample> {
        let river = self.river_graph.sample_horizontal(position)?;

        Some(HydrologyRiverSurfaceSample {
            water_level: river.height,
            strength: river.strength,
        })
    }

    fn river_water_with_margin(
        &self,
        position: Vec2,
        margin: f32,
    ) -> Option<HydrologyWaterSample<'_>> {
        let river = self
            .river_graph
            .sample_horizontal_with_margin(position, margin)
            .filter(|river| river.strength > SHORE_STRENGTH)?;
        let profile = smoothstep(river.strength);

        Some(HydrologyWaterSample {
            fluid_id: self.settings.water_fluid.as_str(),
            water_level: river.height,
            bed_level: river.height - self.river_carve_depth * profile,
            strength: river.strength,
            kind: HydrologyWaterKind::River,
        })
    }

    fn water_with_margin(
        &self,
        position: Vec2,
        margin: f32,
    ) -> Option<HydrologyWaterSample<'_>> {
        let mut selected = None;

        for body in &self.water_bodies {
            let strength = body.horizontal_strength_with_margin(position, margin);
            if strength <= 0.0 {
                continue;
            }

            choose_water(
                &mut selected,
                HydrologyWaterSample {
                    fluid_id: body.fluid_id.as_str(),
                    water_level: body.water_level,
                    bed_level: body.water_level - body.carve_depth * strength,
                    strength,
                    kind: HydrologyWaterKind::Lake,
                },
            );
        }

        if let Some(river) = self.river_water_with_margin(position, margin) {
            choose_water(&mut selected, river);
        }

        let ocean_strength = self.ocean_strength_at(position);
        if ocean_strength > 0.0 {
            let target_floor =
                self.sea_level - OCEAN_MINIMUM_DEPTH - OCEAN_EXTRA_DEPTH * ocean_strength;
            let bed_level = self
                .macro_sample_at(position)
                .map_or(target_floor, |sample| {
                    lerp(sample.elevation, target_floor, ocean_strength)
                });

            choose_water(
                &mut selected,
                HydrologyWaterSample {
                    fluid_id: self.settings.water_fluid.as_str(),
                    water_level: self.sea_level,
                    bed_level,
                    strength: ocean_strength,
                    kind: HydrologyWaterKind::Ocean,
                },
            );
        }

        selected
    }
}

fn choose_water<'a>(
    selected: &mut Option<HydrologyWaterSample<'a>>,
    candidate: HydrologyWaterSample<'a>,
) {
    let should_replace = selected.as_ref().is_none_or(|current| {
        candidate.water_level > current.water_level
            || ((candidate.water_level - current.water_level).abs() <= f32::EPSILON
                && candidate.bed_level < current.bed_level)
    });

    if should_replace {
        *selected = Some(candidate);
    }
}
