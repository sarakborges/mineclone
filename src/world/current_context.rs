use bevy::{ecs::system::SystemParam, prelude::*};

use crate::content::{
    day_night_cycle::{DayNightCycleDefinition, DayNightCycleRegistry, DayNightSample},
    dimension::{DimensionDefinition, DimensionRegistry},
    sky::{SkyDefinition, SkyRegistry},
};

use super::{day_night::DayNightClock, dimension::CurrentDimension};

#[derive(SystemParam)]
pub(crate) struct CurrentDimensionContext<'w> {
    current: Res<'w, CurrentDimension>,
    dimensions: Res<'w, DimensionRegistry>,
}

impl CurrentDimensionContext<'_> {
    pub(crate) fn id(&self) -> &str {
        &self.current.id
    }

    pub(crate) fn definition(&self) -> Option<&DimensionDefinition> {
        self.dimensions.get(&self.current.id)
    }

    pub(crate) fn inputs_changed(&self) -> bool {
        self.current.is_changed() || self.dimensions.is_changed()
    }
}

#[derive(SystemParam)]
pub(crate) struct DayNightContext<'w> {
    dimension: CurrentDimensionContext<'w>,
    cycles: Res<'w, DayNightCycleRegistry>,
    clock: Res<'w, DayNightClock>,
}

impl DayNightContext<'_> {
    pub(crate) fn dimension(&self) -> Option<&DimensionDefinition> {
        self.dimension.definition()
    }

    pub(crate) fn cycle(&self) -> Option<&DayNightCycleDefinition> {
        let dimension = self.dimension.definition()?;
        self.cycles.get(&dimension.day_night_cycle)
    }

    pub(crate) fn clock(&self) -> &DayNightClock {
        &self.clock
    }

    pub(crate) fn sample(&self) -> Option<DayNightSample> {
        self.cycle()
            .map(|cycle| cycle.sample(self.clock.normalized_time))
    }

    pub(crate) fn world_time(&self) -> Option<(u32, u32)> {
        self.cycle()
            .map(|cycle| cycle.world_time(self.clock.normalized_time))
    }

    pub(crate) fn inputs_changed(&self) -> bool {
        self.dimension.inputs_changed() || self.cycles.is_changed() || self.clock.is_changed()
    }
}

#[derive(SystemParam)]
pub(crate) struct SkyContext<'w> {
    dimension: CurrentDimensionContext<'w>,
    skies: Res<'w, SkyRegistry>,
}

impl SkyContext<'_> {
    pub(crate) fn sky(&self) -> Option<&SkyDefinition> {
        let dimension = self.dimension.definition()?;
        self.skies.get(&dimension.sky)
    }
}

#[derive(SystemParam)]
pub(crate) struct SkyDayNightContext<'w> {
    day_night: DayNightContext<'w>,
    skies: Res<'w, SkyRegistry>,
}

impl SkyDayNightContext<'_> {
    pub(crate) fn cycle(&self) -> Option<&DayNightCycleDefinition> {
        self.day_night.cycle()
    }

    pub(crate) fn clock(&self) -> &DayNightClock {
        self.day_night.clock()
    }

    pub(crate) fn sample(&self) -> Option<DayNightSample> {
        self.day_night.sample()
    }

    pub(crate) fn sky(&self) -> Option<&SkyDefinition> {
        let dimension = self.day_night.dimension()?;
        self.skies.get(&dimension.sky)
    }

    pub(crate) fn inputs_changed(&self) -> bool {
        self.day_night.inputs_changed() || self.skies.is_changed()
    }
}
