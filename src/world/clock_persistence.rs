use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::day_night_cycle::DayNightCycleRegistry;

use super::{
    current_context::CurrentDimensionContext,
    day_night::DayNightClock,
    save_session::WorldSession,
    world_names::{WORLDS_DIRECTORY, validate_world_name},
};

const CLOCK_PREFIX: &str = "clock-";
const CLOCK_SUFFIX: &str = ".json";
const CLOCK_SAVE_SECONDS: f32 = 60.0;

#[derive(Debug, Deserialize, Serialize)]
struct SavedClock {
    id: String,
    day: u64,
    tick_in_day: u64,
}

#[derive(Resource)]
pub(crate) struct ClockPersistence {
    timer: Timer,
}

impl Default for ClockPersistence {
    fn default() -> Self {
        Self { timer: Timer::from_seconds(CLOCK_SAVE_SECONDS, TimerMode::Repeating) }
    }
}

pub(crate) fn reset_clock_persistence(mut persistence: ResMut<ClockPersistence>) {
    persistence.timer.reset();
}

pub(crate) fn persist_clock_periodically(
    time: Res<Time<Real>>,
    mut persistence: ResMut<ClockPersistence>,
    session: Res<WorldSession>,
    clock: Res<DayNightClock>,
) {
    if !persistence.timer.tick(time.delta()).just_finished() {
        return;
    }
    let Some(id) = session.id.as_deref() else { return };
    if let Err(error) = save_clock(id, clock.day, clock.tick_in_day()) {
        warn!("World clock checkpoint failed; full world saves remain valid: {error}");
    }
}

/// Apply a lightweight clock checkpoint only when it is both valid for the
/// active cycle and later than the clock restored from the immutable snapshot.
/// A stale checkpoint can therefore never roll a newer full save backwards.
pub(crate) fn restore_persisted_clock(
    session: Res<WorldSession>,
    dimension: CurrentDimensionContext,
    cycles: Res<DayNightCycleRegistry>,
    mut clock: ResMut<DayNightClock>,
) {
    let Some(id) = session.id.as_deref() else { return };
    let saved = match load_clock(id) {
        Ok(Some(saved)) => saved,
        Ok(None) => return,
        Err(error) => {
            warn!("Ignoring unreadable world clock checkpoint: {error}");
            return;
        }
    };
    if saved <= (clock.day, clock.tick_in_day()) {
        return;
    }
    let definition = dimension
        .definition()
        .expect("loaded dimension must exist in content registry");
    let cycle = cycles
        .get(&definition.day_night_cycle)
        .expect("loaded day-night cycle must exist in content registry");
    if !clock.restore(saved.0, saved.1, cycle.day_duration_ticks) {
        warn!("Ignoring world clock checkpoint that is invalid for the active day-night cycle");
    }
}

fn save_clock(id: &str, day: u64, tick_in_day: u64) -> io::Result<()> {
    validate_world_name(id)?;
    if day == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "saved clock day must be positive"));
    }
    let directory = Path::new(WORLDS_DIRECTORY).join(id);
    if !fs::symlink_metadata(&directory)?.file_type().is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "world directory cannot be a symbolic link"));
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let filename = format!("{CLOCK_PREFIX}{stamp:039}{CLOCK_SUFFIX}");
    let temporary = directory.join(format!("{filename}.tmp"));
    let final_path = directory.join(&filename);
    let clock = SavedClock { id: id.to_owned(), day, tick_in_day };
    let mut file = OpenOptions::new().write(true).create_new(true).open(&temporary)?;
    let result = (|| {
        serde_json::to_writer(&mut file, &clock).map_err(io::Error::other)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, &final_path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
        return result;
    }

    // Clock checkpoints are immutable. Only prune older files after the new
    // checkpoint is durably published, so a crash can at worst leave extras.
    for entry in fs::read_dir(&directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() || entry.path() == final_path {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if parse_clock_stamp(name).is_some() {
            let _ = fs::remove_file(entry.path());
        }
    }
    Ok(())
}

fn load_clock(id: &str) -> io::Result<Option<(u64, u64)>> {
    validate_world_name(id)?;
    let directory = Path::new(WORLDS_DIRECTORY).join(id);
    if !fs::symlink_metadata(&directory)?.file_type().is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "world directory cannot be a symbolic link"));
    }
    let mut candidates = Vec::new();
    for entry in fs::read_dir(&directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if let Some(stamp) = parse_clock_stamp(name) {
            candidates.push((stamp, entry.path()));
        }
    }
    candidates.sort_unstable_by_key(|(stamp, _)| std::cmp::Reverse(*stamp));
    for (_, path) in candidates {
        let Ok(file) = fs::File::open(path) else { continue };
        let Ok(clock) = serde_json::from_reader::<_, SavedClock>(io::BufReader::new(file)) else {
            continue;
        };
        if clock.id == id && clock.day > 0 {
            return Ok(Some((clock.day, clock.tick_in_day)));
        }
    }
    Ok(None)
}

fn parse_clock_stamp(name: &str) -> Option<u128> {
    let digits = name.strip_prefix(CLOCK_PREFIX)?.strip_suffix(CLOCK_SUFFIX)?;
    (digits.len() == 39 && digits.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| digits.parse().ok())
        .flatten()
}
