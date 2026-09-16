use bevy::prelude::*;

use super::HydrologyRegion;
use crate::world::hydrology::{
    constants::{OCEAN_EXTRA_DEPTH, OCEAN_MINIMUM_DEPTH, SHORE_STRENGTH},
    math::{lerp, river_channel_profile},
    types::{HydrologyRiverSurfaceSample, HydrologyWaterKind, HydrologyWaterSample},
};

impl HydrologyRegion {
    pub(crate) fn water_at(&self, position: Vec2) -> Option<HydrologyWaterSample<'_>> {
        self.water_with_margin(position, 0.0, None)
    }

    // The physical fluid pass knows the original terrain height. Reject an
    // unsupported high lake/ocean candidate *before* selecting the highest
    // water level, so it cannot hide a lower, supported river at a junction.
    // Other hydrology consumers retain the original unfiltered water_at API.
    pub(crate) fn supported_water_at(
        &self,
        position: Vec2,
        surface_height: f32,
    ) -> Option<HydrologyWaterSample<'_>> {
        self.water_with_margin(position, 0.0, Some(surface_height))
    }

    pub(crate) fn water_near(
        &self,
        position: Vec2,
        radius: f32,
    ) -> Option<HydrologyWaterSample<'_>> {
        self.water_with_margin(position, radius.max(0.0), None)
    }

    pub(crate) fn river_surface_at(
        &self,
        position: Vec2,
    ) -> Option<HydrologyRiverSurfaceSample> {
        let river = self
            .river_graph
            .sample_horizontal(position)
            .filter(|river| river.strength > SHORE_STRENGTH)?;

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
        // The margin only discovers nearby channels: it must not inflate the
        // physical river bed, which shares its profile with density carving.
        let profile = river_channel_profile(river.normalized_distance);

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
        surface_height: Option<f32>,
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
                surface_height,
            );
        }

        if let Some(river) = self.river_water_with_margin(position, margin) {
            choose_water(&mut selected, river, surface_height);
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

            // The first continentalness threshold can still leave terrain above
            // sea level. Such a dry ocean sample must not supersede an actual
            // river mouth merely because the ocean's nominal water level is high.
            if bed_level < self.sea_level - 0.5 {
                choose_water(
                    &mut selected,
                    HydrologyWaterSample {
                        fluid_id: self.settings.water_fluid.as_str(),
                        water_level: self.sea_level,
                        bed_level,
                        strength: ocean_strength,
                        kind: HydrologyWaterKind::Ocean,
                    },
                    surface_height,
                );
            }
        }

        selected
    }
}

fn choose_water<'a>(
    selected: &mut Option<HydrologyWaterSample<'a>>,
    candidate: HydrologyWaterSample<'a>,
    surface_height: Option<f32>,
) {
    // The physical fluid pass must never pick a floating water body. Filter
    // each candidate rather than filtering the *winner*, which could leave
    // a supported river hidden under an unsupported higher lake.
    if surface_height.is_some_and(|surface| surface + 0.5 < candidate.bed_level) {
        return;
    }

    let should_replace = selected.as_ref().is_none_or(|current| {
        candidate.water_level > current.water_level
            || ((candidate.water_level - current.water_level).abs() <= f32::EPSILON
                && candidate.bed_level < current.bed_level)
    });

    if should_replace {
        *selected = Some(candidate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(
        kind: HydrologyWaterKind,
        water_level: f32,
        bed_level: f32,
    ) -> HydrologyWaterSample<'static> {
        HydrologyWaterSample {
            fluid_id: "asteria:test/water",
            water_level,
            bed_level,
            strength: 1.0,
            kind,
        }
    }

    #[test]
    fn unsupported_high_lake_cannot_hide_a_supported_lower_river() {
        let mut selected = None;
        choose_water(
            &mut selected,
            sample(HydrologyWaterKind::Lake, 100.0, 94.0),
            Some(85.0),
        );
        assert!(selected.is_none());

        choose_water(
            &mut selected,
            sample(HydrologyWaterKind::River, 83.0, 78.0),
            Some(85.0),
        );
        assert_eq!(selected.unwrap().kind, HydrologyWaterKind::River);
    }

    #[test]
    fn unfiltered_hydrology_still_prefers_highest_water_level() {
        let mut selected = None;
        choose_water(
            &mut selected,
            sample(HydrologyWaterKind::River, 83.0, 78.0),
            None,
        );
        choose_water(
            &mut selected,
            sample(HydrologyWaterKind::Lake, 100.0, 94.0),
            None,
        );
        assert_eq!(selected.unwrap().kind, HydrologyWaterKind::Lake);
    }
}
