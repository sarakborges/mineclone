use bevy::prelude::*;

use super::HydrologyRegion;
use crate::world::hydrology::{
    constants::{OCEAN_EXTRA_DEPTH, OCEAN_MINIMUM_DEPTH, SHORE_STRENGTH},
    math::{lerp, river_channel_profile},
    types::{HydrologyRiverSurfaceSample, HydrologyWaterKind, HydrologyWaterSample},
};

// All physical hydrology consumers must agree on whether the unmodified
// terrain can support a natural source. A missing original surface means the
// caller only wants an unfiltered hydrological sample.
pub(super) fn bed_has_support(surface_height: Option<f32>, bed_level: f32) -> bool {
    surface_height.is_none_or(|surface| surface + 0.5 >= bed_level)
}

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
        self.river_surface_with_support(position, None)
    }

    // Carving the headroom above a river is part of generating its physical
    // channel. Do not excavate that roof if its bed has no supporting terrain.
    pub(crate) fn supported_river_surface_at(
        &self,
        position: Vec2,
        surface_height: f32,
    ) -> Option<HydrologyRiverSurfaceSample> {
        self.river_surface_with_support(position, Some(surface_height))
    }

    fn river_surface_with_support(
        &self,
        position: Vec2,
        surface_height: Option<f32>,
    ) -> Option<HydrologyRiverSurfaceSample> {
        let river = self.river_water_with_margin(position, 0.0)?;
        bed_has_support(surface_height, river.bed_level).then_some(HydrologyRiverSurfaceSample {
            water_level: river.water_level,
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
            // Density uses the original terrain column as the starting height
            // whenever it is known. Using the interpolated macro elevation
            // here instead could classify that same carved column as dry and
            // leave an ocean-sized hole with no physical water source.
            let base = surface_height
                .or_else(|| self.macro_sample_at(position).map(|sample| sample.elevation))
                .unwrap_or(target_floor);
            let bed_level = lerp(base, target_floor, ocean_strength);

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
    // Filter each candidate rather than filtering the *winner*, which could
    // leave a supported river hidden under an unsupported higher lake.
    if !bed_has_support(surface_height, candidate.bed_level) {
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
    use crate::world::{
        feature_graph::FeatureGraph,
        hydrology::{constants::MACRO_SAMPLE_GRID, types::HydrologyMacroSample},
    };

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

    #[test]
    fn support_rule_accepts_exact_bed_boundary() {
        assert!(bed_has_support(Some(79.5), 80.0));
        assert!(!bed_has_support(Some(79.0), 80.0));
        assert!(bed_has_support(None, 100.0));
    }

    #[test]
    fn physical_ocean_uses_the_same_original_column_floor_as_density() {
        let region = HydrologyRegion {
            coord: IVec2::ZERO,
            river_graph: FeatureGraph::default(),
            river_carve_depth: 7.0,
            water_bodies: Vec::new(),
            sea_level: 90.0,
            settings: Default::default(),
            ocean_weight: 1.0,
            macro_samples: vec![
                HydrologyMacroSample {
                    elevation: 120.0,
                    continentalness: 0.39,
                };
                MACRO_SAMPLE_GRID * MACRO_SAMPLE_GRID
            ],
        };
        let position = Vec2::splat(64.0);
        let original_surface = 80.0;

        // The macro-only lookup is dry, but the actual terrain column is
        // submerged after carving. Both physical water and density must use
        // that column's original height when generating it.
        assert!(region.water_at(position).is_none());
        let water = region.supported_water_at(position, original_surface).unwrap();
        assert_eq!(water.kind, HydrologyWaterKind::Ocean);
        let density_delta =
            region.density_deltas_for_column::<1>(position, 40.0, original_surface)[0];
        assert!((water.bed_level - (original_surface + density_delta)).abs() < 0.001);
        assert!(water.bed_level < region.sea_level - 0.5);
    }
}
