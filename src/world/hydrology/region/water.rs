use bevy::prelude::*;

use super::HydrologyRegion;
use crate::world::hydrology::{
    constants::{RIVER_WATER_BODY_APPROACH_MARGIN, SHORE_STRENGTH},
    math::river_channel_profile,
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
    // unsupported high lake candidate before selecting the highest water level,
    // so it cannot hide a lower, supported river at a junction.
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
        let river = self.river_water_with_margin(position, 0.0, surface_height)?;
        let water_body_opening = self
            .water_bodies
            .iter()
            .filter_map(|body| {
                if !bed_has_support(
                    surface_height,
                    body.water_level - body.carve_depth,
                ) {
                    return None;
                }
                let strength = body.approach_strength_with_margin(
                    position,
                    RIVER_WATER_BODY_APPROACH_MARGIN,
                );
                (strength > 0.0).then_some(strength)
            })
            .fold(0.0_f32, f32::max);
        let strength = river.strength * (1.0 - water_body_opening.clamp(0.0, 1.0));
        (strength > f32::EPSILON).then_some(HydrologyRiverSurfaceSample {
            water_level: river.water_level,
            strength,
        })
    }

    fn river_water_with_margin(
        &self,
        position: Vec2,
        margin: f32,
        surface_height: Option<f32>,
    ) -> Option<HydrologyWaterSample<'_>> {
        let river = self.river_graph.sample_horizontal_filtered(
            position,
            margin,
            1.0,
            |river| {
                river.strength > SHORE_STRENGTH
                    && bed_has_support(
                        surface_height,
                        river.height
                            - self.river_carve_depth
                                * river_channel_profile(river.normalized_distance),
                    )
            },
        )?;
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

        if let Some(river) = self.river_water_with_margin(position, margin, surface_height) {
            choose_water(&mut selected, river, surface_height);
        }

        selected
    }
}

fn choose_water<'a>(
    selected: &mut Option<HydrologyWaterSample<'a>>,
    candidate: HydrologyWaterSample<'a>,
    surface_height: Option<f32>,
) {
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
    use crate::world::feature_graph::FeatureGraph;

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

    fn region(graph: FeatureGraph) -> HydrologyRegion {
        HydrologyRegion {
            coord: IVec2::ZERO,
            river_graph: graph,
            river_carve_depth: 7.0,
            water_bodies: Vec::new(),
            settings: Default::default(),
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
    fn overlapping_river_edges_choose_supported_physical_water() {
        for high_first in [true, false] {
            let mut graph = FeatureGraph::default();
            let high_from = graph.add_node(Vec3::new(0.0, 100.0, 0.0));
            let high_to = graph.add_node(Vec3::new(20.0, 100.0, 0.0));
            let low_from = graph.add_node(Vec3::new(0.0, 86.0, 3.0));
            let low_to = graph.add_node(Vec3::new(20.0, 86.0, 3.0));
            if high_first {
                graph.add_edge(high_from, high_to, 10.0, 10.0);
                graph.add_edge(low_from, low_to, 10.0, 10.0);
            } else {
                graph.add_edge(low_from, low_to, 10.0, 10.0);
                graph.add_edge(high_from, high_to, 10.0, 10.0);
            }

            let region = region(graph);
            let position = Vec2::new(10.0, 0.0);
            let original_surface = 85.0;
            assert_eq!(region.water_at(position).unwrap().water_level, 100.0);
            assert_eq!(region.river_surface_at(position).unwrap().water_level, 100.0);

            let supported = region.supported_water_at(position, original_surface).unwrap();
            assert_eq!(supported.kind, HydrologyWaterKind::River);
            assert_eq!(supported.water_level, 86.0);
            assert!(supported.bed_level <= original_surface + 0.5);
        }
    }
}
