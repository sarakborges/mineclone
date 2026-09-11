mod hash;
mod lake;
mod river;

use bevy::prelude::*;

use crate::world::feature_graph::FeatureGraph;

use self::{
    hash::compare_position,
    lake::{UndergroundLake, lake_for_anchor},
    river::{
        UndergroundWaterfall, add_river_segment, connection_carries_water, river_sample_at,
    },
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct UndergroundWaterSample {
    pub water_level: f32,
    pub bed_level: f32,
}

#[derive(Clone, Debug, Default)]
pub(super) struct UndergroundWaterRegion {
    river_graph: FeatureGraph,
    lakes: Vec<UndergroundLake>,
    waterfalls: Vec<UndergroundWaterfall>,
}

impl UndergroundWaterRegion {
    pub(super) fn from_anchors(anchors: &[Vec3], seed: u64) -> Self {
        let mut anchors = anchors.to_vec();
        anchors.sort_by(compare_position);
        anchors.dedup_by(|left, right| *left == *right);

        let lakes = anchors
            .into_iter()
            .filter_map(|anchor| lake_for_anchor(anchor, seed))
            .collect();

        Self {
            river_graph: FeatureGraph::default(),
            lakes,
            waterfalls: Vec::new(),
        }
    }

    pub(super) fn connection_carries_water(from: Vec3, to: Vec3, seed: u64) -> bool {
        connection_carries_water(from, to, seed)
    }

    pub(super) fn add_river_segment(
        &mut self,
        from: Vec3,
        to: Vec3,
        start_cave_radius: f32,
        end_cave_radius: f32,
    ) {
        add_river_segment(
            &mut self.river_graph,
            &mut self.waterfalls,
            from,
            to,
            start_cave_radius,
            end_cave_radius,
        );
    }

    pub(crate) fn water_at(&self, position: Vec3) -> Option<UndergroundWaterSample> {
        for waterfall in &self.waterfalls {
            if waterfall.contains(position) {
                return Some(UndergroundWaterSample {
                    water_level: position.y + 0.5,
                    bed_level: position.y - 0.5,
                });
            }
        }

        let horizontal = Vec2::new(position.x, position.z);
        let mut selected = None;

        for lake in &self.lakes {
            let strength = lake.horizontal_strength(horizontal);
            if strength <= 0.0 {
                continue;
            }

            consider_water(
                &mut selected,
                UndergroundWaterSample {
                    water_level: lake.water_level,
                    bed_level: lake.water_level - lake.depth * strength,
                },
                position.y,
            );
        }

        if let Some(river) = river_sample_at(&self.river_graph, horizontal) {
            consider_water(&mut selected, river, position.y);
        }

        selected
    }
}

fn consider_water(
    selected: &mut Option<UndergroundWaterSample>,
    candidate: UndergroundWaterSample,
    y: f32,
) {
    let cell_bottom = y - 0.5;
    let cell_top = y + 0.5;
    if cell_top <= candidate.bed_level || cell_bottom >= candidate.water_level {
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
    use super::lake::anchor_has_lake;

    #[test]
    fn underground_water_is_deterministic() {
        let anchors = [Vec3::new(12.5, 30.5, 8.5), Vec3::new(80.5, 18.5, 20.5)];
        let first = UndergroundWaterRegion::from_anchors(&anchors, 42);
        let second = UndergroundWaterRegion::from_anchors(&anchors, 42);

        assert_eq!(first.lakes.len(), second.lakes.len());
        for position in [
            Vec3::new(12.5, 28.5, 8.5),
            Vec3::new(80.5, 16.5, 20.5),
        ] {
            assert_eq!(
                first.water_at(position).is_some(),
                second.water_at(position).is_some(),
            );
        }
    }

    #[test]
    fn river_water_occupies_only_the_lower_part_of_a_connector() {
        let mut water = UndergroundWaterRegion::default();
        water.add_river_segment(
            Vec3::new(0.0, 20.0, 0.0),
            Vec3::new(20.0, 20.0, 0.0),
            6.0,
            6.0,
        );

        assert!(water.water_at(Vec3::new(10.0, 17.5, 0.0)).is_some());
        assert!(water.water_at(Vec3::new(10.0, 21.0, 0.0)).is_none());
    }

    #[test]
    fn steep_underground_river_segment_forms_a_waterfall() {
        let mut water = UndergroundWaterRegion::default();
        water.add_river_segment(
            Vec3::new(0.0, 30.0, 0.0),
            Vec3::new(5.0, 18.0, 0.0),
            6.0,
            6.0,
        );

        assert!(!water.waterfalls.is_empty());
        let midpoint = water.waterfalls[0].from.lerp(water.waterfalls[0].to, 0.5);
        assert!(water.water_at(midpoint).is_some());
    }

    #[test]
    fn lake_anchor_always_drains_into_a_lower_connection() {
        let mut chosen = None;
        for x in 0..128 {
            let anchor = Vec3::new(x as f32 + 0.5, 40.5, 0.5);
            if anchor_has_lake(anchor, 42) {
                chosen = Some(anchor);
                break;
            }
        }
        let high = chosen.expect("test should find a deterministic lake anchor");
        let low = high + Vec3::new(20.0, -12.0, 0.0);

        assert!(UndergroundWaterRegion::connection_carries_water(high, low, 42));
    }
}
