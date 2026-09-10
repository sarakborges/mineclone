use bevy::prelude::*;

use super::HydrologyRegion;
use crate::world::hydrology::types::HydrologyWaterSample;

impl HydrologyRegion {
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

        if let Some(river) = self
            .river_graph
            .sample_horizontal(position)
            .filter(|river| river.strength > 0.0)
        {
            choose_higher_water(
                &mut selected,
                HydrologyWaterSample {
                    fluid_id: self.settings.water_fluid.as_str(),
                    water_level: river.height,
                },
            );
        }

        if self.ocean_strength_at(position) > 0.0 {
            choose_higher_water(
                &mut selected,
                HydrologyWaterSample {
                    fluid_id: self.settings.water_fluid.as_str(),
                    water_level: self.sea_level,
                },
            );
        }

        selected
    }
}

fn choose_higher_water<'a>(
    selected: &mut Option<HydrologyWaterSample<'a>>,
    candidate: HydrologyWaterSample<'a>,
) {
    if selected
        .as_ref()
        .is_none_or(|current| candidate.water_level > current.water_level)
    {
        *selected = Some(candidate);
    }
}
