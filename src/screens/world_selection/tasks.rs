use std::{
    io,
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};

use bevy::log::{info, warn};

use crate::{
    content::{
        block::BlockRegistry, creature::CreatureRegistry,
        day_night_cycle::DayNightCycleRegistry, dimension::DimensionRegistry,
        fluid::FluidRegistry, layer::LayerRegistry, tool::ToolRegistry,
    },
    voxel::world::VoxelWorld,
    world::save_catalog::{
        SaveRegistries, WorldDirectoryLock, WorldSnapshot, WorldSummary, list_verified_worlds,
        load_world,
    },
};

type WorldScanResult = Arc<Mutex<Option<io::Result<Vec<WorldSummary>>>>>;
type WorldLoadResult = Arc<Mutex<WorldLoadSlot>>;
type LoadedWorld = (WorldSnapshot, VoxelWorld, WorldDirectoryLock);

#[derive(Default)]
struct WorldLoadSlot {
    abandoned: bool,
    complete: bool,
    result: Option<io::Result<LoadedWorld>>,
}

pub(super) struct PendingWorldScan {
    result: WorldScanResult,
}

impl PendingWorldScan {
    pub(super) fn start(registries: SaveRegistries<'_>) -> io::Result<Self> {
        let copy_started = Instant::now();
        let owned = registries.owned_for_pruning();
        info!(
            "Saved-world catalog definitions copied on main thread: {:?}",
            copy_started.elapsed()
        );

        let result: WorldScanResult = Arc::new(Mutex::new(None));
        let worker_result = Arc::clone(&result);
        thread::Builder::new()
            .name("asteria-world-scan".to_owned())
            .spawn(move || {
                let scan_started = Instant::now();
                let verified = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    list_verified_worlds(&owned)
                }))
                .unwrap_or_else(|_| {
                    Err(io::Error::other("saved-world verification worker panicked"))
                });
                info!(
                    "Saved-world catalog verification: duration={:?}, successful={}, worlds={}",
                    scan_started.elapsed(),
                    verified.is_ok(),
                    verified.as_ref().map_or(0, Vec::len)
                );
                if let Ok(mut slot) = worker_result.lock() {
                    *slot = Some(verified);
                }
            })?;

        Ok(Self { result })
    }

    pub(super) fn poll(&self) -> Option<io::Result<Vec<WorldSummary>>> {
        self.result
            .try_lock()
            .ok()
            .and_then(|mut slot| slot.take())
    }
}

struct OwnedLoadContent {
    blocks: BlockRegistry,
    layers: LayerRegistry,
    fluids: FluidRegistry,
    tools: ToolRegistry,
    creatures: CreatureRegistry,
    dimensions: DimensionRegistry,
    cycles: DayNightCycleRegistry,
}

impl OwnedLoadContent {
    fn capture(registries: SaveRegistries<'_>) -> Self {
        let mut tools = ToolRegistry::default();
        for definition in registries.tools.iter() {
            tools.insert(definition.clone());
        }
        let mut creatures = CreatureRegistry::default();
        for definition in registries.creatures.iter() {
            creatures.insert(definition.clone());
        }
        let mut dimensions = DimensionRegistry::default();
        let mut cycles = DayNightCycleRegistry::default();
        for definition in registries.dimensions.iter() {
            if let Some(cycle) = registries.cycles.get(&definition.day_night_cycle) {
                cycles.insert(cycle.clone());
            }
            dimensions.insert(definition.clone());
        }

        Self {
            blocks: registries.blocks.clone(),
            layers: registries.layers.clone(),
            fluids: registries.fluids.clone(),
            tools,
            creatures,
            dimensions,
            cycles,
        }
    }

    fn registries(&self) -> SaveRegistries<'_> {
        SaveRegistries {
            blocks: &self.blocks,
            layers: &self.layers,
            fluids: &self.fluids,
            tools: &self.tools,
            creatures: &self.creatures,
            dimensions: &self.dimensions,
            cycles: &self.cycles,
        }
    }
}

pub(super) struct WorldLoadCompletion {
    pub(super) abandoned: bool,
    pub(super) result: Option<io::Result<LoadedWorld>>,
}

pub(super) struct PendingWorldLoad {
    id: String,
    result: WorldLoadResult,
}

impl PendingWorldLoad {
    pub(super) fn start(id: String, registries: SaveRegistries<'_>) -> io::Result<Self> {
        let copy_started = Instant::now();
        let owned = OwnedLoadContent::capture(registries);
        info!(
            "World {id} load definitions copied on main thread: {:?}",
            copy_started.elapsed()
        );

        let result: WorldLoadResult = Arc::new(Mutex::new(WorldLoadSlot::default()));
        let worker_result = Arc::clone(&result);
        let worker_id = id.clone();
        thread::Builder::new()
            .name("asteria-world-load".to_owned())
            .spawn(move || {
                let load_started = Instant::now();
                let loaded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    load_world(&worker_id, owned.registries())
                }))
                .unwrap_or_else(|_| Err(io::Error::other("saved-world loading worker panicked")));
                info!(
                    "World {worker_id} load worker: duration={:?}, successful={}",
                    load_started.elapsed(),
                    loaded.is_ok()
                );
                let mut loaded = Some(loaded);
                {
                    let mut slot = worker_result
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    slot.complete = true;
                    if !slot.abandoned {
                        slot.result = loaded.take();
                    }
                }
                drop(loaded);
            })?;

        Ok(Self { id, result })
    }

    pub(super) fn id(&self) -> &str {
        &self.id
    }

    pub(super) fn poll(&self) -> Option<WorldLoadCompletion> {
        let mut slot = self.result.try_lock().ok()?;
        if !slot.complete {
            return None;
        }
        Some(WorldLoadCompletion {
            abandoned: slot.abandoned,
            result: slot.result.take(),
        })
    }

    pub(super) fn abandon(&self) {
        // Cancellation and worker publication are serialized by THIS small
        // mutex. An already-completed large world is moved to a disposer rather
        // than being dropped on the input frame; a late result is discarded by
        // the original worker itself. Neither case leaves a world in the menu.
        let stale = {
            let mut slot = self
                .result
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            slot.abandoned = true;
            slot.result.take()
        };
        if let Some(stale) = stale
            && let Err(error) = thread::Builder::new()
                .name("asteria-discard-world".to_owned())
                .spawn(move || drop(stale))
        {
            warn!("Could not start abandoned-world cleanup worker: {error}");
        }
    }
}
