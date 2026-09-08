use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum DayNightPhase {
    Dawn,
    Day,
    Dusk,
    Night,
}

#[derive(Clone, Deserialize)]
pub struct DayNightPhases<T> {
    pub dawn: T,
    pub day: T,
    pub dusk: T,
    pub night: T,
}

impl<T> DayNightPhases<T> {
    pub fn get(&self, phase: DayNightPhase) -> &T {
        match phase {
            DayNightPhase::Dawn => &self.dawn,
            DayNightPhase::Day => &self.day,
            DayNightPhase::Dusk => &self.dusk,
            DayNightPhase::Night => &self.night,
        }
    }

    pub fn next(phase: DayNightPhase) -> DayNightPhase {
        match phase {
            DayNightPhase::Dawn => DayNightPhase::Day,
            DayNightPhase::Day => DayNightPhase::Dusk,
            DayNightPhase::Dusk => DayNightPhase::Night,
            DayNightPhase::Night => DayNightPhase::Dawn,
        }
    }
}
